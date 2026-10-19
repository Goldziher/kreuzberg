use super::*;
use std::io::{Cursor, Write};
use tar::Builder as TarBuilder;
use zip::write::{FileOptions, ZipWriter};

#[tokio::test]
async fn test_zip_extractor() {
    let extractor = ZipExtractor::new();

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, '_, ()>::default();

        zip.start_file("test.txt", options).unwrap();
        zip.write_all(b"Hello, World!").unwrap();

        zip.finish().unwrap();
    }

    let bytes = cursor.into_inner();
    let config = ExtractionConfig::default();

    let result = extractor
        .extract_content(&bytes, "application/zip", &config)
        .await
        .unwrap();
    let result =
        crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

    assert_eq!(result.mime_type, "application/zip");
    assert!(result.content.contains("ZIP Archive"));
    assert!(result.content.contains("test.txt"));
    assert!(result.content.contains("Hello, World!"));
    assert!(result.metadata.format.is_some());
    let archive_meta = match result.metadata.format.as_ref().unwrap() {
        crate::types::FormatMetadata::Archive(meta) => meta,
        _ => panic!("Expected Archive metadata"),
    };
    assert_eq!(archive_meta.format, "ZIP");
    assert_eq!(archive_meta.file_count, 1);
}

/// Regression test for #121: member bodies were emitted by iterating an `AHashMap`, whose
/// order aHash randomizes per process, so the same archive rendered differently on every run.
///
/// The member names below are deliberately anti-alphabetical, so this also pins *which*
/// deterministic order was chosen: archive order, matching the "Files:" listing — a plain
/// `sort()` would have produced alpha/middle/zebra and failed here.
#[tokio::test]
async fn should_emit_archive_members_in_archive_order_not_hash_order() {
    let extractor = ZipExtractor::new();

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, '_, ()>::default();

        for (name, body) in [
            ("zebra.txt", "zebra body"),
            ("alpha.txt", "alpha body"),
            ("middle.txt", "middle body"),
        ] {
            zip.start_file(name, options).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }

        zip.finish().unwrap();
    }

    let bytes = cursor.into_inner();
    let config = ExtractionConfig::default();

    let result = extractor
        .extract_content(&bytes, "application/zip", &config)
        .await
        .unwrap();
    let result =
        crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

    let positions: Vec<usize> = ["=== zebra.txt ===", "=== alpha.txt ===", "=== middle.txt ==="]
        .iter()
        .map(|marker| {
            result
                .content
                .find(marker)
                .unwrap_or_else(|| panic!("{marker} missing from {:?}", result.content))
        })
        .collect();

    assert!(
        positions[0] < positions[1] && positions[1] < positions[2],
        "members must appear in archive order (zebra, alpha, middle); got offsets {positions:?} in {:?}",
        result.content
    );
}

#[tokio::test]
async fn test_zip_filters_bookkeeping_and_binary_junk_from_children() {
    let extractor = ZipExtractor::new();

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, '_, ()>::default();

        zip.start_file("report.txt", options).unwrap();
        zip.write_all(b"Quarterly report body.").unwrap();

        // macOS Finder bookkeeping file (binary "Bud1..." header).
        zip.start_file(".DS_Store", options).unwrap();
        zip.write_all(&[0x00, 0x00, 0x00, 0x01, b'B', b'u', b'd', b'1'])
            .unwrap();

        // AppleDouble resource fork sidecar under the macOS archive-utility folder.
        zip.start_file("__MACOSX/._report.txt", options).unwrap();
        zip.write_all(&[
            0x00, 0x05, 0x16, 0x07, 0x00, 0x02, b'M', b'a', b'c', b' ', b'O', b'S', b' ', b'X',
        ])
        .unwrap();

        // AppleDouble sidecar at the top level (same file, no __MACOSX wrapper).
        zip.start_file("._report.txt", options).unwrap();
        zip.write_all(&[
            0x00, 0x05, 0x16, 0x07, 0x00, 0x02, b'M', b'a', b'c', b' ', b'O', b'S', b' ', b'X',
        ])
        .unwrap();

        // Python bytecode cache.
        zip.start_file("__pycache__/mod.cpython-311.pyc", options).unwrap();
        zip.write_all(&[0x42, 0x0d, 0x0d, 0x0a, 0x00, 0x00, 0x00, 0x00])
            .unwrap();

        zip.finish().unwrap();
    }

    let bytes = cursor.into_inner();
    let config = ExtractionConfig::default();

    let result = extractor
        .extract_content(&bytes, "application/zip", &config)
        .await
        .unwrap();

    let children = result.children.expect("archive should extract the real document");
    assert_eq!(
        children.len(),
        1,
        "only report.txt should survive filtering: {children:?}"
    );
    assert_eq!(children[0].path, "report.txt");
    assert_eq!(children[0].mime_type, "text/plain");

    assert!(
        !result.processing_warnings.is_empty(),
        "expected a ProcessingWarning about filtered bookkeeping/binary entries"
    );
    let warning = &result.processing_warnings[0];
    assert_eq!(warning.source, "archive");
    assert!(
        warning.message.contains("Filtered"),
        "warning message should mention filtering: {}",
        warning.message
    );
}

/// Regression test for task #709: the per-entry recursive-extraction loop in
/// `build_archive_doc` must stop starting new entries once cancellation is
/// signalled, instead of working through every remaining entry regardless.
///
/// Proves the checkpoint actually stops work (not merely that an error comes
/// back elsewhere): the token is cancelled *before* extraction starts, so the
/// loop's first-iteration check must break before any of the 3 entries is
/// recursively extracted, leaving zero children — against code with the
/// checkpoint removed, all 3 entries are still recursed into regardless of
/// cancellation, so `children` would be `Some` with 3 entries, not `None`.
#[tokio::test]
async fn test_zip_extraction_stops_recursing_once_cancelled() {
    let extractor = ZipExtractor::new();

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, '_, ()>::default();
        for (name, content) in [("a.txt", "alpha"), ("b.txt", "bravo"), ("c.txt", "charlie")] {
            zip.start_file(name, options).unwrap();
            zip.write_all(content.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }
    let bytes = cursor.into_inner();

    let token = crate::cancellation::CancellationToken::new();
    token.cancel();
    let cancelled_config = ExtractionConfig {
        cancel_token: Some(token),
        ..ExtractionConfig::default()
    };

    let cancelled_result = extractor
        .extract_content(&bytes, "application/zip", &cancelled_config)
        .await
        .expect("extraction must not error when cancelled, only skip recursive children");

    assert!(
        cancelled_result.children.is_none(),
        "no archive entry should be recursively extracted once the token is already \
         cancelled, got {:?}",
        cancelled_result.children
    );
    assert!(
        cancelled_result
            .processing_warnings
            .iter()
            .any(|warning| warning.source == "archive" && warning.message.contains("cancelled")),
        "a cancelled run should explain why entries were skipped: {:?}",
        cancelled_result.processing_warnings
    );

    // Sanity check: the same 3 entries, uncancelled, all recurse normally — this
    // rules out the empty result above being an unrelated bug (e.g. MIME
    // detection rejecting `.txt`) rather than the cancellation checkpoint.
    let uncancelled_result = extractor
        .extract_content(&bytes, "application/zip", &ExtractionConfig::default())
        .await
        .unwrap();
    assert_eq!(
        uncancelled_result.children.map(|c| c.len()).unwrap_or(0),
        3,
        "all 3 entries should be recursively extracted when nothing is cancelled"
    );
}

#[tokio::test]
async fn test_zip_markdown_member_routes_to_markdown_extractor() {
    let markdown = "# Title\n\nBody paragraph.\n\n## Section\n\n- a\n- b\n";

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, '_, ()>::default();
        zip.start_file("doc.md", options).unwrap();
        zip.write_all(markdown.as_bytes()).unwrap();
        zip.finish().unwrap();
    }

    let bytes = cursor.into_inner();
    let config = ExtractionConfig {
        output_format: crate::core::config::OutputFormat::Markdown,
        ..Default::default()
    };

    let result = ZipExtractor::new()
        .extract_content(&bytes, "application/zip", &config)
        .await
        .unwrap();

    let children = result.children.expect("archive should extract its member");
    let member = children.iter().find(|c| c.path == "doc.md").unwrap();
    assert_eq!(member.mime_type, "text/markdown");

    let rendered = member
        .result
        .formatted_content
        .as_ref()
        .unwrap_or(&member.result.content);
    assert!(rendered.contains("# Title"), "heading lost: {rendered:?}");
    assert!(!rendered.contains("\\#"), "heading was escaped as prose: {rendered:?}");
}

#[tokio::test]
async fn test_tar_extractor() {
    let extractor = TarExtractor::new();

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut tar = TarBuilder::new(&mut cursor);

        let data = b"Hello, World!";
        let mut header = tar::Header::new_gnu();
        header.set_path("test.txt").unwrap();
        header.set_size(data.len() as u64);
        header.set_cksum();
        tar.append(&header, &data[..]).unwrap();

        tar.finish().unwrap();
    }

    let bytes = cursor.into_inner();
    let config = ExtractionConfig::default();

    let result = extractor
        .extract_content(&bytes, "application/x-tar", &config)
        .await
        .unwrap();
    let result =
        crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

    assert_eq!(result.mime_type, "application/x-tar");
    assert!(result.content.contains("TAR Archive"));
    assert!(result.content.contains("test.txt"));
    assert!(result.content.contains("Hello, World!"));
    assert!(result.metadata.format.is_some());
    let archive_meta = match result.metadata.format.as_ref().unwrap() {
        crate::types::FormatMetadata::Archive(meta) => meta,
        _ => panic!("Expected Archive metadata"),
    };
    assert_eq!(archive_meta.format, "TAR");
    assert_eq!(archive_meta.file_count, 1);
}

#[tokio::test]
async fn test_zip_extractor_invalid() {
    let extractor = ZipExtractor::new();
    let invalid_bytes = vec![0, 1, 2, 3, 4, 5];
    let config = ExtractionConfig::default();

    let result = extractor
        .extract_content(&invalid_bytes, "application/zip", &config)
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_tar_extractor_invalid() {
    let extractor = TarExtractor::new();
    let invalid_bytes = vec![0, 1, 2, 3, 4, 5];
    let config = ExtractionConfig::default();

    let result = extractor
        .extract_content(&invalid_bytes, "application/x-tar", &config)
        .await;
    assert!(result.is_err());
}

#[test]
fn test_zip_plugin_interface() {
    let extractor = ZipExtractor::new();
    assert_eq!(extractor.name(), "zip-extractor");
    assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
    assert!(extractor.supported_mime_types().contains(&"application/zip"));
    assert_eq!(extractor.priority(), 50);
}

#[test]
fn test_tar_plugin_interface() {
    let extractor = TarExtractor::new();
    assert_eq!(extractor.name(), "tar-extractor");
    assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
    assert!(extractor.supported_mime_types().contains(&"application/x-tar"));
    assert!(extractor.supported_mime_types().contains(&"application/tar"));
    assert_eq!(extractor.priority(), 50);
}

#[test]
fn test_gzip_plugin_interface() {
    let extractor = GzipExtractor::new();
    assert_eq!(extractor.name(), "gzip-extractor");
    assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
    assert!(extractor.supported_mime_types().contains(&"application/gzip"));
    assert!(extractor.supported_mime_types().contains(&"application/x-gzip"));
    assert_eq!(extractor.priority(), 50);
}

#[tokio::test]
async fn test_gzip_extractor_valid_data() {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(b"Hello from gzip extraction!").unwrap();
    let compressed = encoder.finish().unwrap();

    let extractor = GzipExtractor::new();
    let config = ExtractionConfig::default();
    let result = extractor
        .extract_content(&compressed, "application/gzip", &config)
        .await;
    assert!(result.is_ok());
    let extraction = result.unwrap();
    let extraction =
        crate::extraction::derive::derive_extraction_result(extraction, true, crate::core::config::OutputFormat::Plain);
    assert!(extraction.content.contains("Hello from gzip extraction!"));
}

#[tokio::test]
async fn test_gzip_extractor_invalid_data() {
    let extractor = GzipExtractor::new();
    let config = ExtractionConfig::default();
    let result = extractor
        .extract_content(&[0, 1, 2, 3], "application/gzip", &config)
        .await;
    assert!(result.is_err());
}
