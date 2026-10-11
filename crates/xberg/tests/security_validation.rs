//! Security validation tests.
//!
//! Tests the system's resilience against malicious inputs including:
//! - Archive attacks (zip bombs, path traversal)
//! - XML attacks (billion laughs, XXE)
//! - Resource exhaustion (large files, memory limits)
//! - Malformed inputs (invalid MIME, encoding)
//! - PDF-specific attacks (malicious JS, weak encryption)

mod helpers;
use helpers::{extract_bytes_document_blocking, extract_uri_document_blocking};

use std::io::Write;
use tempfile::NamedTempFile;
use xberg::XbergError;
use xberg::core::config::ExtractionConfig;

fn trim_trailing_newlines(value: &str) -> &str {
    value.trim_end_matches(['\n', '\r'])
}

fn assert_text_content(actual: &str, expected: &str) {
    assert_eq!(
        trim_trailing_newlines(actual),
        expected,
        "Content mismatch after trimming trailing newlines"
    );
}
#[test]
fn test_archive_zip_bomb_detection() {
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        use zip::write::{FileOptions, ZipWriter};
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, '_, ()>::default();

        zip.start_file("large.txt", options).expect("Operation failed");
        let zeros = vec![0u8; 10 * 1024 * 1024];
        zip.write_all(&zeros).expect("Operation failed");

        zip.finish().expect("Operation failed");
    }

    let bytes = cursor.into_inner();
    let config = ExtractionConfig::default();

    let result = extract_bytes_document_blocking(&bytes, "application/zip", &config);

    // The default `FileOptions` uses `CompressionMethod::Deflated` (the `deflate-flate2`
    // Cargo feature is on), and 10MB of zero bytes deflates to a tiny fraction of that,
    // so `ZipBombValidator` (crates/xberg/src/extractors/security.rs) rejects this well
    // before the 500MB `max_archive_size` ceiling is ever reached: the 100:1
    // `max_compression_ratio` fires first.
    let error = result.expect_err("a 10MB all-zero entry deflates far past the 100:1 ratio limit");
    assert!(
        matches!(error, XbergError::Validation { .. }),
        "expected a Validation error carrying the ZipBombDetected message, got: {error:?}"
    );
    assert!(
        error.to_string().contains("ZIP bomb"),
        "error must name the zip-bomb check, got: {error}"
    );
}

#[test]
fn test_archive_path_traversal_zip() {
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        use zip::write::{FileOptions, ZipWriter};
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, '_, ()>::default();

        zip.start_file("../../etc/passwd", options).expect("Operation failed");
        zip.write_all(b"malicious content").expect("Operation failed");

        zip.finish().expect("Operation failed");
    }

    let bytes = cursor.into_inner();
    let config = ExtractionConfig::default();

    let result = extract_bytes_document_blocking(&bytes, "application/zip", &config);

    // `ZipExtractor` (crates/xberg/src/extractors/archive.rs) never writes an extracted
    // entry to the filesystem using its archive-relative name -- `extract_zip_metadata`
    // (crates/xberg/src/extraction/archive/zip.rs) copies `file.name()` verbatim into
    // `file_list`, and `extract_zip_file_bytes` keys an in-memory `HashMap` by that same
    // raw string. There is therefore no root to escape: a `..`-prefixed name cannot
    // perform a zip-slip write here because no write happens at all. This test pins that
    // fact rather than a traversal *rejection* that would be dishonest to claim.
    let extracted = result.expect("a traversal-looking entry name is not itself invalid ZIP structure");
    let archive_meta = match extracted.metadata.format.as_ref() {
        Some(xberg::FormatMetadata::Archive(m)) => m,
        other => panic!("expected Archive format metadata, got: {other:?}"),
    };
    assert_eq!(
        archive_meta.file_list,
        vec!["../../etc/passwd".to_string()],
        "the raw entry name must survive into metadata unmodified: no sanitization is \
         performed (and none is required, since nothing is written to disk)"
    );
}

#[test]
fn test_archive_path_traversal_tar() {
    // `tar::Header::set_path` (and the `tar` crate generally) refuses to *set* a
    // traversing path -- that only proves the `tar` crate's own writer is defensive, not
    // that xberg's TAR reader (`extract_tar_metadata` in
    // crates/xberg/src/extraction/archive/tar.rs) does anything sane with a header that
    // already carries one. A real hostile TAR file is not produced by `tar::Header::set_path`
    // in the first place, so a faithful test has to build the header the same way an
    // attacker would: write the raw bytes into the fixed-width name field directly,
    // bypassing the path validation entirely. `GnuHeader::name` is a public `[u8; 100]`
    // field for exactly this kind of low-level construction, and `Builder::append` writes
    // whatever header it is given without re-validating the path.
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut builder = tar::Builder::new(&mut cursor);

        let data = b"malicious content";
        let mut header = tar::Header::new_gnu();
        let name = b"../../etc/shadow";
        {
            let gnu = header.as_gnu_mut().expect("a GNU header always has a GNU view");
            gnu.name[..name.len()].copy_from_slice(name);
        }
        header.set_entry_type(tar::EntryType::Regular);
        header.set_mode(0o644);
        header.set_size(data.len() as u64);
        header.set_cksum();

        builder
            .append(&header, &data[..])
            .expect("Builder::append does not validate the path");
        builder.finish().expect("Operation failed");
    }

    let bytes = cursor.into_inner();
    let config = ExtractionConfig::default();

    let result = extract_bytes_document_blocking(&bytes, "application/x-tar", &config);

    // Mirrors `test_archive_path_traversal_zip` above: `extract_tar_metadata` calls
    // `entry.path()` and copies the result verbatim into `file_list` with no traversal
    // check, and `TarExtractor` never writes an extracted entry to the filesystem using
    // that name, so there is no root to escape here either.
    let extracted = result.expect("a traversing TAR entry name is not itself invalid TAR structure");
    let archive_meta = match extracted.metadata.format.as_ref() {
        Some(xberg::FormatMetadata::Archive(m)) => m,
        other => panic!("expected Archive format metadata, got: {other:?}"),
    };
    assert_eq!(
        archive_meta.file_list,
        vec!["../../etc/shadow".to_string()],
        "the raw entry name must survive into metadata unmodified: xberg's TAR reader \
         performs no sanitization (and, as with ZIP, none is required today because \
         nothing is written to disk from this name -- see `ArchiveEntry::path`'s docs in \
         crates/xberg/src/extraction/archive/mod.rs for the hazard this leaves for a \
         future caller that DOES write to disk, and `ArchiveEntry::confined_path` for the \
         safe accessor such a caller must use instead)"
    );
}

#[test]
fn test_archive_absolute_paths_rejected() {
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        use zip::write::{FileOptions, ZipWriter};
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, '_, ()>::default();

        zip.start_file("/tmp/malicious.txt", options).expect("Operation failed");
        zip.write_all(b"malicious content").expect("Operation failed");

        zip.finish().expect("Operation failed");
    }

    let bytes = cursor.into_inner();
    let config = ExtractionConfig::default();

    let result = extract_bytes_document_blocking(&bytes, "application/zip", &config);

    // As with the relative-traversal case above, `ZipExtractor` never writes to the
    // filesystem using the archive's own entry names, so an absolute-looking name is
    // just an opaque string that survives into metadata unmodified.
    let extracted = result.expect("a leading-slash entry name is not itself invalid ZIP structure");
    let archive_meta = match extracted.metadata.format.as_ref() {
        Some(xberg::FormatMetadata::Archive(m)) => m,
        other => panic!("expected Archive format metadata, got: {other:?}"),
    };
    assert_eq!(
        archive_meta.file_list,
        vec!["/tmp/malicious.txt".to_string()],
        "the raw entry name must survive into metadata unmodified"
    );
}

#[test]
fn test_archive_deeply_nested_directories() {
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        use zip::write::{FileOptions, ZipWriter};
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, '_, ()>::default();

        let deep_path = (0..100).map(|i| format!("dir{}", i)).collect::<Vec<_>>().join("/");
        let file_path = format!("{}/file.txt", deep_path);

        zip.start_file(&file_path, options).expect("Operation failed");
        zip.write_all(b"deep content").expect("Operation failed");

        zip.finish().expect("Operation failed");
    }

    let bytes = cursor.into_inner();
    let config = ExtractionConfig::default();

    let result = extract_bytes_document_blocking(&bytes, "application/zip", &config);

    // `SecurityLimits::max_nesting_depth`/`max_xml_depth` bound XML/JSON element nesting
    // (via `SecurityBudget::depth`); nothing in `extract_zip_metadata` counts `/`-delimited
    // segments in an archive entry name, so a 100-segment path is unconditionally accepted.
    let extracted = result.expect("a 100-segment archive path has no depth limit applied to it");
    let archive_meta = match extracted.metadata.format.as_ref() {
        Some(xberg::FormatMetadata::Archive(m)) => m,
        other => panic!("expected Archive format metadata, got: {other:?}"),
    };
    let expected_path = format!(
        "{}/file.txt",
        (0..100).map(|i| format!("dir{i}")).collect::<Vec<_>>().join("/")
    );
    assert_eq!(
        archive_meta.file_list,
        vec![expected_path],
        "the full nested path must survive unmodified"
    );
}

#[test]
#[cfg(feature = "archives")]
fn test_archive_many_small_files() {
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        use zip::write::{FileOptions, ZipWriter};
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, '_, ()>::default();

        for i in 0..1000 {
            zip.start_file(format!("file{}.txt", i), options)
                .expect("Operation failed");
            zip.write_all(b"small content").expect("Operation failed");
        }

        zip.finish().expect("Operation failed");
    }

    let bytes = cursor.into_inner();
    let config = ExtractionConfig::default();

    let result = extract_bytes_document_blocking(&bytes, "application/zip", &config);

    assert!(result.is_ok());
    if let Ok(extracted) = result {
        assert!(extracted.metadata.format.is_some());
    }
}

#[test]
fn test_xml_billion_laughs_attack() {
    let xml = r#"<?xml version="1.0"?>
<!DOCTYPE lolz [
  <!ENTITY lol "lol">
  <!ENTITY lol1 "&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;">
  <!ENTITY lol2 "&lol1;&lol1;&lol1;&lol1;&lol1;&lol1;&lol1;&lol1;&lol1;&lol1;">
  <!ENTITY lol3 "&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;">
]>
<lolz>&lol3;</lolz>"#;

    let config = ExtractionConfig::default();
    let result = extract_bytes_document_blocking(xml.as_bytes(), "application/xml", &config);

    // `quick_xml` does not build an entity table from a `<!DOCTYPE ... [ <!ENTITY ...> ]>`
    // internal subset: a reference to a custom-declared entity like `&lol3;` arrives as
    // `Event::GeneralRef` and `resolve_general_ref`
    // (crates/xberg/src/utils/xml_utils.rs) resolves any name that is not one of the five
    // predefined XML entities (or `nbsp`) to an empty string. So this document can never
    // actually expand -- it must extract successfully with no exponential blowup.
    let extracted = result.expect("undeclared custom entities resolve to empty strings, never expanding");
    assert!(
        extracted.content.len() < 200,
        "a real billion-laughs expansion would be gigabytes; got {} bytes: {:?}",
        extracted.content.len(),
        extracted.content
    );
    assert!(
        !extracted.content.contains("lollollol"),
        "the &lol3; entity must not have been expanded into repeated text: {:?}",
        extracted.content
    );
}

#[test]
fn test_xml_quadratic_blowup() {
    let xml = r#"<?xml version="1.0"?>
<!DOCTYPE bomb [
  <!ENTITY a "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa">
]>
<bomb>&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;</bomb>"#;

    let config = ExtractionConfig::default();
    let result = extract_bytes_document_blocking(xml.as_bytes(), "application/xml", &config);

    // Same reasoning as the billion-laughs case: `&a;` is a custom entity with no
    // predefined resolution, so every reference resolves to an empty string and the
    // 22 references never multiply the 64-byte declared value.
    let extracted = result.expect("undeclared custom entities resolve to empty strings, never expanding");
    assert!(
        extracted.content.len() < 200,
        "a real quadratic blowup would be kilobytes-to-megabytes from this input; got {} bytes: {:?}",
        extracted.content.len(),
        extracted.content
    );
    assert!(
        !extracted.content.contains("aaaaaaaaaaaaaaaa"),
        "the &a; entity must not have been expanded into repeated text: {:?}",
        extracted.content
    );
}

#[test]
fn test_xml_external_entity_injection() {
    let xml = r#"<?xml version="1.0"?>
<!DOCTYPE foo [
  <!ENTITY xxe SYSTEM "file:///etc/passwd">
]>
<foo>&xxe;</foo>"#;

    let config = ExtractionConfig::default();
    let result = extract_bytes_document_blocking(xml.as_bytes(), "application/xml", &config);

    // `resolve_general_ref` (crates/xberg/src/utils/xml_utils.rs) never dereferences a
    // `SYSTEM` identifier: a `<!ENTITY xxe SYSTEM "...">` declaration is not parsed into
    // an entity table at all, so `&xxe;` is just an unrecognized name that resolves to an
    // empty string. No file is ever opened, so this must succeed unconditionally (not
    // only checked "if" it happens to succeed) and the output must be small and free of
    // any host file content.
    let extracted = result.expect("SYSTEM entities are never resolved, so extraction cannot fail on them");
    assert!(
        extracted.content.len() < 100,
        "no file content should ever be inlined; got {} bytes: {:?}",
        extracted.content.len(),
        extracted.content
    );
    assert!(!extracted.content.contains("root:"), "content: {:?}", extracted.content);
    assert!(
        !extracted.content.contains("/bin/bash"),
        "content: {:?}",
        extracted.content
    );
    assert!(
        !extracted.content.contains("/etc/passwd"),
        "the SYSTEM identifier itself must not leak into output either: {:?}",
        extracted.content
    );
}

#[test]
fn test_xml_dtd_entity_expansion() {
    let xml = r#"<?xml version="1.0"?>
<!DOCTYPE data [
  <!ENTITY large "THIS_IS_A_LARGE_STRING_REPEATED_MANY_TIMES">
]>
<data>&large;&large;&large;&large;&large;&large;&large;&large;</data>"#;

    let config = ExtractionConfig::default();
    let result = extract_bytes_document_blocking(xml.as_bytes(), "application/xml", &config);

    // `&large;` is a custom entity; it resolves to an empty string rather than to its
    // declared value, so it can never be expanded even once, let alone 8 times.
    let extracted = result.expect("undeclared custom entities resolve to empty strings, never expanding");
    assert!(
        !extracted.content.contains("THIS_IS_A_LARGE_STRING"),
        "the &large; entity must not have been expanded: {:?}",
        extracted.content
    );
}

#[test]
fn test_resource_large_text_file() {
    let large_text = "This is a line of text that will be repeated many times.\n".repeat(200_000);

    let config = ExtractionConfig::default();
    let result = extract_bytes_document_blocking(large_text.as_bytes(), "text/plain", &config);

    // `PlainTextExtractor` (crates/xberg/src/extractors/text.rs) splits on blank lines
    // ("\n\n"); this fixture has none, so the whole 11.8MB blob is one paragraph and must
    // survive verbatim (modulo the trailing-newline strip every plain-text extraction does).
    let extracted = result.expect("11.8MB of plain text is well under every default security limit");
    let expected = large_text.trim_end_matches(['\n', '\r']);
    assert_text_content(&extracted.content, expected);
}

#[test]
fn test_resource_large_xml_streaming() {
    let mut xml = String::from(r#"<?xml version="1.0"?><root>"#);
    for i in 0..10000 {
        xml.push_str(&format!("<item id=\"{}\">{}</item>", i, "x".repeat(100)));
    }
    xml.push_str("</root>");

    let config = ExtractionConfig::default();
    let result = extract_bytes_document_blocking(xml.as_bytes(), "application/xml", &config);

    // 10,000 flat `<item>` elements, ~1.2MB total, is well inside every default
    // `SecurityLimits` (content size, iteration count, depth), so this must succeed and
    // every item's 100-byte body must survive intact -- proving the parse is neither
    // truncated early nor (on the opposite failure mode) duplicated.
    let extracted = result.expect("10,000 flat elements is well under every default security limit");
    let x_count = extracted.content.chars().filter(|&c| c == 'x').count();
    assert_eq!(
        x_count,
        10_000 * 100,
        "every item's 100-byte text body must survive extraction exactly once"
    );
}

#[test]
fn test_resource_empty_file() {
    let empty = b"";

    let config = ExtractionConfig::default();
    let result = extract_bytes_document_blocking(empty, "text/plain", &config);

    assert!(result.is_ok());
    if let Ok(extracted) = result {
        assert!(extracted.content.is_empty());
    }
}

#[test]
fn test_resource_single_byte_file() {
    let single_byte = b"a";

    let config = ExtractionConfig::default();
    let result = extract_bytes_document_blocking(single_byte, "text/plain", &config);

    assert!(result.is_ok());
    if let Ok(extracted) = result {
        assert_text_content(&extracted.content, "a");
    }
}

#[test]
fn test_resource_null_bytes() {
    let null_bytes = b"Hello\x00World\x00Test\x00";

    let config = ExtractionConfig::default();
    let result = extract_bytes_document_blocking(null_bytes, "text/plain", &config);

    // NUL is not ASCII/Unicode whitespace, so `.trim()` in `PlainTextExtractor::
    // build_internal_document` never strips it; it must survive as ordinary content.
    let extracted = result.expect("embedded NUL bytes are not invalid UTF-8, extraction must succeed");
    assert!(
        extracted.content.contains("Hello")
            && extracted.content.contains("World")
            && extracted.content.contains("Test"),
        "text around the NUL bytes must survive: {:?}",
        extracted.content
    );
}

#[test]
fn test_malformed_invalid_mime_type() {
    let content = b"Some content";

    let config = ExtractionConfig::default();
    let result = extract_bytes_document_blocking(content, "invalid/mime/type", &config);

    assert!(result.is_err());
}

#[test]
fn test_malformed_xml_structure() {
    let malformed_xml = r#"<?xml version="1.0"?><root><item>test</item>"#;

    let config = ExtractionConfig::default();
    let result = extract_bytes_document_blocking(malformed_xml.as_bytes(), "application/xml", &config);

    // `EntityReader`'s `check_end_names` is explicitly turned off (crates/xberg/src/
    // extractors/xml.rs), so a document that runs out of input while elements are still
    // open never raises a parser error -- it just reaches `Event::Eof` early. The
    // extractor reports that with an "unclosed elements" warning instead of failing.
    let extracted = result.expect("an unclosed root element reaches EOF cleanly, it does not error");
    assert!(
        extracted.content.contains("test"),
        "the text that was present before truncation must still extract: {:?}",
        extracted.content
    );
    assert!(
        extracted
            .processing_warnings
            .iter()
            .any(|w| w.message.contains("unclosed") && w.message.contains("root")),
        "an unclosed-element warning naming 'root' must be reported: {:?}",
        extracted.processing_warnings
    );
}

#[test]
fn test_malformed_zip_structure() {
    let corrupt_zip = b"PK\x03\x04CORRUPTED_DATA";

    let config = ExtractionConfig::default();
    let result = extract_bytes_document_blocking(corrupt_zip, "application/zip", &config);

    assert!(result.is_err());
}

#[test]
fn test_malformed_invalid_utf8() {
    let invalid_utf8 = b"Hello \xFF\xFE World";

    let config = ExtractionConfig::default();
    let result = extract_bytes_document_blocking(invalid_utf8, "text/plain", &config);

    // `PlainTextExtractor` decodes through `decode_with_provenance`
    // (crates/xberg/src/utils/mod.rs), which is infallible: it either substitutes U+FFFD
    // (the non-`quality` build) or reinterprets under a detected single-byte encoding (the
    // `quality` build). Either way it never returns an error, so this must always succeed,
    // and the valid ASCII surrounding the bad bytes must survive under both builds.
    let extracted = result.expect("invalid UTF-8 is decoded lossily/reinterpreted, never rejected");
    assert!(
        extracted.content.contains("Hello") && extracted.content.contains("World"),
        "the valid ASCII text around the invalid bytes must survive: {:?}",
        extracted.content
    );
}

#[test]
fn test_malformed_mixed_line_endings() {
    let mixed_endings = b"Line 1\r\nLine 2\nLine 3\rLine 4";

    let config = ExtractionConfig::default();
    let result = extract_bytes_document_blocking(mixed_endings, "text/plain", &config);

    assert!(result.is_ok());
    if let Ok(extracted) = result {
        assert!(extracted.content.contains("Line 1"));
        assert!(extracted.content.contains("Line 2"));
        assert!(extracted.content.contains("Line 3"));
        assert!(extracted.content.contains("Line 4"));
    }
}

/// Assert that `bytes` is rejected the way every structurally-invalid PDF is:
/// `NativeDocument::open_bytes_with_passwords` (crates/xberg/src/pdf/native/mod.rs) wraps
/// `xberg_native_pdf::PdfDocument::from_bytes`'s parse failure (no xref/trailer to find) as
/// `XbergError::Parsing`. See `pdf_integration.rs::test_corrupted_pdf_returns_error_not_panic`
/// for the same contract on other malformed fixtures.
fn assert_rejected_as_invalid_pdf(bytes: &[u8]) {
    let config = ExtractionConfig::default();
    let error =
        extract_bytes_document_blocking(bytes, "application/pdf", &config).expect_err("must be rejected as invalid");
    assert!(
        matches!(error, XbergError::Parsing { .. }),
        "expected a Parsing error, got: {error:?}"
    );
    assert!(
        error.to_string().contains("xberg_native_pdf"),
        "error must name the failing parser, got: {error}"
    );
}

#[test]
fn test_pdf_minimal_valid() {
    // Despite the test's name, this fixture has a `%PDF` header and an `%%EOF` marker but
    // no xref table or trailer, so it is not structurally valid: `xberg_native_pdf` cannot
    // open it, exactly like the other two fixtures below.
    let minimal_pdf = b"%PDF-1.4
This is a very minimal PDF structure for security testing.
%%EOF";

    assert_rejected_as_invalid_pdf(minimal_pdf);
}

#[test]
fn test_pdf_malformed_header() {
    let malformed_pdf = b"%PDF-INVALID
This is not a valid PDF structure";

    assert_rejected_as_invalid_pdf(malformed_pdf);
}

#[test]
fn test_pdf_truncated() {
    let truncated_pdf = b"%PDF-1.4
1 0 obj
<<
/Type /Catalog
>>
endobj";

    assert_rejected_as_invalid_pdf(truncated_pdf);
}

#[test]
fn test_security_nonexistent_file() {
    let config = ExtractionConfig::default();
    let result = extract_uri_document_blocking("/nonexistent/path/to/file.txt", None, &config);

    assert!(result.is_err());
}

#[test]
fn test_security_directory_instead_of_file() {
    let config = ExtractionConfig::default();
    let result = extract_uri_document_blocking("/tmp", None, &config);

    assert!(result.is_err());
}

#[test]
fn test_security_special_file_handling() {
    let mut tmpfile = NamedTempFile::new().expect("Operation failed");
    tmpfile.write_all(b"test content").expect("Operation failed");
    tmpfile.flush().expect("Operation failed");
    let path = tmpfile.path();

    let config = ExtractionConfig::default();
    let result = extract_uri_document_blocking(path.to_str().expect("Operation failed"), None, &config);

    let extracted = result.expect("an extensionless plain-text file should route by content");
    assert_text_content(&extracted.content, "test content");
    assert_eq!(extracted.mime_type, "text/plain");
}
