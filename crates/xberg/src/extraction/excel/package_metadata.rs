//! Office-metadata, revision-header, and cell-comment extraction from the OOXML/ODS
//! package parts of a spreadsheet, split out of `excel.rs` to keep that file under the
//! line-count limit.

use super::*;

#[cfg(feature = "office")]
pub(super) fn extract_xlsx_office_metadata_from_file(file_path: &str) -> Result<HashMap<String, String>> {
    use std::fs::File;
    use zip::ZipArchive;

    // OSError/RuntimeError must bubble up - system errors need user reports ~keep
    let file = File::open(file_path)?;

    let mut archive =
        ZipArchive::new(file).map_err(|e| XbergError::parsing(format!("Failed to open ZIP archive: {}", e)))?;

    extract_xlsx_office_metadata_from_archive(&mut archive)
}

#[cfg(feature = "office")]
pub(super) fn extract_xlsx_office_metadata_from_bytes(data: &[u8]) -> Result<HashMap<String, String>> {
    use zip::ZipArchive;

    let cursor = Cursor::new(data);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| XbergError::parsing(format!("Failed to open ZIP archive: {}", e)))?;

    extract_xlsx_office_metadata_from_archive(&mut archive)
}

/// Read ODS document metadata from the spreadsheet's `meta.xml`.
///
/// An `.ods` is an ODF package, not an OOXML one: its metadata lives in `meta.xml`
/// under the ODF namespaces rather than in `docProps/core.xml`, so the OOXML reader
/// finds nothing and every ODS came back with no title, author or dates at all. ODT
/// and ODP already read this exact part through the same helper (#102).
///
/// The keys match [`extract_xlsx_office_metadata_from_archive`]'s so both spreadsheet
/// families populate `Metadata` identically downstream.
#[cfg(feature = "office")]
fn extract_ods_office_metadata_from_archive<R: std::io::Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
) -> Result<HashMap<String, String>> {
    let properties = crate::extraction::office_metadata::extract_odt_properties(archive)?;
    let mut metadata = HashMap::new();

    let mut insert = |key: &str, value: Option<String>| {
        if let Some(value) = value.filter(|value| !value.is_empty()) {
            metadata.insert(key.to_string(), value);
        }
    };

    insert("title", properties.title);
    insert("subject", properties.subject);
    insert("keywords", properties.keywords);
    insert("description", properties.description);
    insert("language", properties.language);
    insert("revision", properties.editing_cycles);
    insert("created_at", properties.creation_date);
    insert("modified_at", properties.date);

    // ODF splits authorship the other way round from OOXML: `meta:initial-creator` is
    // who created the document and `dc:creator` is who touched it last, whereas OOXML's
    // `dc:creator` is the original author. Map by role, not by tag name, and fall back to
    // `dc:creator` for the author when a producer omits `meta:initial-creator`.
    let author = properties.initial_creator.or_else(|| properties.creator.clone());
    insert("creator", author.clone());
    insert("created_by", author);
    insert("modified_by", properties.creator);

    Ok(metadata)
}

#[cfg(feature = "office")]
pub(super) fn extract_ods_office_metadata_from_file(file_path: &str) -> Result<HashMap<String, String>> {
    use std::fs::File;
    use zip::ZipArchive;

    // OSError/RuntimeError must bubble up - system errors need user reports ~keep
    let file = File::open(file_path)?;

    let mut archive =
        ZipArchive::new(file).map_err(|e| XbergError::parsing(format!("Failed to open ZIP archive: {}", e)))?;

    extract_ods_office_metadata_from_archive(&mut archive)
}

#[cfg(feature = "office")]
pub(super) fn extract_ods_office_metadata_from_bytes(data: &[u8]) -> Result<HashMap<String, String>> {
    use zip::ZipArchive;

    let cursor = Cursor::new(data);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| XbergError::parsing(format!("Failed to open ZIP archive: {}", e)))?;

    extract_ods_office_metadata_from_archive(&mut archive)
}

/// Insert the ECMA-376 core properties (title, creator, subject, …) into `metadata`, if the
/// archive carries a readable core-properties part. A missing/unreadable part is not an
/// error here — the archive simply contributes nothing to `metadata` for this part.
#[cfg(feature = "office")]
fn insert_core_properties_metadata<R: std::io::Read + std::io::Seek>(
    metadata: &mut HashMap<String, String>,
    archive: &mut zip::ZipArchive<R>,
) {
    let Ok(core) = extract_core_properties(archive) else {
        return;
    };
    if let Some(title) = core.title {
        metadata.insert("title".to_string(), title);
    }
    if let Some(creator) = core.creator {
        metadata.insert("creator".to_string(), creator.clone());
        metadata.insert("created_by".to_string(), creator);
    }
    if let Some(subject) = core.subject {
        metadata.insert("subject".to_string(), subject);
    }
    if let Some(keywords) = core.keywords {
        metadata.insert("keywords".to_string(), keywords);
    }
    if let Some(description) = core.description {
        metadata.insert("description".to_string(), description);
    }
    if let Some(modified_by) = core.last_modified_by {
        metadata.insert("modified_by".to_string(), modified_by);
    }
    if let Some(created) = core.created {
        metadata.insert("created_at".to_string(), created);
    }
    if let Some(modified) = core.modified {
        metadata.insert("modified_at".to_string(), modified);
    }
    if let Some(revision) = core.revision {
        metadata.insert("revision".to_string(), revision);
    }
    if let Some(category) = core.category {
        metadata.insert("category".to_string(), category);
    }
    if let Some(content_status) = core.content_status {
        metadata.insert("content_status".to_string(), content_status);
    }
    if let Some(language) = core.language {
        metadata.insert("language".to_string(), language);
    }
}

/// Insert the XLSX app-properties part (worksheet names, application, DocSecurity, …) into
/// `metadata`, if the archive carries a readable app-properties part.
#[cfg(feature = "office")]
fn insert_app_properties_metadata<R: std::io::Read + std::io::Seek>(
    metadata: &mut HashMap<String, String>,
    archive: &mut zip::ZipArchive<R>,
) {
    let Ok(app) = extract_xlsx_app_properties(archive) else {
        return;
    };
    if !app.worksheet_names.is_empty() {
        metadata.insert("worksheet_names".to_string(), app.worksheet_names.join(", "));
    }
    if let Some(company) = app.company {
        metadata.insert("organization".to_string(), company);
    }
    if let Some(application) = app.application {
        metadata.insert("application".to_string(), application);
    }
    if let Some(app_version) = app.app_version {
        metadata.insert("application_version".to_string(), app_version);
    }
    // #230: surface the raw DocSecurity integer plus its decoded ECMA-376 flags.
    // `XlsxAppProperties` never reaches `FormatMetadata::Excel`, so without this the
    // workbook's protection state was discarded entirely. ~keep
    if let Some(raw) = app.doc_security {
        metadata.insert(DOC_SECURITY_KEY.to_string(), raw.to_string());
        for (key, value) in decode_doc_security_flags(raw) {
            metadata.insert(key.to_string(), value.to_string());
        }
    }
}

/// Insert the workbook's custom properties into `metadata` under a `custom_` prefix, if the
/// archive carries a readable custom-properties part. Each JSON value is flattened to its
/// display string the same way regardless of its JSON type.
#[cfg(feature = "office")]
fn insert_custom_properties_metadata<R: std::io::Read + std::io::Seek>(
    metadata: &mut HashMap<String, String>,
    archive: &mut zip::ZipArchive<R>,
) {
    let Ok(custom) = extract_custom_properties(archive) else {
        return;
    };
    for (key, value) in custom {
        let value_str = match value {
            Value::String(s) => s,
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Array(_) | Value::Object(_) => value.to_string(),
        };
        metadata.insert(format!("custom_{}", key), value_str);
    }
}

#[cfg(feature = "office")]
fn extract_xlsx_office_metadata_from_archive<R: std::io::Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
) -> Result<HashMap<String, String>> {
    let mut metadata = HashMap::new();

    insert_core_properties_metadata(&mut metadata, archive);
    insert_app_properties_metadata(&mut metadata, archive);
    insert_custom_properties_metadata(&mut metadata, archive);

    Ok(metadata)
}

/// Extract revision headers from an in-memory `.xlsx`/`.xlsm`/`.xltm` blob.
///
/// Returns `None` when `xl/revisions/revisionHeaders.xml` is absent (the
/// common case for modern files that don't use legacy shared-workbook mode).
/// Returns `Some(vec![])` when the file exists but contains no `<header>`
/// elements. On any parse error the function logs a warning and returns `None`
/// so that the rest of extraction succeeds.
pub(super) fn extract_xlsx_revisions_from_bytes(data: &[u8]) -> Option<Vec<DocumentRevision>> {
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).ok()?;
    extract_xlsx_revisions_from_archive(&mut archive)
}

/// Extract revision headers from an `.xlsx`/`.xlsm`/`.xltm` file on disk.
///
/// Same semantics as [`extract_xlsx_revisions_from_bytes`].
pub(super) fn extract_xlsx_revisions_from_file(file_path: &str) -> Option<Vec<DocumentRevision>> {
    let file = std::fs::File::open(file_path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    extract_xlsx_revisions_from_archive(&mut archive)
}

/// Core revision-header parser shared by the file and bytes paths.
fn extract_xlsx_revisions_from_archive<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
) -> Option<Vec<DocumentRevision>> {
    const HEADERS_PATH: &str = "xl/revisions/revisionHeaders.xml";

    let xml_bytes = {
        let entry = match archive.by_name(HEADERS_PATH) {
            Ok(e) => e,
            Err(zip::result::ZipError::FileNotFound) => return None,
            Err(e) => {
                tracing::warn!(
                    path = HEADERS_PATH,
                    error = %e,
                    "failed to open xl/revisions/revisionHeaders.xml"
                );
                return None;
            }
        };
        let mut buf = Vec::new();
        if entry.take(MAX_EXCEL_ZIP_MEMBER_SIZE).read_to_end(&mut buf).is_err() {
            return None;
        }
        buf
    };

    match parse_revision_headers_xml(&xml_bytes) {
        Ok(revisions) => Some(revisions),
        Err(e) => {
            tracing::warn!(error = %e, "failed to parse xl/revisions/revisionHeaders.xml");
            None
        }
    }
}

/// Parse `xl/revisions/revisionHeaders.xml` and emit one `DocumentRevision`
/// per `<header>` element.
///
/// Each header carries a `guid` (→ `revision_id`), `userName` (→ `author`),
/// and `dateTime` (→ `timestamp`). `anchor` and `delta` are empty for v1;
/// per-cell log parsing (`revisionLog*.xml`) is a future follow-up.
///
/// `RevisionKind::FormatChange` is used as the closest available variant
/// because the header file does not distinguish what *kind* of changes the
/// revision contains — that information is in the per-revision log file.
fn parse_revision_headers_xml(xml_bytes: &[u8]) -> Result<Vec<DocumentRevision>> {
    let xml_str = crate::text::utf8_validation::from_utf8(xml_bytes)
        .map_err(|e| XbergError::parsing(format!("invalid UTF-8 in revisionHeaders.xml: {e}")))?;

    let doc = roxmltree::Document::parse(xml_str)
        .map_err(|e| XbergError::parsing(format!("failed to parse revisionHeaders.xml: {e}")))?;

    const SPREADSHEETML_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

    let mut revisions = Vec::new();
    for node in doc.descendants() {
        if !node.has_tag_name((SPREADSHEETML_NS, "header")) {
            continue;
        }
        let revision_id = node
            .attribute("guid")
            .unwrap_or("")
            .trim_matches(|c| c == '{' || c == '}')
            .to_string();
        let revision_id = if revision_id.is_empty() {
            format!("xlsx-rev-{}", revisions.len())
        } else {
            revision_id
        };
        let author = node.attribute("userName").filter(|s| !s.is_empty()).map(str::to_string);
        let timestamp = node.attribute("dateTime").filter(|s| !s.is_empty()).map(str::to_string);
        revisions.push(DocumentRevision {
            revision_id,
            author,
            timestamp,
            kind: RevisionKind::FormatChange,
            anchor: None,
            delta: RevisionDelta::default(),
        });
    }

    Ok(revisions)
}

/// Extract cell comments from an in-memory `.xlsx`/`.xlsm`/`.xltm` blob.
///
/// Returns `None` when the archive contains no `xl/comments*.xml` parts (the
/// common case: most workbooks have no cell comments) or cannot be opened as
/// a ZIP. Otherwise returns every comment found across all comment parts as
/// `"<cell_ref>: <text>"` entries joined by `"; "`.
///
/// Comments are *not* attributed to a specific sheet here: resolving which
/// `xl/comments<N>.xml` part belongs to which worksheet requires walking
/// `xl/worksheets/_rels/sheetN.xml.rels`, which is a follow-up (xberg-io/xberg#89).
/// The cell reference in each entry is still enough to locate the comment
/// inside the workbook.
pub(super) fn extract_xlsx_comments_from_bytes(data: &[u8]) -> Option<String> {
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).ok()?;
    extract_xlsx_comments_from_archive(&mut archive)
}

/// Extract cell comments from an `.xlsx`/`.xlsm`/`.xltm` file on disk. Same
/// semantics as [`extract_xlsx_comments_from_bytes`].
pub(super) fn extract_xlsx_comments_from_file(file_path: &str) -> Option<String> {
    let file = std::fs::File::open(file_path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    extract_xlsx_comments_from_archive(&mut archive)
}

/// Core comment-part scanner shared by the file and bytes paths.
fn extract_xlsx_comments_from_archive<R: Read + Seek>(archive: &mut zip::ZipArchive<R>) -> Option<String> {
    let mut comment_parts: Vec<String> = Vec::new();
    for i in 0..archive.len() {
        let name = archive.by_index(i).ok()?.name().ok()?.into_owned();
        if name.starts_with("xl/comments") && name.ends_with(".xml") {
            comment_parts.push(name);
        }
    }
    if comment_parts.is_empty() {
        return None;
    }
    comment_parts.sort();

    let mut entries = Vec::new();
    for part in comment_parts {
        let xml_bytes = {
            let entry = match archive.by_name(&part) {
                Ok(e) => e,
                Err(_) => continue,
            };
            let mut buf = Vec::new();
            if entry.take(MAX_EXCEL_ZIP_MEMBER_SIZE).read_to_end(&mut buf).is_err() {
                continue;
            }
            buf
        };
        if let Ok(parsed) = parse_comments_xml(&xml_bytes) {
            entries.extend(parsed);
        } else {
            tracing::warn!(part = %part, "failed to parse worksheet comments part");
        }
    }

    if entries.is_empty() {
        None
    } else {
        Some(entries.join("; "))
    }
}

/// Parse a single `xl/comments<N>.xml` part into `"<cell_ref>: <text>"`
/// strings, one per `<comment>` element with non-empty text. A comment's text
/// may be split across multiple `<r><t>` runs (rich text); these are
/// concatenated in document order.
pub(super) fn parse_comments_xml(xml_bytes: &[u8]) -> Result<Vec<String>> {
    let xml_str = crate::text::utf8_validation::from_utf8(xml_bytes)
        .map_err(|e| XbergError::parsing(format!("invalid UTF-8 in comments.xml: {e}")))?;

    let doc = roxmltree::Document::parse(xml_str)
        .map_err(|e| XbergError::parsing(format!("failed to parse comments.xml: {e}")))?;

    const SPREADSHEETML_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

    let mut entries = Vec::new();
    for node in doc.descendants() {
        if !node.has_tag_name((SPREADSHEETML_NS, "comment")) {
            continue;
        }
        let cell_ref = node.attribute("ref").unwrap_or("");
        let text: String = node
            .descendants()
            .filter(|n| n.has_tag_name((SPREADSHEETML_NS, "t")))
            .filter_map(|n| n.text())
            .collect();
        let text = text.trim();
        if !text.is_empty() {
            entries.push(format!("{cell_ref}: {text}"));
        }
    }

    Ok(entries)
}
