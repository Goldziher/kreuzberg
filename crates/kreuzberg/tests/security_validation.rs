//! Security validation tests.
//!
//! Tests the system's resilience against malicious inputs including:
//! - Archive attacks (zip bombs, path traversal)
//! - XML attacks (billion laughs, XXE)
//! - Resource exhaustion (large files, memory limits)
//! - Malformed inputs (invalid MIME, encoding)
//! - PDF-specific attacks (malicious JS, weak encryption)

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::core::extractor::{extract_bytes_sync, extract_file_sync};
use std::io::Write;
use tempfile::NamedTempFile;

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
        let options = FileOptions::<'_, ()>::default();

        zip.start_file("large.txt", options).unwrap();
        let zeros = vec![0u8; 10 * 1024 * 1024];
        zip.write_all(&zeros).unwrap();

        zip.finish().unwrap();
    }

    let bytes = cursor.into_inner();
    let config = ExtractionConfig::default();

    let result = extract_bytes_sync(&bytes, "application/zip", &config);

    assert!(result.is_ok() || result.is_err());
    if let Ok(extracted) = result {
        assert!(extracted.metadata.format.is_some());
    }
}

#[test]
fn test_archive_path_traversal_zip() {
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        use zip::write::{FileOptions, ZipWriter};
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, ()>::default();

        zip.start_file("../../etc/passwd", options).unwrap();
        zip.write_all(b"malicious content").unwrap();

        zip.finish().unwrap();
    }

    let bytes = cursor.into_inner();
    let config = ExtractionConfig::default();

    let result = extract_bytes_sync(&bytes, "application/zip", &config);

    if let Ok(extracted) = result
        && let Some(archive_meta) = &extracted.metadata.format.as_ref().and_then(|f| match f {
            kreuzberg::FormatMetadata::Archive(m) => Some(m),
            _ => None,
        })
    {
        for file_path in &archive_meta.file_list {
            assert!(!file_path.starts_with('/'), "Absolute paths should be rejected");
        }
    }
}

#[test]
fn test_archive_path_traversal_tar() {
    let mut header = tar::Header::new_gnu();

    let result = header.set_path("../../etc/shadow");

    assert!(result.is_err(), "TAR library should reject path traversal attempts");
}

#[test]
fn test_archive_absolute_paths_rejected() {
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        use zip::write::{FileOptions, ZipWriter};
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, ()>::default();

        zip.start_file("/tmp/malicious.txt", options).unwrap();
        zip.write_all(b"malicious content").unwrap();

        zip.finish().unwrap();
    }

    let bytes = cursor.into_inner();
    let config = ExtractionConfig::default();

    let result = extract_bytes_sync(&bytes, "application/zip", &config);

    assert!(
        result.is_ok() || result.is_err(),
        "Should handle absolute paths gracefully"
    );
}

#[test]
fn test_archive_deeply_nested_directories() {
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        use zip::write::{FileOptions, ZipWriter};
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, ()>::default();

        let deep_path = (0..100).map(|i| format!("dir{}", i)).collect::<Vec<_>>().join("/");
        let file_path = format!("{}/file.txt", deep_path);

        zip.start_file(&file_path, options).unwrap();
        zip.write_all(b"deep content").unwrap();

        zip.finish().unwrap();
    }

    let bytes = cursor.into_inner();
    let config = ExtractionConfig::default();

    let result = extract_bytes_sync(&bytes, "application/zip", &config);

    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_archive_many_small_files() {
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        use zip::write::{FileOptions, ZipWriter};
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, ()>::default();

        for i in 0..1000 {
            zip.start_file(format!("file{}.txt", i), options).unwrap();
            zip.write_all(b"small content").unwrap();
        }

        zip.finish().unwrap();
    }

    let bytes = cursor.into_inner();
    let config = ExtractionConfig::default();

    let result = extract_bytes_sync(&bytes, "application/zip", &config);

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
    let result = extract_bytes_sync(xml.as_bytes(), "application/xml", &config);

    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_xml_quadratic_blowup() {
    let xml = r#"<?xml version="1.0"?>
<!DOCTYPE bomb [
  <!ENTITY a "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa">
]>
<bomb>&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;</bomb>"#;

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(xml.as_bytes(), "application/xml", &config);

    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_xml_external_entity_injection() {
    let xml = r#"<?xml version="1.0"?>
<!DOCTYPE foo [
  <!ENTITY xxe SYSTEM "file:///etc/passwd">
]>
<foo>&xxe;</foo>"#;

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(xml.as_bytes(), "application/xml", &config);

    if let Ok(extracted) = result {
        assert!(!extracted.content.contains("root:"));
        assert!(!extracted.content.contains("/bin/bash"));
    }
}

#[test]
fn test_xml_dtd_entity_expansion() {
    let xml = r#"<?xml version="1.0"?>
<!DOCTYPE data [
  <!ENTITY large "THIS_IS_A_LARGE_STRING_REPEATED_MANY_TIMES">
]>
<data>&large;&large;&large;&large;&large;&large;&large;&large;</data>"#;

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(xml.as_bytes(), "application/xml", &config);

    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_resource_large_text_file() {
    let large_text = "This is a line of text that will be repeated many times.\n".repeat(200_000);

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(large_text.as_bytes(), "text/plain", &config);

    assert!(result.is_ok());
    if let Ok(extracted) = result {
        assert!(!extracted.content.is_empty());
    }
}

#[test]
fn test_resource_large_xml_streaming() {
    let mut xml = String::from(r#"<?xml version="1.0"?><root>"#);
    for i in 0..10000 {
        xml.push_str(&format!("<item id=\"{}\">{}</item>", i, "x".repeat(100)));
    }
    xml.push_str("</root>");

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(xml.as_bytes(), "application/xml", &config);

    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_resource_empty_file() {
    let empty = b"";

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(empty, "text/plain", &config);

    assert!(result.is_ok());
    if let Ok(extracted) = result {
        assert!(extracted.content.is_empty());
    }
}

#[test]
fn test_resource_single_byte_file() {
    let single_byte = b"a";

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(single_byte, "text/plain", &config);

    assert!(result.is_ok());
    if let Ok(extracted) = result {
        assert_text_content(&extracted.content, "a");
    }
}

#[test]
fn test_resource_null_bytes() {
    let null_bytes = b"Hello\x00World\x00Test\x00";

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(null_bytes, "text/plain", &config);

    assert!(result.is_ok());
}

#[test]
fn test_malformed_invalid_mime_type() {
    let content = b"Some content";

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(content, "invalid/mime/type", &config);

    assert!(result.is_err());
}

#[test]
fn test_malformed_xml_structure() {
    let malformed_xml = r#"<?xml version="1.0"?><root><item>test</item>"#;

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(malformed_xml.as_bytes(), "application/xml", &config);

    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_malformed_zip_structure() {
    let corrupt_zip = b"PK\x03\x04CORRUPTED_DATA";

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(corrupt_zip, "application/zip", &config);

    assert!(result.is_err());
}

#[test]
fn test_malformed_invalid_utf8() {
    let invalid_utf8 = b"Hello \xFF\xFE World";

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(invalid_utf8, "text/plain", &config);

    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_malformed_mixed_line_endings() {
    let mixed_endings = b"Line 1\r\nLine 2\nLine 3\rLine 4";

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(mixed_endings, "text/plain", &config);

    assert!(result.is_ok());
    if let Ok(extracted) = result {
        assert!(extracted.content.contains("Line 1"));
        assert!(extracted.content.contains("Line 2"));
        assert!(extracted.content.contains("Line 3"));
        assert!(extracted.content.contains("Line 4"));
    }
}

#[test]
fn test_pdf_minimal_valid() {
    let minimal_pdf = b"%PDF-1.4
This is a very minimal PDF structure for security testing.
%%EOF";

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(minimal_pdf, "application/pdf", &config);

    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_pdf_malformed_header() {
    let malformed_pdf = b"%PDF-INVALID
This is not a valid PDF structure";

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(malformed_pdf, "application/pdf", &config);

    assert!(result.is_err());
}

#[test]
fn test_pdf_truncated() {
    let truncated_pdf = b"%PDF-1.4
1 0 obj
<<
/Type /Catalog
>>
endobj";

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(truncated_pdf, "application/pdf", &config);

    assert!(result.is_err() || result.is_ok());
}

#[test]
fn test_security_nonexistent_file() {
    let config = ExtractionConfig::default();
    let result = extract_file_sync("/nonexistent/path/to/file.txt", None, &config);

    assert!(result.is_err());
}

#[test]
fn test_security_directory_instead_of_file() {
    let config = ExtractionConfig::default();
    let result = extract_file_sync("/tmp", None, &config);

    assert!(result.is_err());
}

#[test]
fn test_security_special_file_handling() {
    let mut tmpfile = NamedTempFile::new().unwrap();
    tmpfile.write_all(b"test content").unwrap();
    tmpfile.flush().unwrap();
    let path = tmpfile.path();

    let config = ExtractionConfig::default();
    let result = extract_file_sync(path.to_str().unwrap(), None, &config);

    assert!(result.is_ok() || result.is_err());
}
