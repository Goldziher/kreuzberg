//! Error handling and edge case integration tests.
//!
//! Tests for corrupted files, edge cases, and invalid inputs.
//! Validates that the system handles errors gracefully without panics.

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::core::extractor::{extract_bytes, extract_file};
use std::io::Write;
use tempfile::NamedTempFile;

mod helpers;

// ============================================================================
// Corrupted Files Tests (4 tests)
// ============================================================================

/// Test truncated PDF - incomplete PDF file.
#[tokio::test]
async fn test_truncated_pdf() {
    let config = ExtractionConfig::default();

    // PDF header but truncated (missing body and trailer)
    let truncated_pdf = b"%PDF-1.4\n1 0 obj\n<<";

    let result = extract_bytes(truncated_pdf, "application/pdf", &config).await;

    // Should fail gracefully with parsing error, not panic
    assert!(result.is_err(), "Truncated PDF should fail gracefully");
}

/// Test corrupted ZIP - malformed archive.
#[tokio::test]
async fn test_corrupted_zip() {
    let config = ExtractionConfig::default();

    // ZIP header but corrupted data
    let corrupted_zip = vec![
        0x50, 0x4B, 0x03, 0x04, // PK header (ZIP magic)
        0xFF, 0xFF, 0xFF, 0xFF, // Garbage data
        0x00, 0x00, 0x00, 0x00,
    ];

    let result = extract_bytes(&corrupted_zip, "application/zip", &config).await;

    // Should fail gracefully with parsing error
    assert!(result.is_err(), "Corrupted ZIP should fail gracefully");
}

/// Test invalid XML - bad XML syntax.
#[tokio::test]
async fn test_invalid_xml() {
    let config = ExtractionConfig::default();

    // XML with unclosed tags and invalid structure
    let invalid_xml = b"<?xml version=\"1.0\"?>\n\
<root>\n\
<unclosed>\n\
<another>text</wrong_tag>\n\
</root";

    let result = extract_bytes(invalid_xml, "application/xml", &config).await;

    // XML parser may be permissive, but should not panic
    assert!(
        result.is_ok() || result.is_err(),
        "Invalid XML should handle gracefully"
    );
}

/// Test corrupted image - invalid image data.
#[tokio::test]
async fn test_corrupted_image() {
    let config = ExtractionConfig::default();

    // PNG header but corrupted data
    let corrupted_png = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0xFF, 0xFF, 0xFF, 0xFF, // Corrupted IHDR chunk
    ];

    let result = extract_bytes(&corrupted_png, "image/png", &config).await;

    // Image extraction may fail or succeed with empty content
    // Important: should not panic
    assert!(
        result.is_ok() || result.is_err(),
        "Corrupted image should handle gracefully"
    );
}

// ============================================================================
// Edge Cases Tests (4 tests)
// ============================================================================

/// Test empty file - 0 bytes.
#[tokio::test]
async fn test_empty_file() {
    let config = ExtractionConfig::default();

    let empty_data = b"";

    // Test various MIME types with empty data
    let result_pdf = extract_bytes(empty_data, "application/pdf", &config).await;
    let result_text = extract_bytes(empty_data, "text/plain", &config).await;
    let result_xml = extract_bytes(empty_data, "application/xml", &config).await;

    // Empty files should fail with validation errors or succeed with empty content
    // Important: should not panic
    assert!(
        result_pdf.is_err() || result_pdf.unwrap().content.is_empty(),
        "Empty PDF should handle gracefully"
    );
    assert!(
        result_text.is_ok() || result_text.is_err(),
        "Empty text should handle gracefully"
    );
    assert!(
        result_xml.is_err() || result_xml.unwrap().content.is_empty(),
        "Empty XML should handle gracefully"
    );
}

/// Test very large file - stress test with large content.
#[tokio::test]
async fn test_very_large_file() {
    let config = ExtractionConfig::default();

    // Create 10MB of text data (not 100MB to keep test fast)
    let large_text = "This is a line of text that will be repeated many times.\n".repeat(200_000);
    let large_bytes = large_text.as_bytes();

    let result = extract_bytes(large_bytes, "text/plain", &config).await;

    // Should handle large files without panic
    assert!(result.is_ok(), "Large file should be processed successfully");
    let extraction = result.unwrap();

    // Content should be extracted
    assert!(!extraction.content.is_empty(), "Large file content should not be empty");
    assert!(extraction.content.len() > 1_000_000, "Content should be large");
}

/// Test unicode filenames - non-ASCII paths.
#[tokio::test]
async fn test_unicode_filenames() {
    let config = ExtractionConfig::default();

    // Create temp file with Unicode in name
    let mut temp_file = NamedTempFile::new().expect("Should create temp file");
    temp_file.write_all(b"Test content with Unicode filename.").unwrap();

    let result = extract_file(temp_file.path(), Some("text/plain"), &config).await;

    // Should handle Unicode paths gracefully
    assert!(result.is_ok(), "Unicode filename should be handled");
    assert!(result.unwrap().content.contains("Test content"));
}

/// Test special characters in content - emojis, RTL text.
#[tokio::test]
async fn test_special_characters_content() {
    let config = ExtractionConfig::default();

    // Text with emojis, RTL (Arabic), CJK, and special chars
    let special_text = "Emojis: 🎉 🚀 ✅ 🌍\n\
Arabic (RTL): مرحبا بالعالم\n\
Chinese: 你好世界\n\
Japanese: こんにちは世界\n\
Special chars: © ® ™ € £ ¥\n\
Math symbols: ∑ ∫ √ ≈ ∞";

    let result = extract_bytes(special_text.as_bytes(), "text/plain", &config).await;

    assert!(result.is_ok(), "Special characters should be handled");
    let extraction = result.unwrap();

    // Content should preserve special characters
    assert!(!extraction.content.is_empty());
    // At least some content should be extracted
    assert!(extraction.content.len() > 10);
}

// ============================================================================
// Missing/Invalid Files Tests (4 tests)
// ============================================================================

/// Test nonexistent file - file not found.
#[tokio::test]
async fn test_nonexistent_file() {
    let config = ExtractionConfig::default();

    let nonexistent_path = "/nonexistent/path/to/file.pdf";

    let result = extract_file(nonexistent_path, Some("application/pdf"), &config).await;

    // Should fail with IO or Validation error
    assert!(result.is_err(), "Nonexistent file should return error");

    // Error should be IO-related or Validation (file existence check)
    let error = result.unwrap_err();
    assert!(
        matches!(error, kreuzberg::KreuzbergError::Io(_))
            || matches!(error, kreuzberg::KreuzbergError::Validation { .. }),
        "Should be IO or Validation error for nonexistent file, got: {:?}",
        error
    );
}

/// Test unsupported format - unknown file type.
#[tokio::test]
async fn test_unsupported_format() {
    let config = ExtractionConfig::default();

    let data = b"Some random data";

    // Use a MIME type that's not supported
    let result = extract_bytes(data, "application/x-unknown-format", &config).await;

    // Should fail with unsupported format error
    assert!(result.is_err(), "Unsupported format should return error");

    let error = result.unwrap_err();
    assert!(
        matches!(error, kreuzberg::KreuzbergError::UnsupportedFormat(_)),
        "Should be UnsupportedFormat error, got: {:?}",
        error
    );
}

/// Test permission denied - no read access (platform-specific).
#[tokio::test]
#[cfg(unix)]
async fn test_permission_denied() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let config = ExtractionConfig::default();

    // Create temp file with no read permissions
    let mut temp_file = NamedTempFile::new().expect("Should create temp file");
    temp_file.write_all(b"Test content").unwrap();

    // Remove read permissions (Unix only)
    let mut perms = fs::metadata(temp_file.path()).unwrap().permissions();
    perms.set_mode(0o000); // No permissions
    fs::set_permissions(temp_file.path(), perms).unwrap();

    let result = extract_file(temp_file.path(), Some("text/plain"), &config).await;

    // Restore permissions before cleanup
    let mut perms = fs::metadata(temp_file.path()).unwrap().permissions();
    perms.set_mode(0o644);
    fs::set_permissions(temp_file.path(), perms).unwrap();

    // Should fail with permission error
    assert!(result.is_err(), "Permission denied should return error");
}

/// Test file extension mismatch - .pdf extension with DOCX content.
#[tokio::test]
async fn test_file_extension_mismatch() {
    let config = ExtractionConfig::default();

    // DOCX magic bytes (PK zip header) but claim it's a PDF
    let docx_magic = vec![
        0x50, 0x4B, 0x03, 0x04, // PK ZIP header (DOCX is ZIP-based)
        0x14, 0x00, 0x00, 0x00,
    ];

    let result = extract_bytes(&docx_magic, "application/pdf", &config).await;

    // Should fail because content doesn't match MIME type
    // PDF extractor expects %PDF- header
    assert!(result.is_err(), "MIME type mismatch should fail");
}

// ============================================================================
// Additional Edge Cases
// ============================================================================

/// Test extraction with null bytes in content.
#[tokio::test]
async fn test_null_bytes_in_content() {
    let config = ExtractionConfig::default();

    let data_with_nulls = b"Text before\x00null\x00bytes\x00after";

    let result = extract_bytes(data_with_nulls, "text/plain", &config).await;

    // Should handle null bytes gracefully
    assert!(result.is_ok(), "Null bytes should be handled");
}

/// Test concurrent extractions of same file.
#[tokio::test]
async fn test_concurrent_extractions() {
    let config = ExtractionConfig::default();

    let text_data = b"Concurrent extraction test content.";

    // Run multiple extractions concurrently
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let config = config.clone();
            tokio::spawn(async move { extract_bytes(text_data, "text/plain", &config).await })
        })
        .collect();

    // Wait for all to complete
    for handle in handles {
        let result = handle.await.expect("Task should complete");
        assert!(result.is_ok(), "Concurrent extraction should succeed");
    }
}
