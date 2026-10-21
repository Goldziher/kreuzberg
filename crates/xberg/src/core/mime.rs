//! MIME type detection and validation.
//!
//! This module provides utilities for detecting MIME types from file extensions
//! and validating them against supported types.
//!
//! Format information is centralized in the `FORMATS` registry. All extension-to-MIME
//! mappings and supported MIME type validation are derived from this single source of truth.

#[cfg(any(
    feature = "office",
    feature = "hwpx",
    feature = "iwork",
    feature = "archives",
    feature = "hwp",
    feature = "email"
))]
use crate::extractors::security::SecurityLimits;
use crate::{Result, XbergError};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::LazyLock;

/// A supported document format entry.
///
/// Represents a file extension and its corresponding MIME type that Xberg can process.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct SupportedFormat {
    /// File extension (without leading dot), e.g., "pdf", "docx"
    pub extension: String,
    /// MIME type string, e.g., "application/pdf"
    pub mime_type: String,
}

pub(crate) const OCTET_STREAM_MIME_TYPE: &str = "application/octet-stream";
pub(crate) const HTML_MIME_TYPE: &str = "text/html";
const MIME_SNIFF_LENGTH: usize = 4096;
const SQLITE_APPLICATION_ID_OFFSET: usize = 68;
const SQLITE_APPLICATION_ID_LENGTH: usize = 4;
const GEOPACKAGE_APPLICATION_ID: &[u8; SQLITE_APPLICATION_ID_LENGTH] = b"GPKG";
const GEOPACKAGE_LEGACY_APPLICATION_ID: &[u8; SQLITE_APPLICATION_ID_LENGTH] = b"GP10";
const J2C_CODESTREAM_MAGIC: &[u8; 4] = b"\xFF\x4F\xFF\x51";
/// MS-CFB (compound binary file) signature, shared by legacy .doc/.xls/.ppt.
#[cfg(any(feature = "office", feature = "hwp", feature = "email"))]
const OLE2_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

#[derive(Clone, Copy, PartialEq, Eq)]
enum PackageInspection {
    HeaderOnly,
    FullArchive,
}

/// Element names that identify a markup fragment as HTML rather than generic XML.
///
/// Deliberately conservative: every entry is an element that exists in HTML and is not a
/// plausible root for the XML vocabularies this crate also extracts (DocBook, JATS, FB2,
/// OPML, RSS). Ambiguous names shared with those vocabularies — `title`, `table`, `para`,
/// `section`, `article`, `link`, `code` — are omitted on purpose, so a borderline document
/// keeps its current XML routing instead of being silently rerouted.
const HTML_FRAGMENT_ELEMENTS: &[&str] = &[
    "a",
    "b",
    "blockquote",
    "body",
    "br",
    "button",
    "div",
    "em",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hr",
    "i",
    "iframe",
    "img",
    "input",
    "label",
    "li",
    "main",
    "meta",
    "nav",
    "ol",
    "option",
    "p",
    "pre",
    "script",
    "select",
    "span",
    "strong",
    "style",
    "table",
    "tbody",
    "td",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "tr",
    "ul",
];

/// Return `true` when `trimmed` opens as HTML rather than as generic XML.
///
/// Recognises the two document preambles case-insensitively (`<!doctype html>` is at least
/// as common in the wild as the uppercase spelling) and, for fragments that carry no
/// preamble at all, the name of the first element.
fn looks_like_html(trimmed: &str) -> bool {
    let lowered_prefix: String = trimmed.chars().take(16).flat_map(char::to_lowercase).collect();
    if lowered_prefix.starts_with("<!doctype html") || lowered_prefix.starts_with("<html") {
        return true;
    }

    let Some(after_bracket) = trimmed.strip_prefix('<') else {
        return false;
    };
    let name_length = after_bracket
        .find(|character: char| !character.is_ascii_alphanumeric())
        .unwrap_or(after_bracket.len());
    let (name, rest) = after_bracket.split_at(name_length);
    // Require the tag to actually close here, so `<tr:foo>` (a namespace prefix that happens
    // to collide with an HTML name) stays XML.
    if !matches!(
        rest.chars().next(),
        Some('>') | Some(' ') | Some('/') | Some('\t') | Some('\n') | Some('\r')
    ) {
        return false;
    }

    let name = name.to_ascii_lowercase();
    HTML_FRAGMENT_ELEMENTS.contains(&name.as_str())
}
pub(crate) const DOCBOOK_MIME_TYPE: &str = "application/docbook+xml";
pub(crate) const JATS_MIME_TYPE: &str = "application/x-jats+xml";

/// Return the XML vocabulary that `trimmed` declares, if it declares one.
///
/// Real DocBook and JATS documents use the `.xml` extension, so the extension
/// map alone routes them to the generic XML extractor and their structure and
/// equations are lost.
///
/// The test is structural, not a search of the text. A public identifier counts
/// only inside the DOCTYPE declaration, and a namespace counts only when the
/// root element declares it. A stylesheet, a schema or a catalog that merely
/// names DocBook keeps its generic XML routing.
fn xml_vocabulary(trimmed: &str) -> Option<&'static str> {
    let doctype = declaration_of(trimmed, "<!DOCTYPE");
    if let Some(doctype) = doctype {
        if doctype.contains("//OASIS//DTD DocBook") {
            return Some(DOCBOOK_MIME_TYPE);
        }
        if doctype.contains("//NLM//DTD JATS") || doctype.contains("//NLM//DTD Journal") {
            return Some(JATS_MIME_TYPE);
        }
    }
    let root = root_start_tag(trimmed)?;
    if root_is_in_namespace(root, "http://docbook.org/ns/docbook") {
        return Some(DOCBOOK_MIME_TYPE);
    }
    if root_has_name_in_namespace(root, "kml", "http://www.opengis.net/kml/2.2") {
        return Some(KML_MIME_TYPE);
    }
    if root_has_name_in_namespace(root, "document", ODF_OFFICE_NAMESPACE)
        && root_attribute_value(root, "office:mimetype") == Some(ODG_MIME_TYPE)
    {
        return Some(ODG_FLAT_MIME_TYPE);
    }
    root_has_name_in_namespace(root, "html", "http://www.w3.org/1999/xhtml").then_some("application/xhtml+xml")
}

fn root_has_name_in_namespace(root: &str, expected_name: &str, namespace: &str) -> bool {
    let qualified_name = root
        .trim_start_matches('<')
        .split([' ', '\t', '\n', '\r', '>', '/'])
        .next()
        .unwrap_or_default();
    let local_name = qualified_name.rsplit(':').next().unwrap_or_default();
    local_name.eq_ignore_ascii_case(expected_name) && root_is_in_namespace(root, namespace)
}

/// Report whether the root element itself belongs to `namespace`.
///
/// A declaration alone proves nothing: an XSL stylesheet that transforms
/// DocBook binds the namespace on its own root. The element belongs to the
/// namespace only when the binding it carries is the one its name uses.
fn root_is_in_namespace(root: &str, namespace: &str) -> bool {
    let name = root
        .trim_start_matches('<')
        .split([' ', '\t', '\n', '\r', '>', '/'])
        .next()
        .unwrap_or_default();
    let binding = match name.split_once(':') {
        Some((prefix, _)) => format!("xmlns:{prefix}"),
        None => "xmlns".to_string(),
    };
    root_attribute_value(root, &binding).is_some_and(|uri| uri == namespace)
}

fn root_attribute_value<'a>(root: &'a str, expected_name: &str) -> Option<&'a str> {
    let mut rest = root.trim_start_matches('<');
    rest = &rest[rest.find(|character: char| character.is_ascii_whitespace())?..];
    loop {
        rest = rest.trim_start();
        if rest.starts_with('>') || rest.starts_with('/') {
            return None;
        }
        let name_length = rest
            .find(|character: char| character.is_ascii_whitespace() || matches!(character, '=' | '>' | '/'))
            .unwrap_or(rest.len());
        let (name, after_name) = rest.split_at(name_length);
        rest = after_name.trim_start();
        rest = rest.strip_prefix('=')?.trim_start();
        let quote = rest
            .chars()
            .next()
            .filter(|character| matches!(character, '"' | '\''))?;
        let value = &rest[quote.len_utf8()..];
        let end = value.find(quote)?;
        if name == expected_name {
            return Some(&value[..end]);
        }
        rest = &value[end + quote.len_utf8()..];
    }
}

/// Return the text of the first declaration that opens with `opener`.
///
/// The declaration ends at its own `>`. An internal subset may hold a `>`
/// inside brackets, so the scan tracks bracket depth rather than searching for
/// a `]` anywhere in the document: a `]` in the body would otherwise stretch
/// the declaration over the whole file.
fn declaration_of<'a>(trimmed: &'a str, opener: &str) -> Option<&'a str> {
    let start = trimmed.find(opener)?;
    let rest = &trimmed[start..];
    let tail = &rest[opener.len()..];
    let end = crate::utils::xml_utils::doctype_end(tail)?;
    Some(&rest[..opener.len() + end])
}

/// Return the start tag of the root element, skipping the prolog.
///
/// The scan stops at the first element, so a namespace bound deeper in the
/// document cannot claim the file.
fn root_start_tag(trimmed: &str) -> Option<&str> {
    let mut rest = trimmed;
    loop {
        let open = rest.find('<')?;
        rest = &rest[open..];
        if rest.starts_with("<?") || rest.starts_with("<!") {
            let skip = if rest.starts_with("<!DOCTYPE") {
                declaration_of(rest, "<!DOCTYPE").map(str::len)?
            } else {
                rest.find('>')?
            };
            debug_assert!(skip < rest.len(), "a declaration ends inside the input");
            rest = &rest[skip + 1..];
            continue;
        }
        let end = rest.find('>')?;
        return Some(&rest[..=end]);
    }
}

pub(crate) const PDF_MIME_TYPE: &str = "application/pdf";
pub(crate) const PLAIN_TEXT_MIME_TYPE: &str = "text/plain";
pub(crate) const POWER_POINT_MIME_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.presentation";
pub(crate) const DOCX_MIME_TYPE: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
pub(crate) const LEGACY_WORD_MIME_TYPE: &str = "application/msword";
pub(crate) const LEGACY_POWERPOINT_MIME_TYPE: &str = "application/vnd.ms-powerpoint";
/// Only reachable from `detect_ole2_package`, which is gated on the feature set that pulls in
/// the `cfb` crate; without one of those features nothing names this constant and `-D warnings`
/// rejects it as dead code. Gate must track that function's. ~keep
#[cfg(any(feature = "office", feature = "hwp", feature = "email"))]
pub(crate) const LEGACY_EXCEL_MIME_TYPE: &str = "application/vnd.ms-excel";

pub(crate) const PST_MIME_TYPE: &str = "application/vnd.ms-outlook-pst";
pub(crate) const WPD_MIME_TYPE: &str = "application/vnd.wordperfect";
pub(crate) const JSON_MIME_TYPE: &str = "application/json";
pub(crate) const GEOJSON_MIME_TYPE: &str = "application/geo+json";
pub(crate) const SQLITE_MIME_TYPE: &str = "application/vnd.sqlite3";
pub(crate) const GEOPACKAGE_MIME_TYPE: &str = "application/geopackage+sqlite3";
pub(crate) const XML_MIME_TYPE: &str = "application/xml";
pub(crate) const KML_MIME_TYPE: &str = "application/vnd.google-earth.kml+xml";
pub(crate) const SOURCE_CODE_MIME_TYPE: &str = "text/x-source-code";

pub(crate) const EXCEL_MIME_TYPE: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
pub(crate) const ODT_MIME_TYPE: &str = "application/vnd.oasis.opendocument.text";
pub(crate) const ODP_MIME_TYPE: &str = "application/vnd.oasis.opendocument.presentation";
pub(crate) const ODS_MIME_TYPE: &str = "application/vnd.oasis.opendocument.spreadsheet";
pub(crate) const ODG_MIME_TYPE: &str = "application/vnd.oasis.opendocument.graphics";
/// Flat single-file XML variant of [`ODG_MIME_TYPE`] (`.fodg`): the whole
/// package's `content.xml` inlined as one document, with no ZIP layer. The
/// packaged MIME type is carried inside it as the root element's
/// `office:mimetype` attribute rather than as a separate file, so content
/// detection reads that attribute (see `xml_vocabulary`) instead of sniffing
/// a ZIP signature. ~keep
pub(crate) const ODG_FLAT_MIME_TYPE: &str = "application/vnd.oasis.opendocument.graphics-flat-xml";
/// ODF namespace bound to the `office:` prefix, used to confirm the root
/// element of a flat ODF document is actually `office:document` before
/// trusting its `office:mimetype` attribute. ~keep
const ODF_OFFICE_NAMESPACE: &str = "urn:oasis:names:tc:opendocument:xmlns:office:1.0";
#[cfg(any(feature = "office", feature = "hwpx", feature = "iwork", feature = "archives"))]
const ZIP_MIME_TYPE: &str = "application/zip";

#[cfg(feature = "hwpx")]
pub(crate) const HWPX_MIME_TYPE: &str = "application/haansofthwpx";
pub(crate) const IWORK_PAGES_MIME_TYPE: &str = "application/x-iwork-pages-sffpages";
pub(crate) const IWORK_NUMBERS_MIME_TYPE: &str = "application/x-iwork-numbers-sffnumbers";
pub(crate) const IWORK_KEYNOTE_MIME_TYPE: &str = "application/x-iwork-keynote-sffkey";

/// Docling DocTags. The format has no registered media type, and its files are
/// conventionally named `*.doctags.txt`, so callers reading those will need to
/// pass this explicitly rather than relying on the extension.
pub(crate) const DOCTAGS_MIME_TYPE: &str = "text/vnd.docling.doctags";

/// A format definition in the centralized registry.
///
/// Each entry defines a document format with its file extensions, primary MIME type,
/// and any MIME type aliases that should also be accepted for this format.
struct FormatEntry {
    /// File extensions (without leading dot). First is canonical.
    extensions: &'static [&'static str],
    /// Primary MIME type for this format.
    mime_type: &'static str,
    /// Additional MIME type aliases that should also be accepted.
    aliases: &'static [&'static str],
}

/// Centralized format registry - the single source of truth for all supported formats.
///
/// Adding a new format requires only adding a single entry here. Both `EXT_TO_MIME`
/// (extension-to-MIME mapping) and `SUPPORTED_MIME_TYPES` (validation set) are
/// derived from this array automatically.
static FORMATS: &[FormatEntry] = &[
    FormatEntry {
        extensions: &["txt"],
        mime_type: "text/plain",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["adoc", "asciidoc"],
        mime_type: "text/asciidoc",
        aliases: &["text/x-asciidoc"],
    },
    FormatEntry {
        extensions: &["vtt"],
        mime_type: "text/vtt",
        aliases: &[],
    },
    // text/troff, text/x-mdoc, text/x-pod and text/x-dokuwiki were removed here (#228). They
    // carried no extensions, so they were unreachable except via a caller-supplied MIME, and
    // the only "extractor" behind them was the plain-text one — which BOM-stripped and split
    // on blank lines, turning roff macros and POD commands into prose that looked like a
    // successful extraction. Listing them made `validate_mime_type` return Ok for formats
    // nothing could actually parse, which is the advertised-but-unroutable half of GH#1387.
    FormatEntry {
        extensions: &["md", "markdown"],
        mime_type: "text/markdown",
        aliases: &["text/x-markdown"],
    },
    FormatEntry {
        extensions: &["commonmark"],
        mime_type: "text/x-commonmark",
        aliases: &[],
    },
    FormatEntry {
        extensions: &[],
        mime_type: "text/x-gfm",
        aliases: &[],
    },
    FormatEntry {
        extensions: &[],
        mime_type: "text/x-markdown-extra",
        aliases: &[],
    },
    FormatEntry {
        extensions: &[],
        mime_type: "text/x-multimarkdown",
        aliases: &[],
    },
    FormatEntry {
        extensions: &[],
        mime_type: "text/x-pandoc",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["qmd"],
        mime_type: "text/x-quarto",
        aliases: &["application/x-quarto"],
    },
    FormatEntry {
        extensions: &["rmd"],
        mime_type: "text/x-r-markdown",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["mdx"],
        mime_type: "text/mdx",
        aliases: &["text/x-mdx"],
    },
    FormatEntry {
        extensions: &["djot", "dj"],
        mime_type: "text/x-djot",
        aliases: &["text/djot"],
    },
    FormatEntry {
        extensions: &["doctags"],
        mime_type: DOCTAGS_MIME_TYPE,
        aliases: &["application/vnd.docling.doctags"],
    },
    FormatEntry {
        extensions: &["pdf"],
        mime_type: "application/pdf",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["html", "htm"],
        mime_type: "text/html",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["xhtml", "xht"],
        mime_type: "application/xhtml+xml",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["docx"],
        mime_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        aliases: &["application/docx"],
    },
    FormatEntry {
        extensions: &["docm"],
        mime_type: "application/vnd.ms-word.document.macroEnabled.12",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["dotx"],
        mime_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.template",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["dotm"],
        mime_type: "application/vnd.ms-word.template.macroEnabled.12",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["doc", "dot"],
        mime_type: "application/msword",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["odt"],
        mime_type: ODT_MIME_TYPE,
        aliases: &[],
    },
    FormatEntry {
        extensions: &["odp"],
        mime_type: ODP_MIME_TYPE,
        aliases: &[],
    },
    FormatEntry {
        extensions: &["pptx"],
        mime_type: "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["ppsx"],
        mime_type: "application/vnd.openxmlformats-officedocument.presentationml.slideshow",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["pptm"],
        mime_type: "application/vnd.ms-powerpoint.presentation.macroEnabled.12",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["potx"],
        mime_type: "application/vnd.openxmlformats-officedocument.presentationml.template",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["potm"],
        mime_type: "application/vnd.ms-powerpoint.template.macroEnabled.12",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["ppt", "pot", "pps"],
        mime_type: "application/vnd.ms-powerpoint",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["xlsx"],
        mime_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["xltx"],
        mime_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.template",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["xls", "xlt", "xla"],
        mime_type: "application/vnd.ms-excel",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["xlsm"],
        mime_type: "application/vnd.ms-excel.sheet.macroEnabled.12",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["xlsb"],
        mime_type: "application/vnd.ms-excel.sheet.binary.macroEnabled.12",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["xlam"],
        mime_type: "application/vnd.ms-excel.addin.macroEnabled.12",
        aliases: &["application/vnd.ms-excel.addin.macroEnabled"],
    },
    FormatEntry {
        extensions: &["xltm"],
        mime_type: "application/vnd.ms-excel.template.macroEnabled.12",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["ods"],
        mime_type: ODS_MIME_TYPE,
        aliases: &[],
    },
    FormatEntry {
        extensions: &["fodg"],
        mime_type: ODG_FLAT_MIME_TYPE,
        aliases: &[],
    },
    FormatEntry {
        extensions: &["dbf"],
        mime_type: "application/vnd.dbf",
        aliases: &["application/x-dbf", "application/dbase"],
    },
    FormatEntry {
        extensions: &["sqlite", "sqlite3", "db"],
        mime_type: SQLITE_MIME_TYPE,
        aliases: &["application/x-sqlite3"],
    },
    FormatEntry {
        extensions: &["gpkg", "gpkx"],
        mime_type: GEOPACKAGE_MIME_TYPE,
        aliases: &[],
    },
    FormatEntry {
        extensions: &["hwp"],
        mime_type: "application/x-hwp",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["hwpx"],
        mime_type: "application/haansofthwpx",
        aliases: &["application/hwp+zip"],
    },
    FormatEntry {
        extensions: &["wpd", "wp", "wp5", "wp6"],
        mime_type: WPD_MIME_TYPE,
        aliases: &["application/wordperfect"],
    },
    FormatEntry {
        extensions: &["bmp"],
        mime_type: "image/bmp",
        aliases: &["image/x-bmp", "image/x-ms-bmp"],
    },
    FormatEntry {
        extensions: &["gif"],
        mime_type: "image/gif",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["jpg", "jpeg"],
        mime_type: "image/jpeg",
        aliases: &["image/pjpeg", "image/jpg"],
    },
    FormatEntry {
        extensions: &["png"],
        mime_type: "image/png",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["tiff", "tif"],
        mime_type: "image/tiff",
        aliases: &["image/x-tiff"],
    },
    FormatEntry {
        extensions: &["webp"],
        mime_type: "image/webp",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["jp2", "jpg2"],
        mime_type: "image/jp2",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["j2c", "j2k", "jpc"],
        mime_type: "image/j2c",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["jbig2", "jb2"],
        mime_type: "image/x-jbig2",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["heic"],
        mime_type: "image/heic",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["heics"],
        mime_type: "image/heic-sequence",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["heif", "hif"],
        mime_type: "image/heif",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["heifs"],
        mime_type: "image/heif-sequence",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["avif"],
        mime_type: "image/avif",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["avcs"],
        mime_type: "image/avcs",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["pnm"],
        mime_type: "image/x-portable-anymap",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["pbm"],
        mime_type: "image/x-portable-bitmap",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["pgm"],
        mime_type: "image/x-portable-graymap",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["ppm"],
        mime_type: "image/x-portable-pixmap",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["csv"],
        mime_type: "text/csv",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["tsv"],
        mime_type: "text/tab-separated-values",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["json"],
        mime_type: "application/json",
        aliases: &["text/json"],
    },
    FormatEntry {
        extensions: &["geojson"],
        mime_type: GEOJSON_MIME_TYPE,
        aliases: &["application/vnd.geo+json"],
    },
    FormatEntry {
        extensions: &[],
        mime_type: "application/csl+json",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["jsonl", "ndjson"],
        mime_type: "application/x-ndjson",
        aliases: &["application/jsonl", "application/x-jsonlines"],
    },
    FormatEntry {
        extensions: &["yaml", "yml"],
        mime_type: "application/yaml",
        aliases: &["application/x-yaml", "text/yaml", "text/x-yaml"],
    },
    FormatEntry {
        extensions: &["toml"],
        mime_type: "application/toml",
        aliases: &["text/toml"],
    },
    FormatEntry {
        extensions: &["xml"],
        mime_type: "application/xml",
        aliases: &["text/xml"],
    },
    FormatEntry {
        extensions: &["kml"],
        mime_type: KML_MIME_TYPE,
        aliases: &[],
    },
    FormatEntry {
        extensions: &["svg"],
        mime_type: "image/svg+xml",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["eml"],
        mime_type: "message/rfc822",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["msg"],
        mime_type: "application/vnd.ms-outlook",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["pst"],
        mime_type: "application/vnd.ms-outlook-pst",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["zip"],
        mime_type: "application/zip",
        aliases: &["application/x-zip-compressed"],
    },
    FormatEntry {
        extensions: &["tar"],
        mime_type: "application/x-tar",
        aliases: &["application/tar", "application/x-gtar", "application/x-ustar"],
    },
    FormatEntry {
        extensions: &["gz", "tgz"],
        mime_type: "application/gzip",
        aliases: &["application/x-gzip"],
    },
    FormatEntry {
        extensions: &["7z"],
        mime_type: "application/x-7z-compressed",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["rst"],
        mime_type: "text/prs.fallenstein.rst",
        aliases: &["text/x-rst"],
    },
    FormatEntry {
        extensions: &["org"],
        mime_type: "text/org",
        aliases: &["text/x-org", "application/x-org"],
    },
    FormatEntry {
        extensions: &["epub"],
        mime_type: "application/epub+zip",
        aliases: &["application/x-epub+zip", "application/vnd.epub+zip"],
    },
    FormatEntry {
        extensions: &["rtf"],
        mime_type: "application/rtf",
        aliases: &["text/rtf"],
    },
    FormatEntry {
        extensions: &["bib"],
        mime_type: "application/x-bibtex",
        aliases: &["text/x-bibtex", "application/x-biblatex"],
    },
    FormatEntry {
        extensions: &["ris"],
        mime_type: "application/x-research-info-systems",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["nbib"],
        mime_type: "application/x-pubmed",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["enw"],
        mime_type: "application/x-endnote+xml",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["fb2"],
        mime_type: "application/x-fictionbook+xml",
        aliases: &["application/x-fictionbook", "text/x-fictionbook"],
    },
    FormatEntry {
        extensions: &["opml"],
        mime_type: "application/xml+opml",
        aliases: &["application/x-opml+xml", "text/x-opml"],
    },
    FormatEntry {
        extensions: &["dbk", "docbook", "docbook4", "docbook5"],
        mime_type: "application/docbook+xml",
        aliases: &["text/docbook"],
    },
    FormatEntry {
        extensions: &["jats", "nxml"],
        mime_type: "application/x-jats+xml",
        aliases: &["text/jats"],
    },
    FormatEntry {
        extensions: &["ipynb"],
        mime_type: "application/x-ipynb+json",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["tex", "latex"],
        mime_type: "application/x-latex",
        aliases: &["text/x-tex", "application/x-tex"],
    },
    FormatEntry {
        extensions: &["typst", "typ"],
        mime_type: "text/vnd.typst",
        aliases: &["text/x-typst", "application/x-typst"],
    },
    FormatEntry {
        extensions: &["pages"],
        mime_type: "application/x-iwork-pages-sffpages",
        aliases: &["application/vnd.apple.pages"],
    },
    FormatEntry {
        extensions: &["numbers"],
        mime_type: "application/x-iwork-numbers-sffnumbers",
        aliases: &["application/vnd.apple.numbers"],
    },
    FormatEntry {
        extensions: &["key"],
        mime_type: "application/x-iwork-keynote-sffkey",
        aliases: &["application/vnd.apple.keynote"],
    },
    FormatEntry {
        extensions: &["mp3", "mpga"],
        mime_type: "audio/mpeg",
        aliases: &["audio/mp3"],
    },
    FormatEntry {
        extensions: &["m4a"],
        mime_type: "audio/mp4",
        aliases: &["audio/x-m4a"],
    },
    FormatEntry {
        extensions: &["wav"],
        mime_type: "audio/wav",
        aliases: &["audio/x-wav"],
    },
    FormatEntry {
        extensions: &["webm"],
        mime_type: "audio/webm",
        aliases: &["video/webm"],
    },
    FormatEntry {
        extensions: &["mp4", "mpg4", "mp4v", "m4v"],
        mime_type: "video/mp4",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["mpeg", "mpg", "mpe", "m1v", "m2v"],
        mime_type: "video/mpeg",
        aliases: &[],
    },
    FormatEntry {
        extensions: &[],
        mime_type: "text/x-source-code",
        aliases: &[
            "application/x-sh",
            "text/x-sh",
            "text/x-shellscript",
            "text/x-python",
            "text/x-r-source",
            "text/x-julia",
            "text/vnd.graphviz",
            "text/x-plantuml",
        ],
    },
];

const fn extensions_equal(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}

const fn count_unique_extensions() -> usize {
    let mut count = 0;
    let mut format_index = 0;
    while format_index < FORMATS.len() {
        let mut extension_index = 0;
        while extension_index < FORMATS[format_index].extensions.len() {
            let extension = FORMATS[format_index].extensions[extension_index];
            let mut duplicate = false;
            let mut earlier_format = 0;
            while earlier_format <= format_index {
                let limit = if earlier_format == format_index {
                    extension_index
                } else {
                    FORMATS[earlier_format].extensions.len()
                };
                let mut earlier_extension = 0;
                while earlier_extension < limit {
                    if extensions_equal(extension, FORMATS[earlier_format].extensions[earlier_extension]) {
                        duplicate = true;
                    }
                    earlier_extension += 1;
                }
                earlier_format += 1;
            }
            if !duplicate {
                count += 1;
            }
            extension_index += 1;
        }
        format_index += 1;
    }
    count
}

/// Number of formats in Xberg's static MIME registry. ~keep
#[cfg_attr(alef, alef(skip))]
pub const SUPPORTED_FORMAT_COUNT: usize = FORMATS.len();

/// Number of unique file extensions in Xberg's static MIME registry. ~keep
#[cfg_attr(alef, alef(skip))]
pub const SUPPORTED_EXTENSION_COUNT: usize = count_unique_extensions();

/// Extension to MIME type mapping, derived from [`FORMATS`].
static EXT_TO_MIME: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    for entry in FORMATS {
        for ext in entry.extensions {
            m.insert(*ext, entry.mime_type);
        }
    }
    m
});

/// All supported MIME types (primary + aliases), derived from [`FORMATS`].
static SUPPORTED_MIME_TYPES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut set = HashSet::new();
    for entry in FORMATS {
        set.insert(entry.mime_type);
        for alias in entry.aliases {
            set.insert(*alias);
        }
    }
    set
});

/// Detect MIME type from a file path.
///
/// Uses file extension to determine MIME type. Falls back to `mime_guess` crate
/// if extension-based detection fails.
///
/// # Arguments
///
/// * `path` - Path to the file
/// * `check_exists` - Whether to verify file existence
///
/// # Returns
///
/// The detected MIME type string.
///
/// # Errors
///
/// Returns `XbergError::Io` if file doesn't exist (when `check_exists` is true).
/// Returns `XbergError::UnsupportedFormat` if MIME type cannot be determined.
pub fn detect_mime_type(path: impl AsRef<Path>, check_exists: bool) -> Result<String> {
    let path = path.as_ref();

    if check_exists && !path.exists() {
        return Err(XbergError::from(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("File does not exist: {}", path.display()),
        )));
    }

    let extension = path.extension().and_then(|ext| ext.to_str()).map(|s| s.to_lowercase());
    tracing::debug!(path = %path.display(), extension = ?extension, "detecting MIME type from path");

    if let Some(ext) = &extension
        && let Some(mime_type) = EXT_TO_MIME.get(ext.as_str())
    {
        tracing::debug!(ext = %ext, mime_type = %mime_type, "matched via EXT_TO_MIME");
        return Ok(mime_type.to_string());
    }

    #[cfg(feature = "tree-sitter")]
    {
        if let Some(ext) = &extension {
            let lang = tree_sitter_language_pack::detect_language_from_extension(ext);
            tracing::debug!(ext = %ext, detected_language = ?lang, "tree-sitter extension detection");
            if lang.is_some() {
                return Ok(SOURCE_CODE_MIME_TYPE.to_string());
            }
        }
    }

    let guess = mime_guess::from_path(path).first();
    tracing::debug!(guess = ?guess, "mime_guess fallback");
    if let Some(mime) = guess {
        return Ok(mime.to_string());
    }

    if let Some(ext) = extension {
        return Err(XbergError::UnsupportedFormat(format!("Unknown extension: .{}", ext)));
    }

    Err(XbergError::validation(format!(
        "Could not determine MIME type from file path: {}",
        path.display()
    )))
}

/// Validate that a MIME type is supported.
///
/// # Arguments
///
/// * `mime_type` - The MIME type to validate
///
/// # Returns
///
/// The validated MIME type (may be normalized).
///
/// # Errors
///
/// Returns `XbergError::UnsupportedFormat` if not supported.
#[cfg_attr(alef, alef(skip))]
pub fn validate_mime_type(mime_type: &str) -> Result<String> {
    let parsed = mime_type.trim().parse::<mime::Mime>().map_err(|_| {
        tracing::debug!(mime_type = %mime_type, "MIME type has invalid syntax");
        XbergError::UnsupportedFormat(mime_type.to_string())
    })?;
    let essence = parsed.essence_str();
    if let Some(supported) = SUPPORTED_MIME_TYPES
        .iter()
        .find(|supported| supported.eq_ignore_ascii_case(essence))
    {
        tracing::trace!(mime_type = %mime_type, matched = %supported, "MIME type validated by essence");
        return Ok(normalize_registered_mime_type(supported).to_string());
    }

    tracing::debug!(mime_type = %mime_type, essence, "MIME type not in supported set");
    Err(XbergError::UnsupportedFormat(mime_type.to_string()))
}

fn normalize_registered_mime_type(mime_type: &str) -> &str {
    // ~keep Preserve existing alias return values; added client spellings use canonical routing.
    match mime_type {
        "application/x-tex" => "application/x-latex",
        "application/x-sh" | "text/x-sh" | "text/x-shellscript" => SOURCE_CODE_MIME_TYPE,
        "application/vnd.apple.pages" => IWORK_PAGES_MIME_TYPE,
        "application/vnd.apple.numbers" => IWORK_NUMBERS_MIME_TYPE,
        "application/vnd.apple.keynote" => IWORK_KEYNOTE_MIME_TYPE,
        _ => mime_type,
    }
}

fn is_generic_mime_type(mime_type: &str) -> bool {
    mime_type
        .trim()
        .parse::<mime::Mime>()
        .is_ok_and(|parsed| parsed.essence_str().eq_ignore_ascii_case(OCTET_STREAM_MIME_TYPE))
}

/// Detect or validate MIME type.
///
/// If `mime_type` is provided, validates it. Otherwise, detects from `path`.
///
/// # Arguments
///
/// * `path` - Optional path to detect MIME type from
/// * `mime_type` - Optional explicit MIME type to validate
///
/// # Returns
///
/// The validated MIME type string.
#[cfg(test)]
pub(crate) fn detect_or_validate(path: Option<&str>, mime_type: Option<&str>) -> Result<String> {
    if let Some(mime) = mime_type {
        tracing::debug!(mime_type = %mime, "validating caller-provided MIME type");
        validate_mime_type(mime)
    } else if let Some(p) = path.map(Path::new) {
        let detected = detect_mime_type(p, true);
        let mut file = std::fs::File::open(p).ok();
        resolve_file_mime(p, detected, file.as_mut())
    } else {
        Err(XbergError::validation(
            "Must provide either path or mime_type".to_string(),
        ))
    }
}

pub(crate) fn detect_or_validate_file(
    path: &Path,
    file: &mut std::fs::File,
    mime_type: Option<&str>,
    policy: crate::core::config::MimeDetectionPolicy,
) -> Result<String> {
    if let Some(mime) = mime_type.filter(|mime| !is_generic_mime_type(mime)) {
        tracing::debug!(mime_type = %mime, "validating caller-provided MIME type");
        return validate_mime_type(mime);
    }

    use crate::core::config::MimeDetectionPolicy;

    match policy {
        MimeDetectionPolicy::PreferContent => resolve_file_mime(path, detect_mime_type(path, false), Some(file)),
        MimeDetectionPolicy::TrustExtension => {
            let extension = detect_mime_type(path, false);
            if let Ok(ref detected) = extension
                && let Ok(validated) = validate_mime_type(detected)
            {
                return Ok(validated);
            }
            resolve_file_mime(path, extension, Some(file))
        }
        MimeDetectionPolicy::ContentOnly => detect_mime_type_from_file_content(file, PackageInspection::FullArchive)
            .ok_or_else(|| XbergError::validation("Could not detect MIME type from file content".to_string()))
            .and_then(|detected| validate_mime_type(&detected)),
    }
}

pub(crate) fn detect_or_validate_bytes(
    content: &[u8],
    filename: Option<&str>,
    mime_type: Option<&str>,
    policy: crate::core::config::MimeDetectionPolicy,
) -> Result<String> {
    if let Some(mime) = mime_type.filter(|mime| !is_generic_mime_type(mime)) {
        tracing::debug!(mime_type = %mime, "validating caller-provided MIME type");
        return validate_mime_type(mime);
    }

    use crate::core::config::MimeDetectionPolicy;
    match policy {
        MimeDetectionPolicy::TrustExtension => {
            let extension = filename.and_then(|name| detect_mime_type(name, false).ok());
            if let Some(ref detected) = extension
                && let Ok(validated) = validate_mime_type(detected)
            {
                return Ok(validated);
            }
            detect_mime_type_from_bytes(content).and_then(|detected| validate_mime_type(&detected))
        }
        MimeDetectionPolicy::ContentOnly => {
            detect_mime_type_from_bytes(content).and_then(|detected| validate_mime_type(&detected))
        }
        MimeDetectionPolicy::PreferContent => {
            let extension = filename.and_then(|name| detect_mime_type(name, false).ok());
            let from_content = detect_mime_type_from_bytes(content);
            match (extension, from_content) {
                (Some(extension), Ok(content_mime)) => {
                    prefer_content_mime(&extension, &content_mime, filename.map(Path::new))
                }
                (Some(extension), Err(_)) => validate_mime_type(&extension),
                (None, Ok(content_mime)) => validate_mime_type(&content_mime),
                (None, Err(error)) => Err(error),
            }
        }
    }
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
pub(crate) async fn resolve_owned_bytes_mime(
    content: Vec<u8>,
    filename: Option<String>,
    mime_type: Option<String>,
    policy: crate::core::config::MimeDetectionPolicy,
) -> Result<(Vec<u8>, String)> {
    if let Some(explicit) = mime_type.as_deref().filter(|mime| !is_generic_mime_type(mime)) {
        return validate_mime_type(explicit).map(|validated| (content, validated));
    }
    if policy == crate::core::config::MimeDetectionPolicy::TrustExtension
        && let Some(filename) = filename.as_deref()
        && let Ok(detected) = detect_mime_type(filename, false)
        && let Ok(validated) = validate_mime_type(&detected)
    {
        return Ok((content, validated));
    }

    tokio::task::spawn_blocking(move || {
        let detected = detect_or_validate_bytes(&content, filename.as_deref(), mime_type.as_deref(), policy)?;
        Ok((content, detected))
    })
    .await
    .map_err(|error| {
        tracing::error!(%error, "byte MIME detection task failed");
        XbergError::Other("Byte MIME detection task failed".to_string())
    })?
}

#[cfg(any(not(feature = "tokio-runtime"), target_arch = "wasm32"))]
pub(crate) async fn resolve_owned_bytes_mime(
    content: Vec<u8>,
    filename: Option<String>,
    mime_type: Option<String>,
    policy: crate::core::config::MimeDetectionPolicy,
) -> Result<(Vec<u8>, String)> {
    let detected = detect_or_validate_bytes(&content, filename.as_deref(), mime_type.as_deref(), policy)?;
    Ok((content, detected))
}

#[cfg(feature = "tree-sitter")]
fn is_plain_graphviz_dot(path: Option<&Path>, content_mime: &str) -> bool {
    content_mime == PLAIN_TEXT_MIME_TYPE
        && path
            .and_then(Path::extension)
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("dot"))
}

fn prefer_content_mime(extension_mime: &str, content_mime: &str, _path: Option<&Path>) -> Result<String> {
    #[cfg(feature = "tree-sitter")]
    if is_plain_graphviz_dot(_path, content_mime) {
        return Ok(SOURCE_CODE_MIME_TYPE.to_string());
    }
    let extension_supported = SUPPORTED_MIME_TYPES.contains(extension_mime);
    if extension_supported
        && (content_mime == PLAIN_TEXT_MIME_TYPE
            || (is_generic_xml_mime(content_mime) && is_specific_xml_mime(extension_mime))
            || (content_mime == JSON_MIME_TYPE && is_specific_json_mime(extension_mime))
            || is_compatible_ooxml_mime(extension_mime, content_mime))
    {
        return validate_mime_type(extension_mime);
    }
    validate_mime_type(content_mime).or_else(|_| validate_mime_type(extension_mime))
}

fn resolve_file_mime(path: &Path, path_detection: Result<String>, file: Option<&mut std::fs::File>) -> Result<String> {
    let detected = match path_detection {
        Ok(detected) => detected,
        Err(path_error) => {
            if let Some(from_content) =
                file.and_then(|file| detect_mime_type_from_file_content(file, PackageInspection::HeaderOnly))
                && let Ok(validated) = validate_mime_type(&from_content)
            {
                tracing::debug!(path = %path.display(), mime_type = %validated,
                        "path MIME unavailable; matched via content");
                return Ok(validated);
            }
            return Err(path_error);
        }
    };
    let resolved = match file.and_then(|file| magic_override(file, &detected, path)) {
        Some(from_magic) => {
            tracing::debug!(path = %path.display(), extension_mime = %detected, magic_mime = %from_magic,
                    "extension/content MIME disagree; preferring content");
            from_magic
        }
        None => detected,
    };
    validate_mime_type(&resolved)
}

/// If the file's magic bytes confidently indicate a different supported MIME
/// type than the extension did, return it. Returns `None` when the content has
/// no signature, the read fails, or content and extension agree.
fn magic_override(file: &mut std::fs::File, extension_mime: &str, _path: &Path) -> Option<String> {
    let from_magic = detect_mime_type_from_file_content(file, PackageInspection::FullArchive)?;

    #[cfg(feature = "tree-sitter")]
    if is_plain_graphviz_dot(Some(_path), &from_magic) {
        return Some(SOURCE_CODE_MIME_TYPE.to_string());
    }
    if from_magic == PLAIN_TEXT_MIME_TYPE && SUPPORTED_MIME_TYPES.contains(extension_mime) {
        return None;
    }
    if is_generic_xml_mime(&from_magic)
        && is_specific_xml_mime(extension_mime)
        && SUPPORTED_MIME_TYPES.contains(extension_mime)
    {
        return None;
    }
    if from_magic == JSON_MIME_TYPE
        && is_specific_json_mime(extension_mime)
        && SUPPORTED_MIME_TYPES.contains(extension_mime)
    {
        return None;
    }
    if is_compatible_ooxml_mime(extension_mime, &from_magic) {
        return None;
    }
    if from_magic != extension_mime && validate_mime_type(&from_magic).is_ok() {
        Some(from_magic)
    } else {
        None
    }
}

fn detect_mime_type_from_file_content(
    file: &mut std::fs::File,
    package_inspection: PackageInspection,
) -> Option<String> {
    file.seek(SeekFrom::Start(0)).ok()?;
    let mut header = [0_u8; MIME_SNIFF_LENGTH];
    let bytes_read = file.read(&mut header).ok()?;
    if bytes_read == 0 {
        return None;
    }

    let header = &header[..bytes_read];
    let content_continues = bytes_read == MIME_SNIFF_LENGTH
        && file
            .metadata()
            .ok()
            .is_some_and(|metadata| metadata.len() > bytes_read as u64);
    let json_candidate = bounded_content_may_be_json(file, header, content_continues);
    let mut from_magic = match detect_mime_type_from_bytes_with_inspection(header, package_inspection) {
        Ok(detected) => detected,
        Err(_) if json_candidate => JSON_MIME_TYPE.to_string(),
        Err(_) => {
            // An MS-CFB compound document (.doc/.xls/.ppt) cannot be typed from a
            // fixed-size prefix: identifying it means following the FAT sector
            // chain to the root directory entry, and a truncated buffer references
            // sectors that are not present in `header` (#1590). Escape to a
            // structure-aware read over the file the same way the ZIP branch below
            // escapes to `detect_zip_package` for an inconclusive archive header. ~keep
            #[cfg(any(feature = "office", feature = "hwp", feature = "email"))]
            if package_inspection == PackageInspection::FullArchive && header.starts_with(&OLE2_MAGIC[..]) {
                return detect_ole2_package(&mut *file, &SecurityLimits::default());
            }
            return None;
        }
    };
    if matches!(from_magic.as_str(), PLAIN_TEXT_MIME_TYPE | OCTET_STREAM_MIME_TYPE) && json_candidate {
        from_magic = JSON_MIME_TYPE.to_string();
    }
    #[cfg(any(feature = "office", feature = "hwpx", feature = "iwork", feature = "archives"))]
    if package_inspection == PackageInspection::FullArchive
        && (from_magic == ZIP_MIME_TYPE || from_magic.starts_with("application/vnd.oasis.opendocument."))
        && let Some(package_mime) = detect_zip_package(&mut *file)
    {
        return Some(package_mime.to_string());
    }

    Some(from_magic)
}

fn bounded_content_may_be_json(file: &mut std::fs::File, header: &[u8], content_continues: bool) -> bool {
    let mut probe = Vec::with_capacity(MIME_SNIFF_LENGTH * 2);
    probe.extend_from_slice(header);
    if content_continues {
        let mut continuation = [0_u8; MIME_SNIFF_LENGTH];
        let bytes_read = file.read(&mut continuation).unwrap_or_default();
        probe.extend_from_slice(&continuation[..bytes_read]);
        let _ = file.seek(SeekFrom::Start(header.len() as u64));
    }

    if !probe
        .iter()
        .copied()
        .find(|byte| !matches!(byte, b' ' | b'\n' | b'\r' | b'\t'))
        .is_some_and(|byte| matches!(byte, b'{' | b'['))
    {
        return false;
    }

    let mut deserializer = serde_json::Deserializer::from_slice(&probe);
    match serde::de::IgnoredAny::deserialize(&mut deserializer) {
        Ok(_) => deserializer.end().is_ok(),
        Err(error) => error.is_eof() && content_continues,
    }
}

/// Generic XML signatures cannot distinguish specialized XML vocabularies.
/// Preserve a supported extension-specific XML MIME so extractor selection can
/// route formats such as FictionBook and DocBook to their semantic parsers.
fn is_specific_xml_mime(mime_type: &str) -> bool {
    mime_type != XML_MIME_TYPE && (mime_type.ends_with("+xml") || mime_type.contains("xml+"))
}

fn is_generic_xml_mime(mime_type: &str) -> bool {
    matches!(mime_type, XML_MIME_TYPE | "text/xml")
}

/// Generic JSON detection cannot distinguish JSON-based document formats.
/// Preserve extension-specific routing for notebooks and line-delimited JSON. ~keep
fn is_specific_json_mime(mime_type: &str) -> bool {
    mime_type != JSON_MIME_TYPE
        && (mime_type.ends_with("+json")
            || matches!(
                mime_type,
                "application/x-ndjson" | "application/jsonl" | "application/x-jsonlines"
            ))
}

fn is_compatible_ooxml_mime(extension_mime: &str, content_mime: &str) -> bool {
    match content_mime {
        DOCX_MIME_TYPE => matches!(
            extension_mime,
            DOCX_MIME_TYPE
                | "application/vnd.ms-word.document.macroEnabled.12"
                | "application/vnd.openxmlformats-officedocument.wordprocessingml.template"
                | "application/vnd.ms-word.template.macroEnabled.12"
        ),
        POWER_POINT_MIME_TYPE => matches!(
            extension_mime,
            POWER_POINT_MIME_TYPE
                | "application/vnd.ms-powerpoint.presentation.macroEnabled.12"
                | "application/vnd.openxmlformats-officedocument.presentationml.slideshow"
                | "application/vnd.openxmlformats-officedocument.presentationml.template"
                | "application/vnd.ms-powerpoint.template.macroEnabled.12"
        ),
        EXCEL_MIME_TYPE => matches!(
            extension_mime,
            EXCEL_MIME_TYPE
                | "application/vnd.ms-excel.sheet.macroEnabled.12"
                | "application/vnd.ms-excel.addin.macroEnabled.12"
                | "application/vnd.ms-excel.template.macroEnabled.12"
                | "application/vnd.ms-excel.sheet.binary.macroEnabled.12"
                | "application/vnd.openxmlformats-officedocument.spreadsheetml.template"
        ),
        _ => false,
    }
}

/// Detect MIME type from raw file bytes.
///
/// Uses magic byte signatures to detect file type from content.
/// Falls back to `infer` crate for comprehensive detection.
///
/// For ZIP-based files, inspects contents to distinguish Office Open XML
/// formats (DOCX, XLSX, PPTX) from plain ZIP archives.
///
/// # Arguments
///
/// * `content` - Raw file bytes
///
/// # Returns
///
/// The detected MIME type string.
///
/// # Errors
///
/// Returns `XbergError::UnsupportedFormat` if MIME type cannot be determined.
pub fn detect_mime_type_from_bytes(content: &[u8]) -> Result<String> {
    detect_mime_type_from_bytes_with_inspection(content, PackageInspection::FullArchive)
}

fn detect_mime_type_from_bytes_with_inspection(
    content: &[u8],
    package_inspection: PackageInspection,
) -> Result<String> {
    if content.starts_with(b"SQLite format 3\0") {
        let application_id =
            content.get(SQLITE_APPLICATION_ID_OFFSET..SQLITE_APPLICATION_ID_OFFSET + SQLITE_APPLICATION_ID_LENGTH);
        if application_id.is_some_and(|identifier| {
            identifier == GEOPACKAGE_APPLICATION_ID || identifier == GEOPACKAGE_LEGACY_APPLICATION_ID
        }) {
            return Ok(GEOPACKAGE_MIME_TYPE.to_string());
        }
    }
    if content.starts_with(b"SQLite format 3\0") {
        return Ok(SQLITE_MIME_TYPE.to_string());
    }
    if content.starts_with(J2C_CODESTREAM_MAGIC) {
        return Ok("image/j2c".to_string());
    }

    if let Some(kind) = infer::get(content) {
        let mime_type = kind.mime_type();

        #[cfg(any(feature = "office", feature = "hwpx", feature = "iwork", feature = "archives"))]
        if mime_type.starts_with("application/vnd.oasis.opendocument.") {
            if package_inspection == PackageInspection::HeaderOnly {
                return Ok(ZIP_MIME_TYPE.to_string());
            }
            return Ok(detect_zip_package(std::io::Cursor::new(content))
                .unwrap_or(ZIP_MIME_TYPE)
                .to_string());
        }

        if mime_type == "application/zip"
            && let Some(office_mime) = detect_office_format_from_zip(content, package_inspection)
        {
            return Ok(office_mime.to_string());
        }

        if SUPPORTED_MIME_TYPES.contains(mime_type) || mime_type.starts_with("image/") {
            // `infer` reads the `<?xml` declaration and stops at generic XML, so
            // the vocabulary check has to run before that result is returned.
            // A caller may pass a truncated header, so decode lossily: a split
            // multi-byte character must not suppress the check.
            let prolog = String::from_utf8_lossy(&content[..content.len().min(8192)]);
            if is_generic_xml_mime(mime_type)
                && let Some(vocabulary) = xml_vocabulary(prolog.trim_start())
            {
                return Ok(vocabulary.to_string());
            }
            return Ok(mime_type.to_string());
        }
    }

    if content.len() >= 4 && content[..4] == [0x21, 0x42, 0x44, 0x4E] {
        return Ok(PST_MIME_TYPE.to_string());
    }

    // WordPerfect (Windows/DOS variants): magic bytes `\xffWPC`. The Mac
    // WordPerfect variant has no reliable magic bytes and is routed by the
    // `.wpd` extension via `EXT_TO_MIME` instead.
    if content.len() >= 4 && content[..4] == [0xFF, 0x57, 0x50, 0x43] {
        return Ok(WPD_MIME_TYPE.to_string());
    }

    if let Ok(text) = std::str::from_utf8(content) {
        let trimmed = text.trim_start();

        if (trimmed.starts_with('{') || trimmed.starts_with('['))
            && let Ok(value) = serde_json::from_str::<serde_json::Value>(text)
        {
            let mime_type = if is_geojson(&value) {
                GEOJSON_MIME_TYPE
            } else {
                JSON_MIME_TYPE
            };
            return Ok(mime_type.to_string());
        }

        // The HTML checks must precede the generic `<` fallback. They used to follow it,
        // where `trimmed.starts_with('<')` matched every tag first and made them dead code
        // (#235). HTML still routed correctly for whole documents only because `infer::get`
        // recognises those earlier in this function; a bare fragment reached here and was
        // typed `application/xml`, then handed to the XML extractor. ~keep
        if !trimmed.starts_with("<?xml") && looks_like_html(trimmed) {
            return Ok(HTML_MIME_TYPE.to_string());
        }

        if trimmed.starts_with("<?xml") || trimmed.starts_with('<') {
            if let Some(vocabulary) = xml_vocabulary(trimmed) {
                return Ok(vocabulary.to_string());
            }
            return Ok(XML_MIME_TYPE.to_string());
        }

        if trimmed.starts_with("%PDF") {
            return Ok(PDF_MIME_TYPE.to_string());
        }

        #[cfg(feature = "tree-sitter")]
        if tree_sitter_language_pack::detect_language_from_content(trimmed).is_some() {
            return Ok(SOURCE_CODE_MIME_TYPE.to_string());
        }

        return Ok(PLAIN_TEXT_MIME_TYPE.to_string());
    }

    // The bytes are not valid UTF-8, but a legacy single-byte encoding (Windows-1252,
    // ISO-8859-1) can still be text our extractors read -- e.g. the CSV extractor's own
    // `encoding_rs`/`chardetng` decoding (xberg-io/xberg#1625). Detecting that encoding
    // here would duplicate that decoder, so this only asks whether the raw bytes are
    // plausibly text at all, via the byte-value distribution a real single-byte-encoded
    // document has. ~keep
    if looks_like_legacy_encoded_text(content) {
        return Ok(PLAIN_TEXT_MIME_TYPE.to_string());
    }

    Err(XbergError::UnsupportedFormat(
        "Could not determine MIME type from bytes".to_string(),
    ))
}

/// Minimum fraction of `content` bytes that must fall in [`is_legacy_text_byte`]'s range for
/// non-UTF-8 `content` to be accepted as legacy-encoded plain text (#1625).
///
/// Genuine prose in a single-byte Western encoding is close to 100% printable ASCII plus a
/// small fraction of accented high bytes; unrelated binary formats mix in enough control and
/// otherwise-unmapped bytes to fall well short of this even though single-byte encodings like
/// Windows-1252 map every byte value and so "decode" without error either way.
const LEGACY_TEXT_PRINTABLE_BYTE_RATIO: f64 = 0.95;

/// The five byte values in `0x80..=0x9F` that Windows-1252 leaves undefined. Every other byte in
/// that range maps to a printable character -- including the curly quotes, en/em dashes and
/// ellipsis that Word and Excel emit, which are the most common marker that a file is CP1252 and
/// not ISO-8859-1. Treating the whole range as non-text rejected a 38-byte CP1252 sentence
/// containing one pair of curly quotes, because two bytes out of 38 already exceed
/// [`LEGACY_TEXT_PRINTABLE_BYTE_RATIO`]. ~keep
const WINDOWS_1252_UNDEFINED_BYTES: [u8; 5] = [0x81, 0x8D, 0x8F, 0x90, 0x9D];

/// Whether `byte` is one that a legacy single-byte text encoding (Windows-1252, ISO-8859-1) maps
/// to a printable character, or is common whitespace.
fn is_legacy_text_byte(byte: u8) -> bool {
    match byte {
        0x09 | 0x0A | 0x0D | 0x20..=0x7E | 0xA0..=0xFF => true,
        0x80..=0x9F => !WINDOWS_1252_UNDEFINED_BYTES.contains(&byte),
        _ => false,
    }
}

/// Heuristic check for legacy-encoded (non-UTF-8) plain text, used only after the UTF-8 fast
/// path above has already failed. A NUL byte rules out text outright; otherwise the content is
/// accepted when at least [`LEGACY_TEXT_PRINTABLE_BYTE_RATIO`] of its bytes are printable
/// (see [`is_legacy_text_byte`]).
fn looks_like_legacy_encoded_text(content: &[u8]) -> bool {
    if content.is_empty() || content.contains(&0) {
        return false;
    }

    let printable_count = content.iter().filter(|&&byte| is_legacy_text_byte(byte)).count();
    (printable_count as f64 / content.len() as f64) >= LEGACY_TEXT_PRINTABLE_BYTE_RATIO
}

fn is_geojson(value: &serde_json::Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    match object.get("type").and_then(serde_json::Value::as_str) {
        Some("Feature") => object.contains_key("geometry") && object.contains_key("properties"),
        Some("FeatureCollection") => object.get("features").is_some_and(serde_json::Value::is_array),
        Some("GeometryCollection") => object.get("geometries").is_some_and(serde_json::Value::is_array),
        Some("Point" | "MultiPoint" | "LineString" | "MultiLineString" | "Polygon" | "MultiPolygon") => {
            object.get("coordinates").is_some_and(serde_json::Value::is_array)
        }
        _ => false,
    }
}

/// Detect Office Open XML format from ZIP content by scanning for marker files.
///
/// Office Open XML formats (DOCX, XLSX, PPTX) are ZIP archives containing specific
/// XML files that identify the format:
/// - DOCX: contains `word/document.xml`
/// - XLSX: contains `xl/workbook.xml`
/// - PPTX: contains `ppt/presentation.xml`
///
/// Apple iWork formats (2013+) also use ZIP with IWA files:
/// - Pages: contains `Index/Document.iwa`
/// - Numbers: contains `Index/CalculationEngine.iwa`
/// - Keynote: contains `Index/Presentation.iwa`
///
/// This function scans the ZIP's local file headers without fully parsing the archive,
/// making it efficient for MIME type detection.
fn detect_office_format_from_zip(content: &[u8], _package_inspection: PackageInspection) -> Option<&'static str> {
    const DOCX_MARKER: &[u8] = b"word/document.xml";
    const XLSX_MARKER: &[u8] = b"xl/workbook.xml";
    const PPTX_MARKER: &[u8] = b"ppt/presentation.xml";
    const PAGES_MARKER: &[u8] = b"Index/Document.iwa";
    const NUMBERS_MARKER: &[u8] = b"Index/CalculationEngine.iwa";
    const KEYNOTE_MARKER: &[u8] = b"Index/Presentation.iwa";
    const KEYNOTE_SLIDE_MARKERS: &[&[u8]] = &[b"Index/Slide-", b"Index/Slide_"];

    #[cfg(feature = "hwpx")]
    const HWPX_MARKER: &[u8] = b"Contents/content.hpf";
    #[cfg(any(feature = "office", feature = "hwpx", feature = "iwork", feature = "archives"))]
    if _package_inspection == PackageInspection::FullArchive {
        return detect_zip_package(std::io::Cursor::new(content));
    }

    #[cfg(feature = "hwpx")]
    if contains_subsequence(content, HWPX_MARKER) {
        return Some(HWPX_MIME_TYPE);
    }

    // A Numbers package carries `Index/Document.iwa` as well, so the
    // discriminating parts are tested before it.
    if contains_subsequence(content, NUMBERS_MARKER) {
        return Some(IWORK_NUMBERS_MIME_TYPE);
    }
    // ~keep: Minimal Keynote packages may contain slide archives without a Presentation.iwa index.
    if contains_subsequence(content, KEYNOTE_MARKER)
        || KEYNOTE_SLIDE_MARKERS
            .iter()
            .any(|marker| contains_subsequence(content, marker))
    {
        return Some(IWORK_KEYNOTE_MIME_TYPE);
    }
    if contains_subsequence(content, PAGES_MARKER) {
        return Some(IWORK_PAGES_MIME_TYPE);
    }

    if contains_subsequence(content, DOCX_MARKER) {
        return Some(DOCX_MIME_TYPE);
    }
    if contains_subsequence(content, XLSX_MARKER) {
        return Some(EXCEL_MIME_TYPE);
    }
    if contains_subsequence(content, PPTX_MARKER) {
        return Some(POWER_POINT_MIME_TYPE);
    }
    None
}

/// Read the `mimetype` entry a ZIP-based document package declares.
///
/// OpenDocument and HWPX both store their own media type in an uncompressed
/// `mimetype` entry, which is the format's authoritative identifier. An HWPX
/// package that carries no `Contents/content.hpf` is still identified here.
/// A package with two `mimetype` entries is rejected rather than guessed at.
/// Identify a ZIP-based office format from the names in the archive directory.
///
/// `detect_office_format_from_zip` searches raw bytes, so it only sees the part
/// of the archive it was given. A caller that reads a fixed-size header misses
/// every part written after it.
#[cfg(any(feature = "office", feature = "hwpx", feature = "iwork", feature = "archives"))]
fn detect_office_format_from_archive<R: Read + Seek>(archive: &mut zip::ZipArchive<R>) -> Option<&'static str> {
    let has = |archive: &mut zip::ZipArchive<R>, name: &str| archive.index_for_name(name).is_some();
    #[cfg(feature = "hwpx")]
    if has(archive, "Contents/content.hpf") {
        return Some(HWPX_MIME_TYPE);
    }
    if has(archive, "word/document.xml") {
        return Some(DOCX_MIME_TYPE);
    }
    if has(archive, "xl/workbook.xml") {
        return Some(EXCEL_MIME_TYPE);
    }
    if has(archive, "ppt/presentation.xml") {
        return Some(POWER_POINT_MIME_TYPE);
    }
    // A Numbers package also carries `Index/Document.iwa`, so the discriminating
    // parts are tested first. Otherwise a spreadsheet is read as a Pages
    // document and yields no sheets at all.
    if has(archive, "Index/CalculationEngine.iwa") {
        return Some(IWORK_NUMBERS_MIME_TYPE);
    }
    if has(archive, "Index/Presentation.iwa")
        || archive
            .file_names()
            .any(|name| name.is_ok_and(|name| name.starts_with("Index/Slide-") || name.starts_with("Index/Slide_")))
    {
        return Some(IWORK_KEYNOTE_MIME_TYPE);
    }
    if has(archive, "Index/Document.iwa") {
        return Some(IWORK_PAGES_MIME_TYPE);
    }
    None
}

/// Identify a legacy MS-CFB compound document (.doc/.xls/.ppt) from its root
/// storage CLSID.
///
/// Mirrors the CLSID table `infer`'s `ole2()` matcher uses, but reads through
/// a `Read + Seek` source instead of a fixed byte slice. A compound file
/// cannot be typed from a truncated prefix: locating the root directory entry
/// means following the FAT sector chain, and a chain built from a partial
/// read references sectors the buffer does not contain (#1590).
/// `cfb::CompoundFile::open` seeks and reads only the header, FAT, and
/// directory sectors it needs, so this does not load the file into memory —
/// the same shape `detect_zip_package` uses for a ZIP-based package.
/// `limits.max_archive_size` still bounds the file this is attempted
/// against, mirroring the bound `zip_central_directory_within_limits` applies
/// before it opens a ZIP central directory.
#[cfg(any(feature = "office", feature = "hwp", feature = "email"))]
fn detect_ole2_package<R: Read + Seek>(mut reader: R, limits: &SecurityLimits) -> Option<String> {
    let length = reader.seek(SeekFrom::End(0)).ok()?;
    if length > limits.max_archive_size as u64 {
        return None;
    }
    reader.seek(SeekFrom::Start(0)).ok()?;
    let compound_file = cfb::CompoundFile::open(reader).ok()?;
    let mime_type = match compound_file.root_entry().clsid().to_string().as_str() {
        "00020810-0000-0000-c000-000000000046" | "00020820-0000-0000-c000-000000000046" => LEGACY_EXCEL_MIME_TYPE,
        "00020906-0000-0000-c000-000000000046" => LEGACY_WORD_MIME_TYPE,
        "64818d10-4f9b-11cf-86ea-00aa00b929e8" => LEGACY_POWERPOINT_MIME_TYPE,
        _ => return None,
    };
    Some(mime_type.to_string())
}

#[cfg(any(feature = "office", feature = "hwpx", feature = "iwork", feature = "archives"))]
fn detect_zip_package<R: Read + Seek>(mut reader: R) -> Option<&'static str> {
    let limits = SecurityLimits::default();
    if !zip_central_directory_within_limits(&mut reader, &limits) {
        return None;
    }
    reader.seek(SeekFrom::Start(0)).ok()?;
    let mut archive = zip::ZipArchive::new(reader).ok()?;

    let declared = detect_zip_mimetype_entry(&mut archive);
    #[cfg(feature = "office")]
    let declared = declared.or_else(|| detect_ooxml_content_type(&mut archive));
    declared.or_else(|| detect_office_format_from_archive(&mut archive))
}

#[cfg(feature = "office")]
fn detect_ooxml_content_type<R: Read + Seek>(archive: &mut zip::ZipArchive<R>) -> Option<&'static str> {
    const CONTENT_TYPES_PATH: &str = "[Content_Types].xml";
    const MAX_CONTENT_TYPES_LENGTH: u64 = 64 * 1024;

    let mut content_types_index = None;
    for index in 0..archive.len() {
        let entry = archive.by_index(index).ok()?;
        if entry.name().ok()?.as_ref() == CONTENT_TYPES_PATH && content_types_index.replace(index).is_some() {
            return None;
        }
    }

    let file = archive.by_index(content_types_index?).ok()?;
    if file.size() > MAX_CONTENT_TYPES_LENGTH {
        return None;
    }
    let mut xml = String::with_capacity(file.size() as usize);
    file.take(MAX_CONTENT_TYPES_LENGTH + 1).read_to_string(&mut xml).ok()?;
    let document = roxmltree::Document::parse(&xml).ok()?;
    let mut detected = None;
    for node in document.descendants().filter(|node| node.has_tag_name("Override")) {
        let part_name = node.attribute("PartName")?;
        let content_type = node.attribute("ContentType")?;
        if let Some(mime_type) = ooxml_package_mime(part_name, content_type)
            && detected.replace(mime_type).is_some()
        {
            return None;
        }
    }
    detected
}

#[cfg(feature = "office")]
fn ooxml_package_mime(part_name: &str, content_type: &str) -> Option<&'static str> {
    match part_name {
        "/word/document.xml" => wordprocessing_package_mime(content_type),
        "/ppt/presentation.xml" => presentation_package_mime(content_type),
        "/xl/workbook.xml" | "/xl/workbook.bin" => spreadsheet_package_mime(content_type),
        _ => None,
    }
}

#[cfg(feature = "office")]
fn wordprocessing_package_mime(content_type: &str) -> Option<&'static str> {
    match content_type {
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml" => Some(DOCX_MIME_TYPE),
        "application/vnd.openxmlformats-officedocument.wordprocessingml.template.main+xml" => {
            Some("application/vnd.openxmlformats-officedocument.wordprocessingml.template")
        }
        "application/vnd.ms-word.document.macroEnabled.main+xml" => {
            Some("application/vnd.ms-word.document.macroEnabled.12")
        }
        "application/vnd.ms-word.template.macroEnabledTemplate.main+xml" => {
            Some("application/vnd.ms-word.template.macroEnabled.12")
        }
        _ => None,
    }
}

#[cfg(feature = "office")]
fn presentation_package_mime(content_type: &str) -> Option<&'static str> {
    match content_type {
        "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml" => {
            Some(POWER_POINT_MIME_TYPE)
        }
        "application/vnd.openxmlformats-officedocument.presentationml.slideshow.main+xml" => {
            Some("application/vnd.openxmlformats-officedocument.presentationml.slideshow")
        }
        "application/vnd.openxmlformats-officedocument.presentationml.template.main+xml" => {
            Some("application/vnd.openxmlformats-officedocument.presentationml.template")
        }
        "application/vnd.ms-powerpoint.presentation.macroEnabled.main+xml" => {
            Some("application/vnd.ms-powerpoint.presentation.macroEnabled.12")
        }
        "application/vnd.ms-powerpoint.template.macroEnabled.main+xml" => {
            Some("application/vnd.ms-powerpoint.template.macroEnabled.12")
        }
        _ => None,
    }
}

#[cfg(feature = "office")]
fn spreadsheet_package_mime(content_type: &str) -> Option<&'static str> {
    match content_type {
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml" => Some(EXCEL_MIME_TYPE),
        "application/vnd.openxmlformats-officedocument.spreadsheetml.template.main+xml" => {
            Some("application/vnd.openxmlformats-officedocument.spreadsheetml.template")
        }
        "application/vnd.ms-excel.sheet.macroEnabled.main+xml" => {
            Some("application/vnd.ms-excel.sheet.macroEnabled.12")
        }
        "application/vnd.ms-excel.template.macroEnabled.main+xml" => {
            Some("application/vnd.ms-excel.template.macroEnabled.12")
        }
        "application/vnd.ms-excel.addin.macroEnabled.main+xml" => {
            Some("application/vnd.ms-excel.addin.macroEnabled.12")
        }
        "application/vnd.ms-excel.sheet.binary.macroEnabled.main" => {
            Some("application/vnd.ms-excel.sheet.binary.macroEnabled.12")
        }
        _ => None,
    }
}

/// Read the `mimetype` entry a ZIP-based document package declares.
#[cfg(any(feature = "office", feature = "hwpx", feature = "iwork", feature = "archives"))]
fn detect_zip_mimetype_entry<R: Read + Seek>(archive: &mut zip::ZipArchive<R>) -> Option<&'static str> {
    /// The value an HWPX package stores in its `mimetype` entry.
    #[cfg(feature = "hwpx")]
    const HWPX_PACKAGE_MIMETYPE: &[u8] = b"application/hwp+zip";
    const MAX_MIMETYPE_LENGTH: u64 = ODP_MIME_TYPE.len() as u64;

    let mut mimetype_index = None;
    for index in 0..archive.len() {
        if archive.by_index(index).ok()?.name().ok()?.as_ref() == "mimetype" && mimetype_index.replace(index).is_some()
        {
            return None;
        }
    }

    #[cfg(feature = "hwpx")]
    let has_hwpx_manifest = archive.index_for_name("Contents/content.hpf").is_some();

    let mimetype = archive.by_index(mimetype_index?).ok()?;
    if mimetype.size() > MAX_MIMETYPE_LENGTH {
        return None;
    }

    let mut value = Vec::with_capacity(mimetype.size() as usize);
    mimetype.take(MAX_MIMETYPE_LENGTH + 1).read_to_end(&mut value).ok()?;
    match value.as_slice() {
        value if value == ODT_MIME_TYPE.as_bytes() => Some(ODT_MIME_TYPE),
        value if value == ODP_MIME_TYPE.as_bytes() => Some(ODP_MIME_TYPE),
        value if value == ODS_MIME_TYPE.as_bytes() => Some(ODS_MIME_TYPE),
        // The HWPX reader needs the manifest, so a package without one keeps its
        // ZIP routing and its members stay readable. The entry is looked up in
        // the archive directory, because Hangul writes it near the end of the
        // file, past any header a caller may have truncated to.
        #[cfg(feature = "hwpx")]
        value if value == HWPX_PACKAGE_MIMETYPE && has_hwpx_manifest => Some(HWPX_MIME_TYPE),
        _ => None,
    }
}

#[cfg(any(feature = "office", feature = "hwpx", feature = "iwork", feature = "archives"))]
struct ZipCentralDirectory {
    offset: u64,
    size: usize,
    entries: u16,
}

#[cfg(any(feature = "office", feature = "hwpx", feature = "iwork", feature = "archives"))]
fn read_zip_central_directory<R: Read + Seek>(reader: &mut R, limits: &SecurityLimits) -> Option<ZipCentralDirectory> {
    const EOCD_SIGNATURE: &[u8; 4] = b"PK\x05\x06";
    const EOCD_MIN_LENGTH: u64 = 22;
    const MAX_ZIP_COMMENT_LENGTH: u64 = u16::MAX as u64;

    let archive_length = reader.seek(SeekFrom::End(0)).ok()?;
    if archive_length < EOCD_MIN_LENGTH || archive_length > limits.max_archive_size as u64 {
        return None;
    }

    let tail_length = archive_length.min(EOCD_MIN_LENGTH + MAX_ZIP_COMMENT_LENGTH);
    reader.seek(SeekFrom::End(-(tail_length as i64))).ok()?;
    let mut tail = vec![0; tail_length as usize];
    reader.read_exact(&mut tail).ok()?;

    let eocd_offset = tail
        .windows(EOCD_SIGNATURE.len())
        .rposition(|window| window == EOCD_SIGNATURE)?;
    let eocd = &tail[eocd_offset..];
    if eocd.len() < EOCD_MIN_LENGTH as usize {
        return None;
    }

    let disk_number = u16::from_le_bytes([eocd[4], eocd[5]]);
    let central_directory_disk = u16::from_le_bytes([eocd[6], eocd[7]]);
    let entries_on_disk = u16::from_le_bytes([eocd[8], eocd[9]]);
    let entries = u16::from_le_bytes([eocd[10], eocd[11]]);
    let size = u32::from_le_bytes([eocd[12], eocd[13], eocd[14], eocd[15]]) as usize;
    let offset = u32::from_le_bytes([eocd[16], eocd[17], eocd[18], eocd[19]]) as u64;
    let comment_length = u16::from_le_bytes([eocd[20], eocd[21]]) as usize;
    let is_valid = eocd.len() == EOCD_MIN_LENGTH as usize + comment_length
        && disk_number == 0
        && central_directory_disk == 0
        && entries_on_disk == entries
        && entries != u16::MAX
        && entries as usize <= limits.max_files_in_archive
        && size <= limits.max_content_size
        && offset.checked_add(size as u64).is_some_and(|end| end <= archive_length);
    is_valid.then_some(ZipCentralDirectory { offset, size, entries })
}

#[cfg(any(feature = "office", feature = "hwpx", feature = "iwork", feature = "archives"))]
fn read_central_directory_entry<R: Read + Seek>(reader: &mut R) -> Option<(Vec<u8>, usize)> {
    const HEADER_SIGNATURE: &[u8; 4] = b"PK\x01\x02";
    const HEADER_LENGTH: usize = 46;

    let mut header = [0; HEADER_LENGTH];
    reader.read_exact(&mut header).ok()?;
    (&header[..4] == HEADER_SIGNATURE).then_some(())?;

    let name_length = u16::from_le_bytes([header[28], header[29]]) as usize;
    let extra_length = u16::from_le_bytes([header[30], header[31]]) as usize;
    let comment_length = u16::from_le_bytes([header[32], header[33]]) as usize;
    let entry_length = HEADER_LENGTH
        .checked_add(name_length)?
        .checked_add(extra_length)?
        .checked_add(comment_length)?;

    let mut name = vec![0; name_length];
    reader.read_exact(&mut name).ok()?;
    reader
        .seek(SeekFrom::Current((extra_length + comment_length) as i64))
        .ok()?;
    Some((name, entry_length))
}

#[cfg(any(feature = "office", feature = "hwpx", feature = "iwork", feature = "archives"))]
fn central_directory_has_unique_mimetype<R: Read + Seek>(reader: &mut R, directory: &ZipCentralDirectory) -> bool {
    if reader.seek(SeekFrom::Start(directory.offset)).is_err() {
        return false;
    }

    let mut bytes_read = 0usize;
    let mut mimetype_entries = 0usize;
    for _ in 0..directory.entries {
        let Some((name, entry_length)) = read_central_directory_entry(reader) else {
            return false;
        };
        let Some(next_bytes_read) = bytes_read.checked_add(entry_length) else {
            return false;
        };
        if next_bytes_read > directory.size {
            return false;
        }
        if name == b"mimetype" {
            mimetype_entries += 1;
            if mimetype_entries > 1 {
                return false;
            }
        }
        bytes_read = next_bytes_read;
    }

    true
}

#[cfg(any(feature = "office", feature = "hwpx", feature = "iwork", feature = "archives"))]
fn zip_central_directory_within_limits<R: Read + Seek>(reader: &mut R, limits: &SecurityLimits) -> bool {
    read_zip_central_directory(reader, limits)
        .is_some_and(|directory| central_directory_has_unique_mimetype(reader, &directory))
}

/// Check if `haystack` contains `needle` as a subsequence.
#[inline]
fn contains_subsequence(haystack: &[u8], needle: &[u8]) -> bool {
    memchr::memmem::find(haystack, needle).is_some()
}

/// Get file extensions for a given MIME type.
///
/// Returns all known file extensions that map to the specified MIME type.
///
/// # Arguments
///
/// * `mime_type` - The MIME type to look up
///
/// # Returns
///
/// A vector of file extensions (without leading dot) for the MIME type.
///
/// # Example
///
/// ```
/// use xberg::core::mime::get_extensions_for_mime;
///
/// let extensions = get_extensions_for_mime("application/pdf").unwrap();
/// assert_eq!(extensions, vec!["pdf"]);
///
/// let doc_extensions = get_extensions_for_mime("application/vnd.openxmlformats-officedocument.wordprocessingml.document").unwrap();
/// assert!(doc_extensions.contains(&"docx".to_string()));
/// ```
pub fn get_extensions_for_mime(mime_type: &str) -> Result<Vec<String>> {
    let mut extensions = Vec::new();

    for entry in FORMATS {
        if entry.mime_type == mime_type || entry.aliases.contains(&mime_type) {
            extensions.extend(entry.extensions.iter().map(|extension| (*extension).to_string()));
        }
    }

    if !extensions.is_empty() {
        extensions.sort();
        extensions.dedup();
        return Ok(extensions);
    }

    let guessed = mime_guess::get_mime_extensions_str(mime_type);
    if let Some(exts) = guessed {
        return Ok(exts.iter().map(|s| s.to_string()).collect());
    }

    Err(XbergError::UnsupportedFormat(format!(
        "No known extensions for MIME type: {}",
        mime_type
    )))
}

/// List all supported document formats.
///
/// Returns every file extension Xberg recognizes together with its
/// corresponding MIME type, derived from the central format registry.
/// Formats that have no registered file extension (such as source code,
/// which is detected dynamically) are not included.
///
/// The static `EXT_TO_MIME` table lists every format the *codebase* knows how
/// to describe, regardless of which Cargo features were compiled in. Advertising
/// that table directly would claim support for extractors that may not exist in
/// this build (see GH#1387). To keep the advertised catalogue honest, the table
/// is intersected with the document extractor registry: an extension is only
/// included if some registered extractor actually claims its MIME type in this
/// build. This can never drift from reality and automatically covers
/// third-party extractors registered at runtime.
///
/// The list is sorted alphabetically by file extension.
///
/// # Returns
///
/// A vector of [`SupportedFormat`] entries sorted by extension, limited to
/// formats with a registered extractor in this build.
///
/// # Example
///
/// ```
/// use xberg::core::mime::list_supported_formats;
///
/// let formats = list_supported_formats();
/// assert!(!formats.is_empty());
/// ```
pub fn list_supported_formats() -> Vec<SupportedFormat> {
    if let Err(error) = crate::extractors::ensure_initialized() {
        tracing::warn!(%error, "failed to initialize document extractor registry before listing formats");
    }

    let registry = crate::plugins::registry::get_document_extractor_registry();
    let registry_guard = registry.read();

    let mut formats: Vec<SupportedFormat> = EXT_TO_MIME
        .iter()
        .filter(|(_ext, mime)| registry_guard.get(mime).is_ok())
        .map(|(ext, mime)| SupportedFormat {
            extension: ext.to_string(),
            mime_type: mime.to_string(),
        })
        .collect();
    formats.sort_by(|a, b| a.extension.cmp(&b.extension));
    formats
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::{Cursor, Write};
    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;

    fn build_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (name, content) in entries {
            archive.start_file(*name, options).unwrap();
            archive.write_all(content).unwrap();
        }
        archive.finish().unwrap().into_inner()
    }

    #[cfg(all(feature = "xml", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    async fn assert_specialized_xml_routes_through_real_extractor(
        extension: &str,
        content: &str,
        expected_mime: &str,
        expected_text: &str,
    ) {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join(format!("routing.{extension}"));
        std::fs::write(&file_path, content).unwrap();

        let config = crate::core::config::ExtractionConfig {
            use_cache: false,
            ..Default::default()
        };
        let result = crate::core::extractor::extract_file(&file_path, None, &config)
            .await
            .unwrap();

        assert_eq!(result.mime_type, expected_mime);
        assert!(
            result.content.contains(expected_text),
            "specialized extractor lost expected text: {}",
            result.content
        );
        assert!(
            !result.content.contains("<article") && !result.content.contains("<FictionBook"),
            "generic XML markup leaked into extracted content: {}",
            result.content
        );
    }

    #[cfg(all(feature = "office", feature = "xml", feature = "tokio-runtime"))]
    #[tokio::test]
    async fn should_route_fb2_extension_to_fictionbook_extractor() {
        let content = r#"<?xml version="1.0" encoding="utf-8"?>
<FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0">
  <description><title-info><book-title>Routing Test</book-title></title-info></description>
  <body><section><title><p>First Chapter</p></title><p>FictionBook semantic text.</p></section></body>
</FictionBook>"#;

        assert_specialized_xml_routes_through_real_extractor(
            "fb2",
            content,
            "application/x-fictionbook+xml",
            "FictionBook semantic text.",
        )
        .await;
    }

    #[cfg(feature = "hwpx")]
    #[test]
    fn should_detect_hwpx_without_a_content_hpf_from_its_mimetype_entry() {
        // Real Hangul packages carry `version.xml` and `Contents/section0.xml`
        // but no `Contents/content.hpf`, so only the `mimetype` entry names them.
        let mut buffer = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buffer));
            let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
            writer.start_file("mimetype", stored).unwrap();
            std::io::Write::write_all(&mut writer, b"application/hwp+zip").unwrap();
            writer
                .start_file("Contents/section0.xml", zip::write::SimpleFileOptions::default())
                .unwrap();
            std::io::Write::write_all(&mut writer, b"<hs:sec/>").unwrap();
            // Written last, as Hangul does, so a truncated header cannot see it.
            writer
                .start_file("Contents/content.hpf", zip::write::SimpleFileOptions::default())
                .unwrap();
            std::io::Write::write_all(&mut writer, b"<opf:package/>").unwrap();
            writer.finish().unwrap();
        }

        assert_eq!(
            detect_mime_type_from_bytes(&buffer).unwrap(),
            "application/haansofthwpx"
        );
    }

    #[test]
    fn should_detect_docbook_by_namespace_when_extension_is_plain_xml() {
        // Real DocBook ships as `.xml`, so only the namespace identifies it.
        let content = br#"<!DOCTYPE refentry [ <!ENTITY % mathent SYSTEM "math.ent"> %mathent; ]>
<refentry xmlns="http://docbook.org/ns/docbook" version="5.0" xml:id="exp">
  <refsect1><para>Text.</para></refsect1>
</refentry>"#;

        assert_eq!(detect_mime_type_from_bytes(content).unwrap(), "application/docbook+xml");
    }

    #[cfg(feature = "office")]
    #[test]
    fn should_detect_a_deck_whose_main_part_is_written_late_in_the_archive() {
        // A real 27-slide deck names `ppt/presentation.xml` 107 KB in, so a
        // detector that reads a fixed-size header sees a plain ZIP and the
        // presentation extracts as a list of archive members.
        let mut buffer = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buffer));
            let options = zip::write::SimpleFileOptions::default();
            for index in 0..12 {
                writer
                    .start_file(format!("ppt/media/image{index}.bin"), options)
                    .unwrap();
                std::io::Write::write_all(&mut writer, &vec![index as u8; 8192]).unwrap();
            }
            writer.start_file("ppt/presentation.xml", options).unwrap();
            std::io::Write::write_all(&mut writer, b"<p:presentation/>").unwrap();
            writer.finish().unwrap();
        }
        // Name it `.zip` so the extension does not answer the question. That is
        // the path a real deck takes: the header search fails, and only the
        // archive directory can identify it.
        let path = std::env::temp_dir().join("xberg_late_part_test.zip");
        std::fs::write(&path, &buffer).unwrap();

        let detected = detect_or_validate(path.to_str(), None).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(
            detected, "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            "the deck is identified by its parts, not by its name"
        );
    }

    #[cfg(feature = "hwpx")]
    #[test]
    fn should_keep_zip_routing_for_an_hwpx_package_without_its_manifest() {
        // `unhwp` needs `Contents/content.hpf`. Without it the HWPX extractor
        // fails outright, so the package stays on the ZIP route and its members
        // remain readable.
        let mut buffer = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buffer));
            let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
            writer.start_file("mimetype", stored).unwrap();
            std::io::Write::write_all(&mut writer, b"application/hwp+zip").unwrap();
            writer
                .start_file("Contents/section0.xml", zip::write::SimpleFileOptions::default())
                .unwrap();
            std::io::Write::write_all(&mut writer, b"<hs:sec/>").unwrap();
            writer.finish().unwrap();
        }

        assert_eq!(detect_mime_type_from_bytes(&buffer).unwrap(), "application/zip");
    }

    #[test]
    fn should_keep_generic_xml_for_a_stylesheet_that_only_names_docbook() {
        // A DocBook XSL customization layer binds the namespace on a foreign
        // root. It is not a DocBook document, and the DocBook extractor drops
        // every element it does not know.
        let content = br#"<?xml version="1.0"?>
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:d="http://docbook.org/ns/docbook" version="1.0">
  <xsl:template match="d:para"><p><xsl:apply-templates/></p></xsl:template>
</xsl:stylesheet>"#;

        assert_eq!(detect_mime_type_from_bytes(content).unwrap(), "text/xml");
    }

    #[test]
    fn should_keep_generic_xml_for_a_catalog_that_has_a_doctype_and_names_the_docbook_dtd() {
        // The declaration ends at its own `>`. A `]` later in the body must not
        // stretch it over the public identifier that follows.
        let content = br#"<?xml version="1.0"?>
<!DOCTYPE catalog PUBLIC "-//OASIS//DTD Entity Resolution XML Catalog V1.0//EN" "catalog.dtd">
<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
  <public publicId="-//OASIS//DTD DocBook XML V4.5//EN" uri="docbookx.dtd"/>
  <note>index a[0] and b[1]</note>
</catalog>"#;

        assert_eq!(detect_mime_type_from_bytes(content).unwrap(), "text/xml");
    }

    #[test]
    fn should_detect_docbook_when_the_body_holds_a_bracket() {
        // A `]` in the body must not make the root element unreachable.
        let content = br#"<?xml version="1.0"?>
<!DOCTYPE book SYSTEM "docbook.dtd">
<book xmlns="http://docbook.org/ns/docbook" version="5.0">
  <chapter><para>The value a[0] is first.</para></chapter>
</book>"#;

        assert_eq!(detect_mime_type_from_bytes(content).unwrap(), "application/docbook+xml");
    }

    #[test]
    fn should_keep_generic_xml_for_a_catalog_that_lists_the_docbook_dtd() {
        let content = br#"<?xml version="1.0"?>
<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
  <public publicId="-//OASIS//DTD DocBook XML V4.5//EN" uri="docbookx.dtd"/>
</catalog>"#;

        assert_eq!(detect_mime_type_from_bytes(content).unwrap(), "text/xml");
    }

    #[test]
    fn should_detect_docbook_when_the_namespace_is_bound_to_a_prefix() {
        let content = br#"<?xml version="1.0"?>
<db:book xmlns:db="http://docbook.org/ns/docbook" version="5.0">
  <db:chapter><db:para>Text.</db:para></db:chapter>
</db:book>"#;

        assert_eq!(detect_mime_type_from_bytes(content).unwrap(), "application/docbook+xml");
    }

    #[test]
    fn should_detect_docbook_by_dtd_public_identifier() {
        let content = br#"<?xml version="1.0"?>
<!DOCTYPE article PUBLIC "-//OASIS//DTD DocBook XML V4.4//EN" "http://www.oasis-open.org/docbook/xml/4.4/docbookx.dtd">
<article><para>Text.</para></article>"#;

        assert_eq!(detect_mime_type_from_bytes(content).unwrap(), "application/docbook+xml");
    }

    #[test]
    fn should_detect_jats_by_dtd_public_identifier() {
        let content = br#"<?xml version="1.0"?>
<!DOCTYPE article PUBLIC "-//NLM//DTD JATS (Z39.96) Journal Archiving DTD v1.0 20120330//EN" "JATS-archivearticle1.dtd">
<article><body><p>Text.</p></body></article>"#;

        assert_eq!(detect_mime_type_from_bytes(content).unwrap(), "application/x-jats+xml");
    }

    #[test]
    fn should_keep_generic_xml_without_a_vocabulary_declaration() {
        let content = br#"<?xml version="1.0"?><catalog><item>Text.</item></catalog>"#;

        // `text/xml` is the registered alias `infer` returns for a declaration.
        assert_eq!(detect_mime_type_from_bytes(content).unwrap(), "text/xml");
    }

    #[cfg(all(feature = "office", feature = "xml", feature = "tokio-runtime"))]
    #[tokio::test]
    async fn should_route_docbook_extensions_to_docbook_extractor() {
        let content = r#"<?xml version="1.0" encoding="utf-8"?>
<article xmlns="http://docbook.org/ns/docbook" version="5.0">
  <title>Routing Test</title><para>DocBook semantic text.</para>
</article>"#;

        for extension in ["docbook", "dbk"] {
            assert_specialized_xml_routes_through_real_extractor(
                extension,
                content,
                "application/docbook+xml",
                "DocBook semantic text.",
            )
            .await;
        }
    }

    #[cfg(all(feature = "xml", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    #[tokio::test]
    async fn should_route_nxml_extension_to_jats_extractor() {
        let content = r#"<?xml version="1.0" encoding="utf-8"?>
<article>
<front><article-meta><title-group><article-title>Routing Test</article-title></title-group></article-meta></front>
<body><sec><title>Results</title><p>NXML semantic text.</p></sec></body></article>"#;

        assert_specialized_xml_routes_through_real_extractor(
            "nxml",
            content,
            "application/x-jats+xml",
            "NXML semantic text.",
        )
        .await;
    }

    #[cfg(all(feature = "xml", feature = "tokio-runtime", not(target_arch = "wasm32")))]
    #[tokio::test]
    async fn should_route_kml_through_the_xml_extractor() {
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
<kml xmlns="http://www.opengis.net/kml/2.2">
  <Placemark><name>Berlin</name></Placemark>
</kml>"#;
        let directory = tempdir().unwrap();
        let path = directory.path().join("placemark.kml");
        std::fs::write(&path, content).unwrap();
        let config = crate::core::config::ExtractionConfig {
            use_cache: false,
            ..Default::default()
        };

        let result = crate::core::extractor::extract_file(&path, None, &config)
            .await
            .unwrap();

        assert_eq!(result.mime_type, "application/vnd.google-earth.kml+xml");
        assert_eq!(result.content, "kml\n  Placemark\n    name\n    Berlin");
    }

    #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
    #[tokio::test]
    async fn should_route_geojson_through_the_structured_extractor() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("point.geojson");
        std::fs::write(&path, br#"{"type":"Point","coordinates":[13.4,52.5]}"#).unwrap();
        let config = crate::core::config::ExtractionConfig {
            use_cache: false,
            geojson: Some(crate::core::config::GeoJsonExtractionConfig {
                include_full_coordinates: true,
            }),
            ..Default::default()
        };

        let result = crate::core::extractor::extract_file(&path, None, &config)
            .await
            .unwrap();

        assert_eq!(result.mime_type, "application/geo+json");
        assert_eq!(result.content, "type: Point\n\ncoordinates\n13.4\n52.5");
    }

    #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
    #[tokio::test]
    async fn should_route_benchmark_text_extensions_to_plain_text_extractor() {
        let test_cases = [
            ("adoc", "text/asciidoc", "AsciiDoc short-extension routing text."),
            ("asciidoc", "text/asciidoc", "AsciiDoc routing text."),
            ("vtt", "text/vtt", "WebVTT routing text."),
        ];

        for (extension, expected_mime, expected_text) in test_cases {
            let dir = tempdir().unwrap();
            let file_path = dir.path().join(format!("routing.{extension}"));
            std::fs::write(&file_path, expected_text).unwrap();

            let config = crate::core::config::ExtractionConfig {
                use_cache: false,
                ..Default::default()
            };
            let result = crate::core::extractor::extract_file(&file_path, None, &config)
                .await
                .unwrap();

            assert_eq!(result.mime_type, expected_mime);
            assert!(result.content.contains(expected_text));
        }
    }

    #[test]
    fn should_resolve_registered_mime_alias_to_extensions() {
        assert_eq!(
            get_extensions_for_mime("text/x-asciidoc").unwrap(),
            vec!["adoc".to_string(), "asciidoc".to_string()]
        );
    }

    #[test]
    fn test_detect_mime_type_pdf() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.pdf");
        File::create(&file_path).unwrap();

        let mime = detect_mime_type(&file_path, true).unwrap();
        assert_eq!(mime, "application/pdf");
    }

    #[test]
    fn test_detect_mime_type_images() {
        let dir = tempdir().unwrap();

        let test_cases = vec![
            ("test.png", "image/png"),
            ("test.jpg", "image/jpeg"),
            ("test.jpeg", "image/jpeg"),
            ("test.gif", "image/gif"),
            ("test.bmp", "image/bmp"),
            ("test.webp", "image/webp"),
            ("test.tiff", "image/tiff"),
        ];

        for (filename, expected_mime) in test_cases {
            let file_path = dir.path().join(filename);
            File::create(&file_path).unwrap();
            let mime = detect_mime_type(&file_path, true).unwrap();
            assert_eq!(mime, expected_mime, "Failed for {}", filename);
        }
    }

    #[test]
    fn test_detect_mime_type_office() {
        let dir = tempdir().unwrap();

        let test_cases = vec![
            ("test.xlsx", EXCEL_MIME_TYPE),
            ("test.xls", "application/vnd.ms-excel"),
            ("test.pptx", POWER_POINT_MIME_TYPE),
            (
                "test.ppsx",
                "application/vnd.openxmlformats-officedocument.presentationml.slideshow",
            ),
            (
                "test.pptm",
                "application/vnd.ms-powerpoint.presentation.macroEnabled.12",
            ),
            ("test.ppt", LEGACY_POWERPOINT_MIME_TYPE),
            ("test.docx", DOCX_MIME_TYPE),
            ("test.doc", LEGACY_WORD_MIME_TYPE),
        ];

        for (filename, expected_mime) in test_cases {
            let file_path = dir.path().join(filename);
            File::create(&file_path).unwrap();
            let mime = detect_mime_type(&file_path, true).unwrap();
            assert_eq!(mime, expected_mime, "Failed for {}", filename);
        }
    }

    #[test]
    fn test_detect_mime_type_data_formats() {
        let dir = tempdir().unwrap();

        let test_cases = vec![
            ("test.json", JSON_MIME_TYPE),
            ("test.yaml", "application/yaml"),
            ("test.toml", "application/toml"),
            ("test.xml", XML_MIME_TYPE),
            ("test.csv", "text/csv"),
        ];

        for (filename, expected_mime) in test_cases {
            let file_path = dir.path().join(filename);
            File::create(&file_path).unwrap();
            let mime = detect_mime_type(&file_path, true).unwrap();
            assert_eq!(mime, expected_mime, "Failed for {}", filename);
        }
    }

    #[test]
    fn test_detect_mime_type_text_formats() {
        let dir = tempdir().unwrap();

        let test_cases = vec![
            ("test.txt", PLAIN_TEXT_MIME_TYPE),
            ("test.md", "text/markdown"),
            ("test.qmd", "text/x-quarto"),
            ("test.Rmd", "text/x-r-markdown"),
            ("test.rmd", "text/x-r-markdown"),
            ("test.html", HTML_MIME_TYPE),
            ("test.htm", HTML_MIME_TYPE),
        ];

        for (filename, expected_mime) in test_cases {
            let file_path = dir.path().join(filename);
            File::create(&file_path).unwrap();
            let mime = detect_mime_type(&file_path, true).unwrap();
            assert_eq!(mime, expected_mime, "Failed for {}", filename);
        }
    }

    #[test]
    fn test_detect_mime_type_email() {
        let dir = tempdir().unwrap();

        let test_cases = vec![
            ("test.eml", "message/rfc822"),
            ("test.msg", "application/vnd.ms-outlook"),
            ("test.pst", PST_MIME_TYPE),
        ];

        for (filename, expected_mime) in test_cases {
            let file_path = dir.path().join(filename);
            File::create(&file_path).unwrap();
            let mime = detect_mime_type(&file_path, true).unwrap();
            assert_eq!(mime, expected_mime, "Failed for {}", filename);
        }
    }

    #[test]
    fn test_validate_mime_type_exact() {
        assert!(validate_mime_type("application/pdf").is_ok());
        assert!(validate_mime_type("text/plain").is_ok());
        assert!(validate_mime_type("text/html").is_ok());
    }

    #[test]
    fn test_validate_mime_type_images() {
        assert!(validate_mime_type("image/jpeg").is_ok());
        assert!(validate_mime_type("image/png").is_ok());
        assert!(validate_mime_type("image/gif").is_ok());
        assert!(validate_mime_type("image/webp").is_ok());
        assert!(validate_mime_type("image/custom-format").is_err());
    }

    #[test]
    fn should_validate_parameterized_mime_by_its_well_formed_essence() {
        assert_eq!(
            validate_mime_type("Application/JSON; Charset=UTF-8").unwrap(),
            "application/json"
        );
        assert_eq!(
            validate_mime_type(" application/geo+json; charset=utf-8 ").unwrap(),
            GEOJSON_MIME_TYPE
        );
    }

    #[test]
    fn should_reject_malformed_mime_syntax_before_registry_lookup() {
        for malformed in [
            "application//json",
            "application/json; charset",
            "application/json, text/plain",
            "application/json; charset=\"unterminated",
        ] {
            assert!(validate_mime_type(malformed).is_err(), "accepted {malformed:?}");
        }
    }

    #[test]
    fn test_validate_mime_type_unsupported() {
        assert!(validate_mime_type("application/unknown").is_err());
    }

    #[test]
    fn test_validate_mime_type_audio_video() {
        assert!(validate_mime_type("audio/mpeg").is_ok());
        assert!(validate_mime_type("audio/mp4").is_ok());
        assert!(validate_mime_type("audio/wav").is_ok());
        assert!(validate_mime_type("audio/webm").is_ok());
        assert!(validate_mime_type("video/mp4").is_ok());
        assert!(validate_mime_type("video/webm").is_ok());
    }

    #[test]
    fn audited_extensions_resolve_to_registered_canonical_mime_types() {
        let expected = [
            ("file.dj", "text/x-djot"),
            ("file.pps", LEGACY_POWERPOINT_MIME_TYPE),
            ("file.xltm", "application/vnd.ms-excel.template.macroEnabled.12"),
            ("file.xla", "application/vnd.ms-excel"),
            ("file.sqlite3", SQLITE_MIME_TYPE),
            ("file.gpkx", GEOPACKAGE_MIME_TYPE),
            ("file.xhtml", "application/xhtml+xml"),
            ("file.xht", "application/xhtml+xml"),
            ("file.heics", "image/heic-sequence"),
            ("file.heifs", "image/heif-sequence"),
            ("file.j2c", "image/j2c"),
            ("file.j2k", "image/j2c"),
            ("file.jpc", "image/j2c"),
            ("file.jpg2", "image/jp2"),
            ("file.hif", "image/heif"),
            ("file.mpeg", "video/mpeg"),
            ("file.mpg", "video/mpeg"),
            ("file.mpe", "video/mpeg"),
            ("file.m1v", "video/mpeg"),
            ("file.m2v", "video/mpeg"),
            ("file.mpg4", "video/mp4"),
            ("file.mp4v", "video/mp4"),
            ("file.m4v", "video/mp4"),
            ("file.typ", "text/vnd.typst"),
        ];

        for (path, mime_type) in expected {
            assert_eq!(detect_mime_type(path, false).unwrap(), mime_type, "failed for {path}");
        }
        let motion_jpeg_2000 = detect_mime_type("file.mj2", false).unwrap();
        assert!(validate_mime_type(&motion_jpeg_2000).is_err());
    }

    #[test]
    fn registered_mime_types_are_canonical_and_compatibility_names_are_aliases() {
        let expected = [
            "application/vnd.dbf",
            "application/yaml",
            "text/prs.fallenstein.rst",
            "text/org",
            "text/vnd.typst",
            "application/vnd.geo+json",
            "application/hwp+zip",
        ];
        for mime_type in expected {
            assert_eq!(validate_mime_type(mime_type).unwrap(), mime_type);
        }
        assert!(validate_mime_type("application/geopackage+vnd.sqlite3").is_err());
    }

    #[test]
    fn test_file_not_exists() {
        let result = detect_mime_type("/nonexistent/file.pdf", true);
        assert!(result.is_err());
    }

    #[test]
    fn test_file_no_extension() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("testfile");
        File::create(&file_path).unwrap();

        let _result = detect_mime_type(&file_path, true);
    }

    #[test]
    fn test_detect_or_validate_with_mime() {
        let result = detect_or_validate(None, Some("application/pdf"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "application/pdf");
    }

    #[test]
    fn test_detect_or_validate_with_path() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.pdf");
        File::create(&file_path).unwrap();

        let result = detect_or_validate(file_path.to_str(), None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "application/pdf");
    }

    #[test]
    fn should_detect_content_when_extension_is_unknown() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("document.unknown");
        std::fs::write(&file_path, b"%PDF-1.7\n").unwrap();

        assert_eq!(detect_or_validate(file_path.to_str(), None).unwrap(), PDF_MIME_TYPE);
    }

    #[test]
    fn should_detect_content_when_path_has_no_extension() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("document");
        std::fs::write(&file_path, b"%PDF-1.7\n").unwrap();

        assert_eq!(detect_or_validate(file_path.to_str(), None).unwrap(), PDF_MIME_TYPE);
    }

    #[test]
    fn should_preserve_unknown_extension_error_when_content_is_unrecognized() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("document.unknown");
        File::create(&file_path).unwrap();

        let error = detect_or_validate(file_path.to_str(), None).unwrap_err();
        assert!(matches!(
            error,
            XbergError::UnsupportedFormat(message) if message == "Unknown extension: .unknown"
        ));
    }

    #[test]
    fn should_preserve_path_error_when_extensionless_content_is_unrecognized() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("document");
        File::create(&file_path).unwrap();

        let error = detect_or_validate(file_path.to_str(), None).unwrap_err();
        assert!(matches!(
            error,
            XbergError::Validation { message, .. }
                if message == format!("Could not determine MIME type from file path: {}", file_path.display())
        ));
    }

    #[cfg(any(feature = "office", feature = "hwpx", feature = "iwork", feature = "archives"))]
    #[test]
    fn should_limit_unknown_zip_fallback_to_the_bounded_header() {
        const PADDING: &[u8] = &[b'x'; MIME_SNIFF_LENGTH + 1];
        let archive = build_zip(&[("padding.bin", PADDING), ("word/document.xml", b"<document/>")]);
        let directory = tempdir().unwrap();
        let file_path = directory.path().join("document.unknown");
        std::fs::write(&file_path, archive).unwrap();

        assert_eq!(detect_or_validate(file_path.to_str(), None).unwrap(), ZIP_MIME_TYPE);
    }

    /// Regression for #1223: a file whose content is a DOCX but whose extension
    /// says .pdf must route by content, matching the bytes entry point — the
    /// path detector previously trusted the extension and picked the PDF
    /// extractor.
    #[test]
    fn misnamed_file_routes_by_content_not_extension() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/office/merged_cells.docx");
        let Ok(docx_bytes) = std::fs::read(&fixture) else {
            eprintln!("skipping: fixture not present at {fixture:?}");
            return;
        };
        let dir = tempdir().unwrap();
        let misnamed = dir.path().join("report.pdf");
        std::fs::write(&misnamed, &docx_bytes).unwrap();

        let detected = detect_or_validate(misnamed.to_str(), None).unwrap();
        assert_eq!(
            detected, "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "DOCX content named .pdf must detect as DOCX, not PDF"
        );
    }

    #[test]
    fn specialized_json_extensions_are_not_overridden_by_generic_json_detection() {
        let dir = tempdir().unwrap();
        let cases = [
            ("document.json", br#"{"value":1}"#.as_slice(), JSON_MIME_TYPE),
            ("records.jsonl", br#"{"value":1}"#.as_slice(), "application/x-ndjson"),
            (
                "notebook.ipynb",
                br#"{"cells":[],"metadata":{},"nbformat":4,"nbformat_minor":5}"#.as_slice(),
                "application/x-ipynb+json",
            ),
        ];

        for (filename, content, expected_mime) in cases {
            let path = dir.path().join(filename);
            std::fs::write(&path, content).unwrap();
            assert_eq!(
                detect_or_validate(path.to_str(), None).unwrap(),
                expected_mime,
                "{filename} should retain its extension-specific JSON MIME type"
            );
        }
    }

    #[test]
    fn unsupported_specialized_xml_extension_does_not_override_supported_content() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("feed.atom");
        std::fs::write(&path, br#"<?xml version="1.0"?><feed/>"#).unwrap();

        assert_eq!(detect_or_validate(path.to_str(), None).unwrap(), "text/xml");
    }

    #[test]
    fn unsupported_specialized_json_extension_does_not_override_supported_content() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("model.gltf");
        std::fs::write(&path, br#"{"asset":{"version":"2.0"}}"#).unwrap();

        assert_eq!(detect_or_validate(path.to_str(), None).unwrap(), JSON_MIME_TYPE);
    }

    #[test]
    fn prefer_content_bytes_falls_back_from_unsupported_specialized_extension_to_plain_text() {
        let detected = detect_or_validate_bytes(
            b"ordinary prose without markup",
            Some("feed.atom"),
            None,
            crate::core::config::MimeDetectionPolicy::PreferContent,
        )
        .unwrap();

        assert_eq!(detected, PLAIN_TEXT_MIME_TYPE);
    }

    #[test]
    fn prefer_content_file_falls_back_from_unsupported_specialized_extension_to_plain_text() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("feed.atom");
        std::fs::write(&path, b"ordinary prose without markup").unwrap();
        let mut file = File::open(&path).unwrap();

        let detected = detect_or_validate_file(
            &path,
            &mut file,
            None,
            crate::core::config::MimeDetectionPolicy::PreferContent,
        )
        .unwrap();

        assert_eq!(detected, PLAIN_TEXT_MIME_TYPE);
    }

    #[test]
    fn content_only_bytes_ignores_a_supported_filename_extension() {
        let detected = detect_or_validate_bytes(
            br#"{"kind":"content"}"#,
            Some("document.txt"),
            None,
            crate::core::config::MimeDetectionPolicy::ContentOnly,
        )
        .unwrap();

        assert_eq!(detected, JSON_MIME_TYPE);
    }

    #[test]
    fn content_only_file_ignores_a_supported_filename_extension() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("document.txt");
        std::fs::write(&path, br#"{"kind":"content"}"#).unwrap();
        let mut file = File::open(&path).unwrap();

        let detected = detect_or_validate_file(
            &path,
            &mut file,
            None,
            crate::core::config::MimeDetectionPolicy::ContentOnly,
        )
        .unwrap();

        assert_eq!(detected, JSON_MIME_TYPE);
    }

    /// Build an in-memory MS-CFB compound document with the given root storage
    /// CLSID, padded with a stream large enough to push the file past
    /// `MIME_SNIFF_LENGTH`. Mirrors `build_test_ppt_ole` in `extraction/ppt/mod.rs`.
    #[cfg(feature = "office")]
    fn build_test_ole2_document(clsid: &str, padding_len: usize) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut compound_file = cfb::CompoundFile::create(cursor).expect("create in-memory OLE container");
        compound_file
            .set_storage_clsid("/", uuid::Uuid::parse_str(clsid).expect("valid CLSID literal"))
            .expect("set root storage CLSID");
        compound_file
            .create_stream("/Padding")
            .expect("create padding stream")
            .write_all(&vec![0_u8; padding_len])
            .expect("write padding stream");
        compound_file.into_inner().into_inner()
    }

    #[cfg(feature = "office")]
    #[test]
    fn content_only_bytes_and_file_agree_on_a_legacy_ole2_document_past_the_sniff_window() {
        // Before the fix: `detect_or_validate_bytes` (whole buffer) detected
        // these correctly via `infer`'s `ole2()` matcher, while
        // `detect_or_validate_file` (bounded MIME_SNIFF_LENGTH prefix) failed
        // outright with "Could not detect MIME type from file content" — a
        // compound file cannot be typed from a truncated prefix because the FAT
        // sector chain that locates the root directory entry references
        // sectors the prefix does not contain (#1590). Same bytes, different
        // answer; a real Word/Excel/PowerPoint document is essentially always
        // larger than the 4096-byte sniff window. ~keep
        use crate::core::config::MimeDetectionPolicy;

        let cases = [
            ("00020906-0000-0000-c000-000000000046", LEGACY_WORD_MIME_TYPE),
            ("00020810-0000-0000-c000-000000000046", LEGACY_EXCEL_MIME_TYPE),
            ("64818d10-4f9b-11cf-86ea-00aa00b929e8", LEGACY_POWERPOINT_MIME_TYPE),
        ];

        for (clsid, expected_mime) in cases {
            let content = build_test_ole2_document(clsid, MIME_SNIFF_LENGTH * 2);
            assert!(
                content.len() > MIME_SNIFF_LENGTH,
                "fixture for {clsid} must exceed the sniff window to exercise #1590"
            );

            let from_bytes = detect_or_validate_bytes(&content, None, None, MimeDetectionPolicy::ContentOnly)
                .unwrap_or_else(|error| panic!("bytes API failed to detect {clsid}: {error}"));
            assert_eq!(from_bytes, expected_mime, "bytes API mismatch for {clsid}");

            let dir = tempdir().unwrap();
            let path = dir.path().join("legacy.bin");
            std::fs::write(&path, &content).unwrap();
            let mut file = File::open(&path).unwrap();
            let from_file = detect_or_validate_file(&path, &mut file, None, MimeDetectionPolicy::ContentOnly)
                .unwrap_or_else(|error| panic!("path API failed to detect {clsid}: {error}"));
            assert_eq!(from_file, expected_mime, "path API mismatch for {clsid}");

            assert_eq!(
                from_bytes, from_file,
                "bytes and path APIs must agree on identical content for {clsid}"
            );
        }
    }

    #[test]
    fn octet_stream_file_hint_falls_back_to_each_detection_policy() {
        use crate::core::config::MimeDetectionPolicy;

        let dir = tempdir().unwrap();
        let path = dir.path().join("document.txt");
        std::fs::write(&path, br#"{"kind":"content"}"#).unwrap();

        for (policy, expected) in [
            (MimeDetectionPolicy::PreferContent, JSON_MIME_TYPE),
            (MimeDetectionPolicy::TrustExtension, PLAIN_TEXT_MIME_TYPE),
            (MimeDetectionPolicy::ContentOnly, JSON_MIME_TYPE),
        ] {
            let mut file = File::open(&path).unwrap();
            let detected = detect_or_validate_file(&path, &mut file, Some(OCTET_STREAM_MIME_TYPE), policy).unwrap();
            assert_eq!(detected, expected, "unexpected MIME for {policy:?}");
        }
    }

    #[test]
    fn content_detection_parses_complete_json_beyond_the_header() {
        use crate::core::config::MimeDetectionPolicy;

        let dir = tempdir().unwrap();
        let prefix = r#"{"payload":""#;
        let split_multibyte = format!("{}{}é\"}}", prefix, "x".repeat(MIME_SNIFF_LENGTH - prefix.len() - 1));
        let cases = [
            format!(r#"{{"payload":"{}"}}"#, "x".repeat(MIME_SNIFF_LENGTH)),
            format!(r#"["{}"]"#, "x".repeat(MIME_SNIFF_LENGTH * 2)),
            format!("{}{{\"payload\":true}}", " ".repeat(MIME_SNIFF_LENGTH + 1)),
            split_multibyte,
        ];

        for (index, content) in cases.iter().enumerate() {
            let path = dir.path().join(format!("document-{index}.txt"));
            std::fs::write(&path, content).unwrap();
            for policy in [MimeDetectionPolicy::PreferContent, MimeDetectionPolicy::ContentOnly] {
                let mut file = File::open(&path).unwrap();
                let detected = detect_or_validate_file(&path, &mut file, None, policy).unwrap();
                assert_eq!(
                    detected, JSON_MIME_TYPE,
                    "unexpected MIME for case {index} with {policy:?}"
                );
            }
        }
    }

    #[test]
    fn content_detection_does_not_treat_large_array_like_text_as_json() {
        use crate::core::config::MimeDetectionPolicy;

        let dir = tempdir().unwrap();
        let cases = [
            ("styles.css", "[slot='header'] .logo { margin: 0; }\n".repeat(130)),
            ("settings.toml", "[database]\nserver = 'localhost'\n".repeat(150)),
        ];

        for (filename, content) in cases {
            assert!(content.len() > MIME_SNIFF_LENGTH);
            let path = dir.path().join(filename);
            std::fs::write(&path, content).unwrap();

            for policy in [MimeDetectionPolicy::PreferContent, MimeDetectionPolicy::ContentOnly] {
                let mut file = File::open(&path).unwrap();
                let detected = detect_or_validate_file(&path, &mut file, None, policy).unwrap();
                assert_ne!(
                    detected, JSON_MIME_TYPE,
                    "unexpected JSON MIME for {filename} with {policy:?}"
                );
            }
        }
    }

    #[test]
    fn content_detection_does_not_treat_whitespace_prefixed_prose_as_json() {
        use crate::core::config::MimeDetectionPolicy;

        let dir = tempdir().unwrap();
        let path = dir.path().join("document.txt");
        let content = format!("{}ordinary prose", " ".repeat(MIME_SNIFF_LENGTH));
        std::fs::write(&path, content).unwrap();

        for policy in [MimeDetectionPolicy::PreferContent, MimeDetectionPolicy::ContentOnly] {
            let mut file = File::open(&path).unwrap();
            let detected = detect_or_validate_file(&path, &mut file, None, policy).unwrap();
            assert_eq!(detected, PLAIN_TEXT_MIME_TYPE, "unexpected MIME for {policy:?}");
        }
    }

    #[test]
    fn test_detect_or_validate_neither() {
        let result = detect_or_validate(None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_case_insensitive_extensions() {
        let dir = tempdir().unwrap();

        let file_path = dir.path().join("test.PDF");
        File::create(&file_path).unwrap();
        let mime = detect_mime_type(&file_path, true).unwrap();
        assert_eq!(mime, "application/pdf");

        let file_path2 = dir.path().join("test.XLSX");
        File::create(&file_path2).unwrap();
        let mime2 = detect_mime_type(&file_path2, true).unwrap();
        assert_eq!(mime2, EXCEL_MIME_TYPE);
    }

    #[test]
    fn test_detect_office_format_from_zip_bytes() {
        let docx_bytes = build_zip(&[("word/document.xml", b"document")]);
        let mime = detect_mime_type_from_bytes(&docx_bytes).unwrap();
        assert_eq!(
            mime, DOCX_MIME_TYPE,
            "Should detect DOCX from ZIP with word/document.xml"
        );

        let xlsx_bytes = build_zip(&[("xl/workbook.xml", b"workbook")]);
        let mime = detect_mime_type_from_bytes(&xlsx_bytes).unwrap();
        assert_eq!(
            mime, EXCEL_MIME_TYPE,
            "Should detect XLSX from ZIP with xl/workbook.xml"
        );

        let pptx_bytes = build_zip(&[("ppt/presentation.xml", b"presentation")]);
        let mime = detect_mime_type_from_bytes(&pptx_bytes).unwrap();
        assert_eq!(
            mime, POWER_POINT_MIME_TYPE,
            "Should detect PPTX from ZIP with ppt/presentation.xml"
        );

        #[cfg(any(feature = "office", feature = "hwpx", feature = "iwork", feature = "archives"))]
        {
            for expected_mime in [ODT_MIME_TYPE, ODP_MIME_TYPE, ODS_MIME_TYPE] {
                let open_document_bytes = build_zip(&[("mimetype", expected_mime.as_bytes())]);
                let mime = detect_mime_type_from_bytes(&open_document_bytes).unwrap();
                assert_eq!(mime, expected_mime, "Should detect exact OpenDocument mimetype entry");
            }
        }

        let plain_zip_bytes: &[u8] = &[
            0x50, 0x4b, 0x03, 0x04, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, b't', b'e', b's', b't', b'.', b't',
            b'x', b't',
        ];
        let mime = detect_mime_type_from_bytes(plain_zip_bytes).unwrap();
        assert_eq!(mime, "application/zip", "Plain ZIP should remain as application/zip");
    }

    #[test]
    fn should_detect_jpeg_2000_codestream_magic_as_j2c() {
        let codestream = [0xFF, 0x4F, 0xFF, 0x51, 0x00, 0x2F, 0x00, 0x00];
        assert_eq!(detect_mime_type_from_bytes(&codestream).unwrap(), "image/j2c");
    }

    #[test]
    fn should_detect_specialized_formats_only_from_unambiguous_content() {
        let mut geopackage = vec![0_u8; 100];
        geopackage[..16].copy_from_slice(b"SQLite format 3\0");
        geopackage[68..72].copy_from_slice(b"GPKG");
        assert_eq!(detect_mime_type_from_bytes(&geopackage).unwrap(), GEOPACKAGE_MIME_TYPE);

        let kml = br#"<?xml version="1.0"?><kml xmlns="http://www.opengis.net/kml/2.2"><Placemark/></kml>"#;
        assert_eq!(detect_mime_type_from_bytes(kml).unwrap(), KML_MIME_TYPE);

        let xhtml = br#"<?xml version="1.0"?><html xmlns="http://www.w3.org/1999/xhtml"><body/></html>"#;
        assert_eq!(detect_mime_type_from_bytes(xhtml).unwrap(), "application/xhtml+xml");
        let decoy_xhtml = br#"<?xml version="1.0"?><html notxmlns="http://www.w3.org/1999/xhtml"><body/></html>"#;
        assert_eq!(detect_mime_type_from_bytes(decoy_xhtml).unwrap(), "text/xml");

        let geojson = br#"{"type":"Point","coordinates":[13.4,52.5]}"#;
        assert_eq!(detect_mime_type_from_bytes(geojson).unwrap(), GEOJSON_MIME_TYPE);
        assert_eq!(
            detect_mime_type_from_bytes(br#"{"type":"Point"}"#).unwrap(),
            JSON_MIME_TYPE
        );
    }

    #[test]
    fn should_detect_large_geojson_feature_collection() {
        let padding = "x".repeat(MIME_SNIFF_LENGTH + 1);
        let content = format!(r#"{{"type":"FeatureCollection","features":[],"metadata":"{padding}"}}"#);

        assert_eq!(
            detect_mime_type_from_bytes(content.as_bytes()).unwrap(),
            GEOJSON_MIME_TYPE
        );
    }

    #[cfg(any(feature = "office", feature = "hwpx", feature = "iwork", feature = "archives"))]
    #[test]
    fn full_zip_inspection_does_not_treat_payload_text_as_an_office_part() {
        let archive = build_zip(&[("notes.txt", b"the string word/document.xml is not a part name")]);
        assert_eq!(detect_mime_type_from_bytes(&archive).unwrap(), ZIP_MIME_TYPE);
    }

    #[cfg(feature = "office")]
    #[test]
    fn ooxml_content_types_preserve_document_subtypes() {
        let cases = [
            (
                "/word/document.xml",
                "application/vnd.ms-word.document.macroEnabled.main+xml",
                "application/vnd.ms-word.document.macroEnabled.12",
            ),
            (
                "/word/document.xml",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.template.main+xml",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.template",
            ),
            (
                "/ppt/presentation.xml",
                "application/vnd.openxmlformats-officedocument.presentationml.slideshow.main+xml",
                "application/vnd.openxmlformats-officedocument.presentationml.slideshow",
            ),
            (
                "/ppt/presentation.xml",
                "application/vnd.ms-powerpoint.presentation.macroEnabled.main+xml",
                "application/vnd.ms-powerpoint.presentation.macroEnabled.12",
            ),
            (
                "/xl/workbook.xml",
                "application/vnd.ms-excel.template.macroEnabled.main+xml",
                "application/vnd.ms-excel.template.macroEnabled.12",
            ),
            (
                "/xl/workbook.bin",
                "application/vnd.ms-excel.sheet.binary.macroEnabled.main",
                "application/vnd.ms-excel.sheet.binary.macroEnabled.12",
            ),
        ];

        for (part_name, content_type, expected) in cases {
            let content_types = format!(
                r#"<?xml version="1.0"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Override PartName="{part_name}" ContentType="{content_type}"/>
</Types>"#
            );
            let archive = build_zip(&[
                ("[Content_Types].xml", content_types.as_bytes()),
                (part_name.trim_start_matches('/'), b"<main/>"),
            ]);
            assert_eq!(detect_mime_type_from_bytes(&archive).unwrap(), expected);
        }
    }

    #[cfg(feature = "office")]
    #[test]
    fn compatible_extension_preserves_ooxml_subtype_when_declaration_is_missing() {
        let archive = build_zip(&[("word/document.xml", b"<w:document/>")]);
        assert_eq!(
            detect_or_validate_bytes(
                &archive,
                Some("report.docm"),
                None,
                crate::core::config::MimeDetectionPolicy::PreferContent,
            )
            .unwrap(),
            "application/vnd.ms-word.document.macroEnabled.12"
        );
    }

    #[test]
    #[cfg(any(feature = "office", feature = "hwpx", feature = "iwork", feature = "archives"))]
    fn reordered_open_document_mimetype_routes_by_exact_entry() {
        const PADDING: &[u8] = &[b'x'; 5_000];
        let dir = tempdir().unwrap();

        for (extension, expected_mime) in [("odt", ODT_MIME_TYPE), ("odp", ODP_MIME_TYPE), ("ods", ODS_MIME_TYPE)] {
            let bytes = build_zip(&[("padding.bin", PADDING), ("mimetype", expected_mime.as_bytes())]);
            assert_eq!(detect_mime_type_from_bytes(&bytes).unwrap(), expected_mime);

            let path = dir.path().join(format!("reordered.{extension}"));
            std::fs::write(&path, bytes).unwrap();
            assert_eq!(detect_or_validate(path.to_str(), None).unwrap(), expected_mime);
        }
    }

    #[test]
    #[cfg(any(feature = "office", feature = "hwpx", feature = "iwork", feature = "archives"))]
    fn odf_detection_rejects_decoys_and_invalid_mimetype_entries() {
        let generic_zip = build_zip(&[("decoy.txt", ODT_MIME_TYPE.as_bytes())]);
        assert_eq!(detect_mime_type_from_bytes(&generic_zip).unwrap(), ZIP_MIME_TYPE);

        let epub = build_zip(&[("mimetype", b"application/epub+zip")]);
        assert_eq!(detect_mime_type_from_bytes(&epub).unwrap(), "application/epub+zip");

        let mixed = build_zip(&[
            ("mimetype", ODT_MIME_TYPE.as_bytes()),
            ("decoy.txt", ODS_MIME_TYPE.as_bytes()),
        ]);
        assert_eq!(detect_mime_type_from_bytes(&mixed).unwrap(), ODT_MIME_TYPE);

        let oversized = build_zip(&[("mimetype", b"application/vnd.oasis.opendocument.text-extra")]);
        assert_eq!(detect_mime_type_from_bytes(&oversized).unwrap(), ZIP_MIME_TYPE);

        let mut duplicate = build_zip(&[
            ("mimetypa", ODT_MIME_TYPE.as_bytes()),
            ("mimetypb", ODP_MIME_TYPE.as_bytes()),
        ]);
        for offset in 0..duplicate.len().saturating_sub(b"mimetypa".len()) {
            let name = &duplicate[offset..offset + b"mimetypa".len()];
            if name == b"mimetypa" || name == b"mimetypb" {
                duplicate[offset..offset + b"mimetype".len()].copy_from_slice(b"mimetype");
            }
        }
        assert_eq!(detect_mime_type_from_bytes(&duplicate).unwrap(), ZIP_MIME_TYPE);

        let truncated = &mixed[..mixed.len() / 2];
        assert_eq!(detect_mime_type_from_bytes(truncated).unwrap(), ZIP_MIME_TYPE);
    }

    #[test]
    #[cfg(any(feature = "office", feature = "hwpx", feature = "iwork", feature = "archives"))]
    fn odf_zip_preflight_rejects_excessive_entry_count() {
        let archive = build_zip(&[("content.txt", b"content")]);
        let default_limits = SecurityLimits::default();
        assert!(zip_central_directory_within_limits(
            &mut Cursor::new(&archive),
            &default_limits
        ));

        let restricted_limits = SecurityLimits {
            max_files_in_archive: 0,
            ..default_limits
        };
        assert!(!zip_central_directory_within_limits(
            &mut Cursor::new(archive),
            &restricted_limits
        ));
    }

    #[test]
    #[cfg(any(feature = "office", feature = "hwpx", feature = "iwork", feature = "archives"))]
    fn odf_extension_does_not_override_generic_zip_content() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("not-an-open-document.odt");
        std::fs::write(&path, build_zip(&[("content.txt", b"plain archive")])).unwrap();

        assert_eq!(detect_or_validate(path.to_str(), None).unwrap(), ZIP_MIME_TYPE);
    }

    #[test]
    fn test_detect_pst_from_bytes() {
        let pst_bytes: &[u8] = &[0x21, 0x42, 0x44, 0x4E, 0x00, 0x00, 0x00, 0x00];
        let mime = detect_mime_type_from_bytes(pst_bytes).unwrap();
        assert_eq!(mime, PST_MIME_TYPE, "Should detect PST from magic bytes");
    }

    #[test]
    fn test_list_supported_formats_not_empty() {
        let formats = list_supported_formats();
        assert!(!formats.is_empty(), "Supported formats list should not be empty");
    }

    #[test]
    fn supported_counts_are_derived_from_the_registry() {
        let extensions: HashSet<&str> = FORMATS
            .iter()
            .flat_map(|entry| entry.extensions.iter().copied())
            .collect();

        assert_eq!(SUPPORTED_FORMAT_COUNT, FORMATS.len());
        assert_eq!(SUPPORTED_EXTENSION_COUNT, extensions.len());
    }

    #[test]
    fn test_list_supported_formats_sorted() {
        let formats = list_supported_formats();
        let extensions: Vec<&str> = formats.iter().map(|f| f.extension.as_str()).collect();
        let mut sorted = extensions.clone();
        sorted.sort();
        assert_eq!(extensions, sorted, "Formats should be sorted by extension");
    }

    #[test]
    fn test_list_supported_formats_includes_common_formats() {
        // `list_supported_formats` now filters against the registered extractor set
        // (#308), so assertions for extensions gated behind optional Cargo features
        // only hold when those features are compiled in.
        let formats = list_supported_formats();
        let extensions: Vec<&str> = formats.iter().map(|f| f.extension.as_str()).collect();

        #[cfg(feature = "pdf")]
        assert!(extensions.contains(&"pdf"), "Should include pdf");
        assert!(extensions.contains(&"md"), "Should include md");
        #[cfg(feature = "office")]
        assert!(extensions.contains(&"docx"), "Should include docx");
        #[cfg(feature = "html")]
        assert!(extensions.contains(&"html"), "Should include html");
        assert!(extensions.contains(&"txt"), "Should include txt");
        assert!(extensions.contains(&"csv"), "Should include csv");
        assert!(extensions.contains(&"json"), "Should include json");
        #[cfg(any(feature = "excel", feature = "excel-wasm"))]
        assert!(extensions.contains(&"xlsx"), "Should include xlsx");
    }

    #[test]
    fn test_list_supported_formats_has_valid_mime_types() {
        let formats = list_supported_formats();
        for format in &formats {
            assert!(!format.extension.is_empty(), "Extension should not be empty");
            assert!(!format.mime_type.is_empty(), "MIME type should not be empty");
            assert!(format.mime_type.contains('/'), "MIME type should contain '/'");
        }
    }

    #[test]
    fn test_formats_registry_consistency() {
        for (ext, mime) in EXT_TO_MIME.iter() {
            assert!(
                SUPPORTED_MIME_TYPES.contains(mime),
                "Extension '{}' maps to MIME '{}' which is not in SUPPORTED_MIME_TYPES",
                ext,
                mime
            );
        }
    }

    #[test]
    fn test_formats_registry_mdx() {
        assert_eq!(EXT_TO_MIME.get("mdx"), Some(&"text/mdx"));
        assert!(SUPPORTED_MIME_TYPES.contains("text/mdx"));
        assert!(SUPPORTED_MIME_TYPES.contains("text/x-mdx"));
    }

    #[test]
    fn geographic_formats_use_their_canonical_mime_types() {
        assert_eq!(EXT_TO_MIME.get("kml"), Some(&"application/vnd.google-earth.kml+xml"));
        assert_eq!(EXT_TO_MIME.get("geojson"), Some(&"application/geo+json"));
        assert!(SUPPORTED_MIME_TYPES.contains("application/vnd.google-earth.kml+xml"));
        assert!(SUPPORTED_MIME_TYPES.contains("application/geo+json"));
    }

    #[test]
    fn test_formats_registry_aliases() {
        assert!(
            SUPPORTED_MIME_TYPES.contains("text/x-markdown"),
            "text/x-markdown alias"
        );
        assert!(SUPPORTED_MIME_TYPES.contains("text/json"), "text/json alias");
        assert!(SUPPORTED_MIME_TYPES.contains("text/yaml"), "text/yaml alias");
        assert!(SUPPORTED_MIME_TYPES.contains("text/xml"), "text/xml alias");
        assert!(SUPPORTED_MIME_TYPES.contains("application/xhtml+xml"), "xhtml alias");
        assert!(SUPPORTED_MIME_TYPES.contains("image/pjpeg"), "pjpeg alias");
        assert!(SUPPORTED_MIME_TYPES.contains("image/x-bmp"), "x-bmp alias");
        assert!(
            SUPPORTED_MIME_TYPES.contains("application/x-zip-compressed"),
            "zip alias"
        );
        assert!(SUPPORTED_MIME_TYPES.contains("text/rtf"), "rtf alias");
        assert!(SUPPORTED_MIME_TYPES.contains("text/x-typst"), "typst alias");
        assert!(SUPPORTED_MIME_TYPES.contains("text/x-python"), "Python source alias");
        assert!(SUPPORTED_MIME_TYPES.contains("text/x-r-source"), "R source alias");
        assert!(SUPPORTED_MIME_TYPES.contains("text/x-julia"), "Julia source alias");
    }

    /// Every alias in [`FORMATS`] must route to the same extractor as its canonical MIME
    /// type.
    ///
    /// `validate_mime_type` accepts an alias verbatim — it does not normalize it to the
    /// canonical form — and `DocumentExtractorRegistry::get` resolves by exact string with
    /// no alias awareness. So an alias that no extractor lists in `supported_mime_types()`
    /// is advertised as supported by `list_supported_formats()` and then rejected as
    /// `UnsupportedFormat` at extraction time (#229, and #289 for the same shape).
    ///
    /// Formats whose canonical MIME has no registered extractor are skipped, so this stays
    /// valid under any feature set: it only ever asserts that an alias is no worse off than
    /// the canonical name beside it.
    #[test]
    fn every_declared_alias_resolves_to_the_same_extractor_as_its_canonical_mime() {
        crate::extractors::ensure_initialized().expect("failed to initialize default extractors");
        let registry = crate::plugins::registry::get_document_extractor_registry();
        let registry = registry.read();

        let mut unclaimed = Vec::new();
        for format in FORMATS {
            let Ok(canonical) = registry.get(format.mime_type) else {
                continue;
            };
            for alias in format.aliases {
                match validate_mime_type(alias).and_then(|mime| registry.get(&mime)) {
                    Ok(aliased) if aliased.name() == canonical.name() => {}
                    Ok(aliased) => unclaimed.push(format!(
                        "{alias} (alias of {}) resolves to {}, not {}",
                        format.mime_type,
                        aliased.name(),
                        canonical.name()
                    )),
                    Err(_) => unclaimed.push(format!(
                        "{alias} (alias of {}) resolves to no extractor, but {} does",
                        format.mime_type, format.mime_type
                    )),
                }
            }
        }

        assert!(
            unclaimed.is_empty(),
            "declared alias MIME types are advertised as supported but unroutable:\n  {}",
            unclaimed.join("\n  ")
        );
    }

    // #1625: a caller with no MIME type who sends legacy-encoded (non-UTF-8) text, such as a
    // Windows-1252 CSV export, must not be refused when xberg's extractors can read it.
    #[test]
    fn should_detect_windows1252_text_as_plain_text() {
        // "name;city\r\nJosé;München\r\n" encoded as Windows-1252 (0xE9 = 'é', 0xFC = 'ü').
        let windows_1252 = b"name;city\r\nJos\xe9;M\xfcnchen\r\n";

        assert_eq!(detect_mime_type_from_bytes(windows_1252).unwrap(), PLAIN_TEXT_MIME_TYPE);
    }

    #[test]
    fn should_still_detect_utf8_text_as_plain_text() {
        let utf8 = "name;city\r\nJosé;München\r\n".as_bytes();

        assert_eq!(detect_mime_type_from_bytes(utf8).unwrap(), PLAIN_TEXT_MIME_TYPE);
    }

    #[test]
    fn should_reject_non_utf8_binary_content_not_recognized_as_a_known_format() {
        // Bytes chosen to defeat both signals a too-eager fallback might rely on: an
        // encoding that maps every byte (so decoding alone "succeeds" without error)
        // and a low but nonzero share of ASCII, so the input still is not text.
        let binary: Vec<u8> = (0u8..=255).cycle().take(600).collect();
        assert!(std::str::from_utf8(&binary).is_err(), "fixture must not be valid UTF-8");

        let result = detect_mime_type_from_bytes(&binary);
        assert!(
            matches!(result, Err(XbergError::UnsupportedFormat(_))),
            "expected UnsupportedFormat, got {result:?}"
        );
    }

    // The fixture above carries NUL bytes, so it is rejected by the NUL guard before the byte
    // ratio is ever computed -- it cannot show that the ratio itself discriminates. This one
    // omits NUL so that the ratio is the only thing left to reject it. ~keep
    #[test]
    fn should_reject_nul_free_binary_content_on_the_printable_byte_ratio_alone() {
        let binary: Vec<u8> = (1u8..=255).cycle().take(600).collect();
        assert!(
            !binary.contains(&0),
            "fixture must exercise the ratio, not the NUL guard"
        );
        assert!(!looks_like_legacy_encoded_text(&binary));

        let result = detect_mime_type_from_bytes(&binary);
        assert!(
            matches!(result, Err(XbergError::UnsupportedFormat(_))),
            "expected UnsupportedFormat, got {result:?}"
        );
    }

    // #1625: curly quotes are the commonest sign that a file is Windows-1252 rather than
    // ISO-8859-1, and they live in `0x80..=0x9F`. A short sentence holding one pair of them
    // must still read as text. ~keep
    #[test]
    fn should_detect_windows1252_curly_quotes_as_plain_text() {
        let windows_1252 = b"He said \x93hello\x94 to Jos\xe9 and left.\r\n";

        assert_eq!(detect_mime_type_from_bytes(windows_1252).unwrap(), PLAIN_TEXT_MIME_TYPE);
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "mime/declared_type_tests.rs"]
mod declared_type_tests;
