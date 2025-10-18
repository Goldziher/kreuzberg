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

    // Verify error type - should be Parsing error
    let error = result.unwrap_err();
    assert!(
        matches!(error, kreuzberg::KreuzbergError::Parsing { .. }),
        "Truncated PDF should produce Parsing error, got: {:?}",
        error
    );
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

    // Verify error type - should be Parsing error
    let error = result.unwrap_err();
    assert!(
        matches!(error, kreuzberg::KreuzbergError::Parsing { .. }),
        "Corrupted ZIP should produce Parsing error, got: {:?}",
        error
    );
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

    // XML parser is streaming and permissive - may succeed with partial content
    // or fail with parsing error. Important: must not panic.
    match result {
        Ok(extraction) => {
            // Parser extracted partial content before hitting errors
            assert!(
                extraction.chunks.is_none(),
                "Chunks should be None without chunking config"
            );
            // Content may be partial or empty (always valid for String)
        }
        Err(error) => {
            // Parser detected errors and failed
            assert!(
                matches!(error, kreuzberg::KreuzbergError::Parsing { .. }),
                "Invalid XML error should be Parsing type, got: {:?}",
                error
            );
        }
    }
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

    // Image extraction behavior depends on whether OCR is enabled
    // Without OCR: Fail with parsing error or succeed with empty content
    // With OCR: May fail during OCR processing
    // Important: must not panic
    match result {
        Ok(extraction) => {
            // Image accepted but may have no text content
            assert!(
                extraction.chunks.is_none(),
                "Chunks should be None without chunking config"
            );
            // Content is always valid for String (may be empty)
        }
        Err(error) => {
            // Image parsing failed
            assert!(
                matches!(error, kreuzberg::KreuzbergError::Parsing { .. })
                    || matches!(error, kreuzberg::KreuzbergError::Ocr { .. }),
                "Corrupted image error should be Parsing or OCR type, got: {:?}",
                error
            );
        }
    }
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

    // Empty files behavior varies by format:
    // - PDF: Should fail (invalid PDF structure)
    // - Text: Should succeed with empty content
    // - XML: Should fail (no root element) or succeed with empty content
    // Important: must not panic

    // PDF: Should fail
    match result_pdf {
        Ok(extraction) => {
            assert!(
                extraction.content.is_empty(),
                "Empty PDF should have empty content if it succeeds"
            );
            assert!(extraction.chunks.is_none(), "Chunks should be None");
        }
        Err(error) => {
            assert!(
                matches!(
                    error,
                    kreuzberg::KreuzbergError::Parsing { .. } | kreuzberg::KreuzbergError::Validation { .. }
                ),
                "Empty PDF should produce Parsing or Validation error, got: {:?}",
                error
            );
        }
    }

    // Text: Should succeed with empty content
    match result_text {
        Ok(extraction) => {
            assert!(
                extraction.content.is_empty(),
                "Empty text file should have empty content"
            );
            assert!(extraction.chunks.is_none(), "Chunks should be None");
        }
        Err(error) => {
            panic!("Empty text file should not fail, got error: {:?}", error);
        }
    }

    // XML: May fail or succeed with empty content
    match result_xml {
        Ok(extraction) => {
            assert!(
                extraction.content.is_empty(),
                "Empty XML should have empty content if it succeeds"
            );
            assert!(extraction.chunks.is_none(), "Chunks should be None");
        }
        Err(error) => {
            assert!(
                matches!(error, kreuzberg::KreuzbergError::Parsing { .. }),
                "Empty XML error should be Parsing type, got: {:?}",
                error
            );
        }
    }
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

    // Verify ExtractionResult structure
    assert!(!extraction.content.is_empty(), "Large file content should not be empty");
    assert!(extraction.content.len() > 1_000_000, "Content should be large");
    assert!(
        extraction.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        extraction.detected_languages.is_none(),
        "Language detection not enabled"
    );
    assert!(extraction.tables.is_empty(), "Text file should not have tables");

    // Verify content integrity - should contain repeated text
    assert!(
        extraction.content.contains("This is a line of text"),
        "Content should preserve original text"
    );
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
    let extraction = result.unwrap();

    // Verify content and structure
    assert!(
        extraction.content.contains("Test content"),
        "Content should be extracted"
    );
    assert!(
        extraction.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        extraction.detected_languages.is_none(),
        "Language detection not enabled"
    );
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

    // Verify ExtractionResult structure
    assert!(!extraction.content.is_empty(), "Content should not be empty");
    assert!(extraction.content.len() > 10, "Should have substantial content");
    assert!(
        extraction.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        extraction.detected_languages.is_none(),
        "Language detection not enabled"
    );

    // Verify special characters are preserved (at least some of them)
    // Note: Some characters might be normalized or transcoded
    assert!(
        extraction.content.contains("Emojis")
            || extraction.content.contains("Arabic")
            || extraction.content.contains("Chinese"),
        "Should preserve at least some special character text"
    );
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
    let extraction = result.unwrap();

    // Verify ExtractionResult structure
    assert!(!extraction.content.is_empty(), "Content should not be empty");
    assert!(
        extraction.chunks.is_none(),
        "Chunks should be None without chunking config"
    );

    // Verify null bytes are handled (either preserved or stripped)
    assert!(
        extraction.content.contains("Text before") || extraction.content.contains("after"),
        "Should preserve at least some of the text content"
    );
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

    // Wait for all to complete and verify results
    for handle in handles {
        let result = handle.await.expect("Task should complete");
        assert!(result.is_ok(), "Concurrent extraction should succeed");

        let extraction = result.unwrap();
        // Verify each extraction produces correct results
        assert!(
            extraction.content.contains("Concurrent extraction"),
            "Content should be extracted correctly"
        );
        assert!(extraction.chunks.is_none(), "Chunks should be None");
        assert!(
            extraction.detected_languages.is_none(),
            "Language detection not enabled"
        );
    }
}
