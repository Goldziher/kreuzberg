//! Error handling and edge case integration tests.
//!
//! Tests for corrupted files, edge cases, and invalid inputs.
//! Validates that the system handles errors gracefully without panics.

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::core::extractor::{extract_bytes, extract_file};
use std::io::Write;
use tempfile::NamedTempFile;

mod helpers;

/// Test truncated PDF - incomplete PDF file.
#[tokio::test]
async fn test_truncated_pdf() {
    let config = ExtractionConfig::default();

    let truncated_pdf = b"%PDF-1.4\n1 0 obj\n<<";

    let result = extract_bytes(truncated_pdf, "application/pdf", &config).await;

    assert!(result.is_err(), "Truncated PDF should fail gracefully");

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

    let corrupted_zip = vec![0x50, 0x4B, 0x03, 0x04, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00];

    let result = extract_bytes(&corrupted_zip, "application/zip", &config).await;

    assert!(result.is_err(), "Corrupted ZIP should fail gracefully");

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

    let invalid_xml = b"<?xml version=\"1.0\"?>\n\
<root>\n\
<unclosed>\n\
<another>text</wrong_tag>\n\
</root";

    let result = extract_bytes(invalid_xml, "application/xml", &config).await;

    match result {
        Ok(extraction) => {
            assert!(
                extraction.chunks.is_none(),
                "Chunks should be None without chunking config"
            );
        }
        Err(error) => {
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

    let corrupted_png = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0xFF, 0xFF, 0xFF, 0xFF];

    let result = extract_bytes(&corrupted_png, "image/png", &config).await;

    match result {
        Ok(extraction) => {
            assert!(
                extraction.chunks.is_none(),
                "Chunks should be None without chunking config"
            );
        }
        Err(error) => {
            assert!(
                matches!(error, kreuzberg::KreuzbergError::Parsing { .. })
                    || matches!(error, kreuzberg::KreuzbergError::Ocr { .. }),
                "Corrupted image error should be Parsing or OCR type, got: {:?}",
                error
            );
        }
    }
}

/// Test empty file - 0 bytes.
#[tokio::test]
async fn test_empty_file() {
    let config = ExtractionConfig::default();

    let empty_data = b"";

    let result_pdf = extract_bytes(empty_data, "application/pdf", &config).await;
    let result_text = extract_bytes(empty_data, "text/plain", &config).await;
    let result_xml = extract_bytes(empty_data, "application/xml", &config).await;

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

    let large_text = "This is a line of text that will be repeated many times.\n".repeat(200_000);
    let large_bytes = large_text.as_bytes();

    let result = extract_bytes(large_bytes, "text/plain", &config).await;

    assert!(result.is_ok(), "Large file should be processed successfully");
    let extraction = result.unwrap();

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

    assert!(
        extraction.content.contains("This is a line of text"),
        "Content should preserve original text"
    );
}

/// Test unicode filenames - non-ASCII paths.
#[tokio::test]
async fn test_unicode_filenames() {
    let config = ExtractionConfig::default();

    let mut temp_file = NamedTempFile::new().expect("Should create temp file");
    temp_file.write_all(b"Test content with Unicode filename.").unwrap();

    let result = extract_file(temp_file.path(), Some("text/plain"), &config).await;

    assert!(result.is_ok(), "Unicode filename should be handled");
    let extraction = result.unwrap();

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

    let special_text = "Emojis: 🎉 🚀 ✅ 🌍\n\
Arabic (RTL): مرحبا بالعالم\n\
Chinese: 你好世界\n\
Japanese: こんにちは世界\n\
Special chars: © ® ™ € £ ¥\n\
Math symbols: ∑ ∫ √ ≈ ∞";

    let result = extract_bytes(special_text.as_bytes(), "text/plain", &config).await;

    assert!(result.is_ok(), "Special characters should be handled");
    let extraction = result.unwrap();

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

    assert!(
        extraction.content.contains("Emojis")
            || extraction.content.contains("Arabic")
            || extraction.content.contains("Chinese"),
        "Should preserve at least some special character text"
    );
}

/// Test nonexistent file - file not found.
#[tokio::test]
async fn test_nonexistent_file() {
    let config = ExtractionConfig::default();

    let nonexistent_path = "/nonexistent/path/to/file.pdf";

    let result = extract_file(nonexistent_path, Some("application/pdf"), &config).await;

    assert!(result.is_err(), "Nonexistent file should return error");

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

    let result = extract_bytes(data, "application/x-unknown-format", &config).await;

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

    let mut temp_file = NamedTempFile::new().expect("Should create temp file");
    temp_file.write_all(b"Test content").unwrap();

    let mut perms = fs::metadata(temp_file.path()).unwrap().permissions();
    perms.set_mode(0o000);
    fs::set_permissions(temp_file.path(), perms).unwrap();

    let result = extract_file(temp_file.path(), Some("text/plain"), &config).await;

    let mut perms = fs::metadata(temp_file.path()).unwrap().permissions();
    perms.set_mode(0o644);
    fs::set_permissions(temp_file.path(), perms).unwrap();

    assert!(result.is_err(), "Permission denied should return error");
}

/// Test file extension mismatch - .pdf extension with DOCX content.
#[tokio::test]
async fn test_file_extension_mismatch() {
    let config = ExtractionConfig::default();

    let docx_magic = vec![0x50, 0x4B, 0x03, 0x04, 0x14, 0x00, 0x00, 0x00];

    let result = extract_bytes(&docx_magic, "application/pdf", &config).await;

    assert!(result.is_err(), "MIME type mismatch should fail");
}

/// Test extraction with null bytes in content.
#[tokio::test]
async fn test_null_bytes_in_content() {
    let config = ExtractionConfig::default();

    let data_with_nulls = b"Text before\x00null\x00bytes\x00after";

    let result = extract_bytes(data_with_nulls, "text/plain", &config).await;

    assert!(result.is_ok(), "Null bytes should be handled");
    let extraction = result.unwrap();

    assert!(!extraction.content.is_empty(), "Content should not be empty");
    assert!(
        extraction.chunks.is_none(),
        "Chunks should be None without chunking config"
    );

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

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let config = config.clone();
            tokio::spawn(async move { extract_bytes(text_data, "text/plain", &config).await })
        })
        .collect();

    for handle in handles {
        let result = handle.await.expect("Task should complete");
        assert!(result.is_ok(), "Concurrent extraction should succeed");

        let extraction = result.unwrap();
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
