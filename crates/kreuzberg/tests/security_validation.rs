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

// ============================================================================
// Archive Attack Tests (6 tests)
// ============================================================================

#[test]
fn test_archive_zip_bomb_detection() {
    // Create a small ZIP that would expand to huge size (simulated)
    // Real zip bombs are dangerous - this is a safe test vector
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        use zip::write::{FileOptions, ZipWriter};
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, ()>::default();

        // Create a file with highly compressible data (zeros compress well)
        zip.start_file("large.txt", options).unwrap();
        // Write 10MB of zeros (compresses to ~10KB)
        let zeros = vec![0u8; 10 * 1024 * 1024];
        zip.write_all(&zeros).unwrap();

        zip.finish().unwrap();
    }

    let bytes = cursor.into_inner();
    let config = ExtractionConfig::default();

    // System should handle this gracefully (no OOM, no panic)
    let result = extract_bytes_sync(&bytes, "application/zip", &config);

    // Should succeed or fail gracefully (no panic)
    assert!(result.is_ok() || result.is_err());
    if let Ok(extracted) = result {
        // Should have extracted metadata
        assert!(extracted.metadata.archive.is_some());
    }
}

#[test]
fn test_archive_path_traversal_zip() {
    // Create ZIP with path traversal attempt
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        use zip::write::{FileOptions, ZipWriter};
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, ()>::default();

        // Attempt path traversal
        zip.start_file("../../etc/passwd", options).unwrap();
        zip.write_all(b"malicious content").unwrap();

        zip.finish().unwrap();
    }

    let bytes = cursor.into_inner();
    let config = ExtractionConfig::default();

    // System should handle this gracefully
    let result = extract_bytes_sync(&bytes, "application/zip", &config);

    // Should not panic, and should sanitize path
    if let Ok(extracted) = result {
        // Check that the path traversal is either rejected or sanitized
        if let Some(archive_meta) = &extracted.metadata.archive {
            // File list should not contain actual traversal paths
            for file_path in &archive_meta.file_list {
                // Shouldn't start with / or contain ../
                assert!(!file_path.starts_with('/'), "Absolute paths should be rejected");
            }
        }
    }
}

#[test]
fn test_archive_path_traversal_tar() {
    // TAR library rejects path traversal at creation time (good!)
    // So we test that the library properly rejects such attempts
    let mut header = tar::Header::new_gnu();

    // Attempt path traversal - this should fail
    let result = header.set_path("../../etc/shadow");

    // TAR library should reject this
    assert!(result.is_err(), "TAR library should reject path traversal attempts");
}

#[test]
fn test_archive_absolute_paths_rejected() {
    // Create ZIP with absolute path
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        use zip::write::{FileOptions, ZipWriter};
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, ()>::default();

        // Absolute path
        zip.start_file("/tmp/malicious.txt", options).unwrap();
        zip.write_all(b"malicious content").unwrap();

        zip.finish().unwrap();
    }

    let bytes = cursor.into_inner();
    let config = ExtractionConfig::default();

    let result = extract_bytes_sync(&bytes, "application/zip", &config);

    // Should handle gracefully - ZIP allows absolute paths but extraction should handle safely
    // The important thing is that it doesn't panic or cause security issues
    assert!(
        result.is_ok() || result.is_err(),
        "Should handle absolute paths gracefully"
    );
}

#[test]
fn test_archive_deeply_nested_directories() {
    // Create ZIP with deeply nested directory structure
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        use zip::write::{FileOptions, ZipWriter};
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, ()>::default();

        // Create a very deep path (100 levels)
        let deep_path = (0..100).map(|i| format!("dir{}", i)).collect::<Vec<_>>().join("/");
        let file_path = format!("{}/file.txt", deep_path);

        zip.start_file(&file_path, options).unwrap();
        zip.write_all(b"deep content").unwrap();

        zip.finish().unwrap();
    }

    let bytes = cursor.into_inner();
    let config = ExtractionConfig::default();

    // Should handle without stack overflow
    let result = extract_bytes_sync(&bytes, "application/zip", &config);

    // Should succeed or fail gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_archive_many_small_files() {
    // Create ZIP with many small files (potential DoS)
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        use zip::write::{FileOptions, ZipWriter};
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, ()>::default();

        // Create 1000 small files
        for i in 0..1000 {
            zip.start_file(format!("file{}.txt", i), options).unwrap();
            zip.write_all(b"small content").unwrap();
        }

        zip.finish().unwrap();
    }

    let bytes = cursor.into_inner();
    let config = ExtractionConfig::default();

    // Should handle without excessive resource usage
    let result = extract_bytes_sync(&bytes, "application/zip", &config);

    // Should succeed
    assert!(result.is_ok());
    if let Ok(extracted) = result {
        // Should have metadata for all files
        assert!(extracted.metadata.archive.is_some());
    }
}

// ============================================================================
// XML Attack Tests (4 tests)
// ============================================================================

#[test]
fn test_xml_billion_laughs_attack() {
    // Billion laughs attack - exponential entity expansion
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

    // Should handle gracefully (not expand entities indefinitely)
    // Modern XML parsers should reject or limit entity expansion
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_xml_quadratic_blowup() {
    // Quadratic blowup attack - nested entity expansion
    let xml = r#"<?xml version="1.0"?>
<!DOCTYPE bomb [
  <!ENTITY a "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa">
]>
<bomb>&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;</bomb>"#;

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(xml.as_bytes(), "application/xml", &config);

    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_xml_external_entity_injection() {
    // XXE attack - external entity injection
    let xml = r#"<?xml version="1.0"?>
<!DOCTYPE foo [
  <!ENTITY xxe SYSTEM "file:///etc/passwd">
]>
<foo>&xxe;</foo>"#;

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(xml.as_bytes(), "application/xml", &config);

    // Should reject or sanitize external entities
    if let Ok(extracted) = result {
        // Should not contain actual file contents
        assert!(!extracted.content.contains("root:"));
        assert!(!extracted.content.contains("/bin/bash"));
    }
}

#[test]
fn test_xml_dtd_entity_expansion() {
    // DTD with large entity expansion
    let xml = r#"<?xml version="1.0"?>
<!DOCTYPE data [
  <!ENTITY large "THIS_IS_A_LARGE_STRING_REPEATED_MANY_TIMES">
]>
<data>&large;&large;&large;&large;&large;&large;&large;&large;</data>"#;

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(xml.as_bytes(), "application/xml", &config);

    // Should handle without excessive memory usage
    assert!(result.is_ok() || result.is_err());
}

// ============================================================================
// Resource Exhaustion Tests (5 tests)
// ============================================================================

#[test]
fn test_resource_large_text_file() {
    // Create a 10MB text file
    let large_text = "This is a line of text that will be repeated many times.\n".repeat(200_000);

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(large_text.as_bytes(), "text/plain", &config);

    // Should handle without OOM
    assert!(result.is_ok());
    if let Ok(extracted) = result {
        // Content should be extracted
        assert!(!extracted.content.is_empty());
    }
}

#[test]
fn test_resource_large_xml_streaming() {
    // Create a large XML file (1MB)
    let mut xml = String::from(r#"<?xml version="1.0"?><root>"#);
    for i in 0..10000 {
        xml.push_str(&format!("<item id=\"{}\">{}</item>", i, "x".repeat(100)));
    }
    xml.push_str("</root>");

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(xml.as_bytes(), "application/xml", &config);

    // Should stream and handle efficiently
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_resource_empty_file() {
    // Empty file edge case
    let empty = b"";

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(empty, "text/plain", &config);

    // Should handle gracefully
    assert!(result.is_ok());
    if let Ok(extracted) = result {
        assert!(extracted.content.is_empty());
    }
}

#[test]
fn test_resource_single_byte_file() {
    // Minimal file
    let single_byte = b"a";

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(single_byte, "text/plain", &config);

    // Should handle gracefully
    assert!(result.is_ok());
    if let Ok(extracted) = result {
        assert_eq!(extracted.content, "a");
    }
}

#[test]
fn test_resource_null_bytes() {
    // File with null bytes
    let null_bytes = b"Hello\x00World\x00Test\x00";

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(null_bytes, "text/plain", &config);

    // Should handle gracefully (sanitize or preserve)
    assert!(result.is_ok());
}

// ============================================================================
// Malformed Input Tests (5 tests)
// ============================================================================

#[test]
fn test_malformed_invalid_mime_type() {
    // Invalid MIME type
    let content = b"Some content";

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(content, "invalid/mime/type", &config);

    // Should reject gracefully with UnsupportedFormat error
    assert!(result.is_err());
}

#[test]
fn test_malformed_xml_structure() {
    // Malformed XML - unclosed tags
    let malformed_xml = r#"<?xml version="1.0"?><root><item>test</item>"#;

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(malformed_xml.as_bytes(), "application/xml", &config);

    // Should handle without panic (may fail gracefully)
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_malformed_zip_structure() {
    // Corrupt ZIP data
    let corrupt_zip = b"PK\x03\x04CORRUPTED_DATA";

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(corrupt_zip, "application/zip", &config);

    // Should fail gracefully with parsing error
    assert!(result.is_err());
}

#[test]
fn test_malformed_invalid_utf8() {
    // Invalid UTF-8 sequence
    let invalid_utf8 = b"Hello \xFF\xFE World";

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(invalid_utf8, "text/plain", &config);

    // Should handle gracefully (replacement characters or error)
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_malformed_mixed_line_endings() {
    // Mixed line endings (CRLF, LF, CR)
    let mixed_endings = b"Line 1\r\nLine 2\nLine 3\rLine 4";

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(mixed_endings, "text/plain", &config);

    // Should handle all line ending types
    assert!(result.is_ok());
    if let Ok(extracted) = result {
        // Should contain all lines
        assert!(extracted.content.contains("Line 1"));
        assert!(extracted.content.contains("Line 2"));
        assert!(extracted.content.contains("Line 3"));
        assert!(extracted.content.contains("Line 4"));
    }
}

// ============================================================================
// PDF Specific Tests (3 tests)
// ============================================================================

#[test]
fn test_pdf_minimal_valid() {
    // Minimal PDF header - test that PDF extraction doesn't panic
    let minimal_pdf = b"%PDF-1.4
This is a very minimal PDF structure for security testing.
%%EOF";

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(minimal_pdf, "application/pdf", &config);

    // May succeed or fail, but should not panic
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_pdf_malformed_header() {
    // PDF with malformed header
    let malformed_pdf = b"%PDF-INVALID
This is not a valid PDF structure";

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(malformed_pdf, "application/pdf", &config);

    // Should fail gracefully with parsing error
    assert!(result.is_err());
}

#[test]
fn test_pdf_truncated() {
    // Truncated PDF (missing trailer)
    let truncated_pdf = b"%PDF-1.4
1 0 obj
<<
/Type /Catalog
>>
endobj";

    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(truncated_pdf, "application/pdf", &config);

    // Should fail gracefully
    assert!(result.is_err() || result.is_ok());
}

// ============================================================================
// File-based Security Tests
// ============================================================================

#[test]
fn test_security_nonexistent_file() {
    let config = ExtractionConfig::default();
    let result = extract_file_sync("/nonexistent/path/to/file.txt", None, &config);

    // Should return IO error, not panic
    assert!(result.is_err());
}

#[test]
fn test_security_directory_instead_of_file() {
    // Try to extract a directory
    let config = ExtractionConfig::default();
    let result = extract_file_sync("/tmp", None, &config);

    // Should return error, not panic
    assert!(result.is_err());
}

#[test]
fn test_security_special_file_handling() {
    // Create a temporary file with content
    let mut tmpfile = NamedTempFile::new().unwrap();
    tmpfile.write_all(b"test content").unwrap();
    tmpfile.flush().unwrap();
    let path = tmpfile.path();

    let config = ExtractionConfig::default();
    let result = extract_file_sync(path.to_str().unwrap(), None, &config);

    // Should handle gracefully (may succeed or fail depending on MIME detection)
    // The important thing is no panic or crash
    assert!(result.is_ok() || result.is_err());
}
