//! Regression tests for two EPUB defects:
//!
//! 1. A reachable char-boundary panic in `collect_annotation_uris`
//!    (`crates/xberg/src/extractors/epub/mod.rs`). `TextAnnotation` byte offsets are
//!    recorded against the *raw* text buffer by `extraction::html::structure`
//!    (`structure.rs:749`), but the text they are later sliced from has already been
//!    through `normalize_whitespace` (`structure.rs:824`), which collapses whitespace
//!    runs and trims. That shift routinely lands an offset mid-codepoint on non-ASCII
//!    content, and the length-only guard that used to sit at the slice site did not
//!    catch that -- only a char-boundary check does.
//!
//! 2. Unbounded ZIP member reads: EPUB opened its ZIP archive with no size validation
//!    at all (`ZipBombValidator` was never called) and read every member --
//!    `META-INF/container.xml`, the OPF, every spine XHTML document, the cover image,
//!    and every `<img>` -- with a plain `read_to_string`/`read_to_end` with no upper
//!    bound. A ratio-clean but declared-small archive whose real (compressed) content
//!    decompresses to far more bytes than its header claims was read in full.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test/bench binaries print by design; org logging policy exempts tests
#![cfg(feature = "office")]

use std::io::{Cursor, Write};
// Used only by the `tokio-runtime`-gated test below. ~keep
#[cfg(feature = "tokio-runtime")]
use std::panic::{self, AssertUnwindSafe};
use xberg::core::config::ExtractionConfig;
use xberg::extractors::EpubExtractor;
use xberg::extractors::security::SecurityLimits;
use xberg::plugins::InternalDocumentExtractor;
use xberg::types::internal::{ElementKind, InternalDocument};
use zip::CompressionMethod;
use zip::write::{SimpleFileOptions, ZipWriter};

const EPUB_MIME_TYPE: &str = "application/epub+zip";

/// Join every non-structural element's text with `\n`, mirroring the helper used by
/// `epub_spine_semantics_tests.rs` so exact-content assertions are stable across the
/// element-tree render path rather than depending on the public rendering pipeline.
fn content(document: &InternalDocument) -> String {
    if let Some(content) = &document.pre_rendered_content {
        return content.clone();
    }

    document
        .elements
        .iter()
        .filter(|element| {
            !matches!(
                element.kind,
                ElementKind::ListStart { .. }
                    | ElementKind::ListEnd
                    | ElementKind::QuoteStart
                    | ElementKind::QuoteEnd
                    | ElementKind::GroupStart
                    | ElementKind::GroupEnd
                    | ElementKind::PageBreak
                    | ElementKind::Image { .. }
                    | ElementKind::Table { .. }
            )
        })
        .map(|element| element.text.as_str())
        .filter(|text| !text.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn container_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#
}

/// Build a minimal, otherwise well-formed EPUB with a single chapter whose `<body>`
/// content is `body` verbatim.
fn build_epub_with_chapter_body(body: &str) -> Vec<u8> {
    let opf_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<package version="3.0" unique-identifier="bookid" xmlns="http://www.idpf.org/2007/opf">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Test Book</dc:title>
    <dc:language>en</dc:language>
  </metadata>
  <manifest>
    <item id="ch1" href="ch1.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="ch1"/>
  </spine>
</package>"#;

    let chapter_xhtml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <head><title>Chapter One</title></head>
  <body>{}</body>
</html>"#,
        body
    );

    let mut buffer = Vec::new();
    {
        let mut writer = ZipWriter::new(Cursor::new(&mut buffer));
        let options = SimpleFileOptions::default();

        for (path, contents) in [
            ("META-INF/container.xml", container_xml().to_string()),
            ("OEBPS/content.opf", opf_xml.to_string()),
            ("OEBPS/ch1.xhtml", chapter_xhtml),
        ] {
            writer.start_file(path, options).expect("zip start_file failed");
            writer.write_all(contents.as_bytes()).expect("zip write failed");
        }

        writer.finish().expect("zip finish failed");
    }
    buffer
}

/// Locate the central directory record for `filename` and overwrite its declared
/// uncompressed-size field with `new_size`, leaving the compressed-size field (which
/// bounds how many *compressed* bytes the decompressor is fed) untouched.
///
/// `ZipBombValidator` and the actual decompression path both build their view of an
/// entry from this same central directory record (`zip::read::central_header_to_zip_file_inner`
/// populates one `ZipFileData` shared by both), so this single patch is enough to make
/// the validator believe the entry is tiny while decompression still produces the real,
/// much larger, content.
fn patch_central_directory_uncompressed_size(buffer: &mut [u8], filename: &str, new_size: u32) {
    const CENTRAL_DIRECTORY_SIGNATURE: [u8; 4] = [0x50, 0x4B, 0x01, 0x02];
    const FILE_NAME_LENGTH_OFFSET: usize = 28;
    const FILE_NAME_OFFSET: usize = 46;
    const UNCOMPRESSED_SIZE_OFFSET: usize = 24;

    let name_bytes = filename.as_bytes();
    let mut pos = 0usize;
    while pos + FILE_NAME_OFFSET <= buffer.len() {
        if buffer[pos..pos + 4] == CENTRAL_DIRECTORY_SIGNATURE {
            let name_len = u16::from_le_bytes([
                buffer[pos + FILE_NAME_LENGTH_OFFSET],
                buffer[pos + FILE_NAME_LENGTH_OFFSET + 1],
            ]) as usize;
            let name_start = pos + FILE_NAME_OFFSET;
            let name_end = name_start + name_len;
            if name_end <= buffer.len() && &buffer[name_start..name_end] == name_bytes {
                let size_at = pos + UNCOMPRESSED_SIZE_OFFSET;
                buffer[size_at..size_at + 4].copy_from_slice(&new_size.to_le_bytes());
                return;
            }
        }
        pos += 1;
    }
    panic!("could not locate a central directory record for '{filename}' to patch");
}

/// A link annotation whose recorded byte offsets no longer align with a character
/// boundary after whitespace normalization must not panic. Before the fix,
/// `collect_annotation_uris` guarded only the annotation's length against the text's
/// byte length, not that `start`/`end` fell on a UTF-8 char boundary.
///
/// The fixture's raw text is `"a  xyééé"` (the `<a>` contributes no visible
/// whitespace around it); `normalize_whitespace` collapses the double space and trims
/// it to `"a xyééé"`. The link annotation recorded against the *raw* buffer as
/// `start=4,end=5` therefore lands on `"a xyééé"` at a position where byte 4 begins
/// the first 'é' (a 2-byte UTF-8 sequence: valid boundaries are 4 and 6, not 5). The
/// unfixed code's length-only guard (`ann.end as usize <= text.len()`) passes, and
/// `&text[4..5]` panics with:
///
///   byte index 5 is not a char boundary; it is inside 'é' (bytes 4..6) of `a xyééé`
// `tokio::runtime::Runtime::new` needs tokio's `rt-multi-thread`, which the `office`
// feature alone does not pull in, so an `office`-only build failed to compile this test
// crate. The file's other tests are runtime-free; only this one needs the gate. ~keep
#[cfg(feature = "tokio-runtime")]
#[test]
fn test_link_annotation_landing_mid_codepoint_does_not_panic() {
    let bytes = build_epub_with_chapter_body(r#"<p>a  x<a href="http://example.com/e">y</a>ééé</p>"#);
    let config = ExtractionConfig::default();

    let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
        let runtime = tokio::runtime::Runtime::new().expect("failed to build a tokio runtime");
        runtime.block_on(EpubExtractor.extract_content(&bytes, EPUB_MIME_TYPE, &config))
    }));

    assert!(
        outcome.is_ok(),
        "extraction panicked on a link annotation whose offsets landed mid-codepoint"
    );

    let document = outcome
        .unwrap()
        .expect("extraction must still succeed; only the label is dropped, not the document");

    assert!(
        document
            .processing_warnings
            .iter()
            .any(|warning| warning.message.contains("did not align with a character boundary")),
        "expected a warning documenting the dropped label, got: {:?}",
        document.processing_warnings
    );
}

/// Positive control: an ordinary chapter with no non-ASCII offset drift must keep
/// extracting exactly the same text as before. This is a crash fix, not a behavior
/// change for well-behaved input -- a test that only checks non-emptiness would pass
/// just as well if the fix silently dropped or mangled content.
#[tokio::test]
async fn test_ordinary_epub_extracts_exact_content() {
    let bytes = build_epub_with_chapter_body("<p>Hello world.</p>");
    let config = ExtractionConfig::default();

    let document = EpubExtractor
        .extract_content(&bytes, EPUB_MIME_TYPE, &config)
        .await
        .expect("a well-formed EPUB must still extract successfully");

    assert_eq!(
        content(&document),
        "Hello world.",
        "an ordinary chapter's extracted text must match exactly, not merely be non-empty"
    );
}

/// An entry whose real compression ratio exceeds `SecurityLimits::max_compression_ratio`
/// must be rejected by `ZipBombValidator` before any parsing happens, with the specific
/// `SecurityError::ZipBombDetected`, not a generic parse failure.
///
/// `OEBPS/bomb.dat` is not referenced by any manifest/spine entry (there is no OPF in
/// this archive at all) -- `ZipBombValidator::validate` scans every entry in the
/// central directory unconditionally, so this is caught before `META-INF/container.xml`
/// is ever read.
#[tokio::test]
async fn test_high_compression_ratio_member_is_rejected() {
    const BOMB_PAYLOAD_LEN: usize = 5 * 1024 * 1024;
    let bomb_payload = vec![b'A'; BOMB_PAYLOAD_LEN];

    let mut buffer = Vec::new();
    {
        let mut writer = ZipWriter::new(Cursor::new(&mut buffer));
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        writer
            .start_file("META-INF/container.xml", options)
            .expect("zip start_file failed");
        writer.write_all(container_xml().as_bytes()).expect("zip write failed");

        writer
            .start_file("OEBPS/bomb.dat", options)
            .expect("zip start_file failed");
        writer.write_all(&bomb_payload).expect("zip write failed");

        writer.finish().expect("zip finish failed");
    }

    let config = ExtractionConfig::default();
    let error = EpubExtractor
        .extract_content(&buffer, EPUB_MIME_TYPE, &config)
        .await
        .expect_err("an entry whose compression ratio exceeds the configured limit must be rejected");

    let message = error.to_string();
    assert!(
        message.contains("Potential ZIP bomb detected"),
        "expected a SecurityError::ZipBombDetected message, got: {message}"
    );
}

/// An archive whose entry count exceeds `SecurityLimits::max_files_in_archive` must be
/// rejected with the specific `SecurityError::TooManyFiles`.
#[tokio::test]
async fn test_archive_with_too_many_entries_is_rejected() {
    let limits = SecurityLimits::default();
    let too_many = limits.max_files_in_archive + 1;

    let mut buffer = Vec::new();
    {
        let mut writer = ZipWriter::new(Cursor::new(&mut buffer));
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        for index in 0..too_many {
            writer
                .start_file(format!("OEBPS/f{index}.txt"), options)
                .expect("zip start_file failed");
        }
        writer.finish().expect("zip finish failed");
    }

    let config = ExtractionConfig::default();
    let error = EpubExtractor
        .extract_content(&buffer, EPUB_MIME_TYPE, &config)
        .await
        .expect_err("an archive exceeding max_files_in_archive must be rejected");

    let message = error.to_string();
    assert!(
        message.contains("too many files"),
        "expected a SecurityError::TooManyFiles message, got: {message}"
    );
}

/// The security validator trusts the ZIP central directory completely -- it never
/// decompresses anything. A lying header (small declared uncompressed size, but a real
/// compressed stream that decodes to far more) sails straight past it. This test proves
/// the `.take(MAX_EPUB_MEMBER_SIZE)` added to `read_file_from_zip` is a second,
/// independent line of defense that still bounds the read.
///
/// The OPF payload here is built so that, read in full, it is a syntactically valid (if
/// silly) OPF document: a real header, a giant filler XML comment, then the comment's
/// closing `-->` and `</package>`. If the whole ~17 MiB were read (the pre-fix
/// behavior), this would parse successfully and `extract_content` would return `Ok`.
/// With the read capped at `MAX_EPUB_MEMBER_SIZE` (16 MiB), the read is truncated
/// inside the filler comment, the XML is left unterminated, and `parse_opf` fails --
/// proving the cap fired even though the (lied) declared size claimed only 10 bytes.
#[tokio::test]
async fn test_bounded_read_fires_even_when_declared_size_understates_real_content() {
    const REAL_PAYLOAD_LEN: usize = 17 * 1024 * 1024;
    const LIED_DECLARED_SIZE: u32 = 10;

    let mut opf_content = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<package version="3.0" unique-identifier="bookid" xmlns="http://www.idpf.org/2007/opf">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Bomb</dc:title></metadata>
  <manifest><item id="ch1" href="ch1.xhtml" media-type="application/xhtml+xml"/></manifest>
  <spine><itemref idref="ch1"/></spine>
  <!--"#,
    );
    opf_content.push_str(&"a".repeat(REAL_PAYLOAD_LEN));
    opf_content.push_str("-->\n</package>");

    let mut buffer = Vec::new();
    {
        let mut writer = ZipWriter::new(Cursor::new(&mut buffer));
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        writer
            .start_file("META-INF/container.xml", options)
            .expect("zip start_file failed");
        writer.write_all(container_xml().as_bytes()).expect("zip write failed");

        writer
            .start_file("OEBPS/content.opf", options)
            .expect("zip start_file failed");
        writer.write_all(opf_content.as_bytes()).expect("zip write failed");

        writer.finish().expect("zip finish failed");
    }

    patch_central_directory_uncompressed_size(&mut buffer, "OEBPS/content.opf", LIED_DECLARED_SIZE);

    let config = ExtractionConfig::default();
    let error = EpubExtractor
        .extract_content(&buffer, EPUB_MIME_TYPE, &config)
        .await
        .expect_err(
            "a member whose real decompressed size exceeds the per-file cap must not be read in \
             full, even when the ZIP's declared size claims otherwise",
        );

    let message = error.to_string();
    assert!(
        message.contains("Failed to parse OPF file") || message.contains("larger than its declared uncompressed size"),
        "expected the read to be truncated mid-document or rejected by the zip reader, got: {error}"
    );
}
