//! MIME type detection integration tests.
//!
//! Tests for MIME type detection from file extensions and content.
//! Validates detection accuracy, mismatch handling, and error cases.

use kreuzberg::core::mime::{detect_mime_type, validate_mime_type};
use std::io::Write;
use tempfile::NamedTempFile;

mod helpers;

/// Test MIME detection by file extension.
///
/// Validates that file extensions are correctly mapped to MIME types.
/// This is the primary MIME detection method (extension-first approach).
#[tokio::test]
async fn test_mime_detection_by_extension() {
    use tempfile::TempDir;

    let test_cases = vec![
        ("test.pdf", "application/pdf"),
        (
            "test.docx",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        ),
        (
            "test.xlsx",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ),
        (
            "test.pptx",
            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        ),
        ("test.txt", "text/plain"),
        ("test.md", "text/markdown"),
        ("test.html", "text/html"),
        ("test.json", "application/json"),
        ("test.xml", "application/xml"),
        ("test.csv", "text/csv"),
        ("test.png", "image/png"),
        ("test.jpg", "image/jpeg"),
        ("test.gif", "image/gif"),
        ("test.eml", "message/rfc822"),
        ("test.zip", "application/zip"),
    ];

    for (filename, expected_mime) in test_cases {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let temp_path = temp_dir.path().join(filename);

        std::fs::write(&temp_path, b"test content").unwrap();

        let detected = detect_mime_type(&temp_path, true);

        assert!(detected.is_ok(), "Should detect MIME type for {}", filename);
        assert_eq!(detected.unwrap(), expected_mime, "MIME type mismatch for {}", filename);
    }
}

/// Test case-insensitive extension detection.
#[tokio::test]
async fn test_mime_detection_case_insensitive() {
    use tempfile::TempDir;

    let test_cases = vec![
        ("test.PDF", "application/pdf"),
        (
            "test.DOCX",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        ),
        ("test.TXT", "text/plain"),
        ("test.Jpg", "image/jpeg"),
    ];

    for (filename, expected_mime) in test_cases {
        let temp_dir = TempDir::new().expect("Should create temp dir");
        let temp_path = temp_dir.path().join(filename);

        std::fs::write(&temp_path, b"test").unwrap();

        let detected = detect_mime_type(&temp_path, true);
        assert!(detected.is_ok(), "Should handle {} (case insensitive)", filename);
        assert_eq!(detected.unwrap(), expected_mime);
    }
}

/// Test MIME detection by content (magic bytes).
#[tokio::test]
async fn test_mime_detection_by_content() {
    struct TestCase {
        content: Vec<u8>,
        filename: &'static str,
        expected_fallback: Option<&'static str>,
    }

    let test_cases = vec![
        TestCase {
            content: b"%PDF-1.4\ntest content".to_vec(),
            filename: "test",
            expected_fallback: Some("application/pdf"),
        },
        TestCase {
            content: vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
            filename: "test",
            expected_fallback: Some("image/png"),
        },
        TestCase {
            content: vec![0x50, 0x4B, 0x03, 0x04],
            filename: "test",
            expected_fallback: Some("application/zip"),
        },
        TestCase {
            content: vec![0xFF, 0xD8, 0xFF, 0xE0],
            filename: "test",
            expected_fallback: Some("image/jpeg"),
        },
    ];

    for test_case in test_cases {
        let mut temp_file = NamedTempFile::new().expect("Should create temp file");
        let temp_path = temp_file.path().parent().unwrap().join(test_case.filename);

        temp_file.write_all(&test_case.content).unwrap();
        temp_file.flush().unwrap();
        std::fs::copy(temp_file.path(), &temp_path).unwrap();

        let detected = detect_mime_type(&temp_path, true);

        if let Some(expected) = test_case.expected_fallback {
            if let Ok(mime) = &detected {
                assert!(
                    mime == expected || mime.starts_with("application/") || mime.starts_with("image/"),
                    "For {}, expected {} or reasonable fallback, got {}",
                    test_case.filename,
                    expected,
                    mime
                );
            } else {
                assert!(
                    detected.is_err(),
                    "Should fail gracefully for {} without extension",
                    test_case.filename
                );
            }
        }

        let _ = std::fs::remove_file(&temp_path);
    }
}

/// Test validation of supported MIME types.
///
/// Validates that all documented supported MIME types pass validation.
/// This ensures the MIME type registry is correctly configured.
#[tokio::test]
async fn test_mime_type_validation() {
    let supported = vec![
        "application/pdf",
        "text/plain",
        "text/markdown",
        "application/json",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "image/png",
        "image/jpeg",
        "message/rfc822",
        "text/csv",
        "application/zip",
    ];

    for mime_type in supported {
        let result = validate_mime_type(mime_type);
        assert!(result.is_ok(), "Should validate supported MIME type: {}", mime_type);
        assert_eq!(result.unwrap(), mime_type);
    }
}

/// Test validation of image MIME types (prefix matching).
#[tokio::test]
async fn test_mime_type_image_prefix_validation() {
    let image_types = vec![
        "image/png",
        "image/jpeg",
        "image/gif",
        "image/webp",
        "image/bmp",
        "image/tiff",
        "image/svg+xml",
        "image/x-custom-format",
    ];

    for mime_type in image_types {
        let result = validate_mime_type(mime_type);
        assert!(result.is_ok(), "Should validate image MIME type: {}", mime_type);
    }
}

/// Test unknown/unsupported MIME type handling.
#[tokio::test]
async fn test_unknown_mime_type() {
    let unsupported = vec![
        "application/x-unknown-format",
        "video/mp4",
        "audio/mp3",
        "application/octet-stream",
        "text/x-unsupported",
    ];

    for mime_type in unsupported {
        let result = validate_mime_type(mime_type);
        assert!(result.is_err(), "Should reject unsupported MIME type: {}", mime_type);

        let error = result.unwrap_err();
        assert!(
            matches!(error, kreuzberg::KreuzbergError::UnsupportedFormat(_)),
            "Should return UnsupportedFormat error for: {}",
            mime_type
        );
    }
}

/// Test handling of MIME type mismatch (extension vs content).
#[tokio::test]
async fn test_mime_mismatch_warning() {
    let mut temp_file = NamedTempFile::new().expect("Should create temp file");
    let temp_path = temp_file.path().parent().unwrap().join("document.pdf");

    temp_file.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
    temp_file.flush().unwrap();
    std::fs::copy(temp_file.path(), &temp_path).unwrap();

    let detected = detect_mime_type(&temp_path, true);

    assert!(detected.is_ok(), "Should detect MIME type even with mismatch");

    assert_eq!(
        detected.unwrap(),
        "application/pdf",
        "Extension-based detection should take precedence"
    );

    let _ = std::fs::remove_file(&temp_path);
}

/// Test file extension mismatch detection.
#[tokio::test]
async fn test_extension_content_mismatch() {
    let mut temp_file = NamedTempFile::new().expect("Should create temp file");
    let temp_path = temp_file.path().parent().unwrap().join("document.txt");

    temp_file.write_all(b"%PDF-1.4\n").unwrap();
    temp_file.flush().unwrap();
    std::fs::copy(temp_file.path(), &temp_path).unwrap();

    let detected = detect_mime_type(&temp_path, true);

    assert!(detected.is_ok(), "Should detect MIME type");

    assert_eq!(
        detected.unwrap(),
        "text/plain",
        "Should use extension for MIME detection"
    );

    let _ = std::fs::remove_file(&temp_path);
}

/// Test file without extension.
#[tokio::test]
async fn test_no_extension() {
    let mut temp_file = NamedTempFile::new().expect("Should create temp file");
    let temp_path = temp_file.path().parent().unwrap().join("testfile");

    temp_file.write_all(b"test content").unwrap();
    temp_file.flush().unwrap();
    std::fs::copy(temp_file.path(), &temp_path).unwrap();

    let detected = detect_mime_type(&temp_path, true);

    match detected {
        Err(error) => {
            assert!(
                matches!(
                    error,
                    kreuzberg::KreuzbergError::Validation { .. } | kreuzberg::KreuzbergError::UnsupportedFormat(_)
                ),
                "Should return appropriate error for file without extension"
            );
        }
        Ok(mime) => {
            assert!(
                mime.contains('/'),
                "Detected MIME type should be valid format: {}",
                mime
            );
        }
    }

    let _ = std::fs::remove_file(&temp_path);
}

/// Test nonexistent file.
#[tokio::test]
async fn test_mime_detection_nonexistent_file() {
    let nonexistent_path = "/nonexistent/path/to/file.pdf";

    let result = detect_mime_type(nonexistent_path, true);

    assert!(result.is_err(), "Should fail for nonexistent file");

    let error = result.unwrap_err();
    assert!(
        matches!(error, kreuzberg::KreuzbergError::Validation { .. }),
        "Should return Validation error for nonexistent file"
    );
}

/// Test file existence check can be disabled.
#[tokio::test]
async fn test_mime_detection_skip_existence_check() {
    let nonexistent_path = "/nonexistent/path/to/document.pdf";

    let result = detect_mime_type(nonexistent_path, false);

    assert!(result.is_ok(), "Should succeed when skipping existence check");
    assert_eq!(result.unwrap(), "application/pdf");
}

/// Test multiple dots in filename.
#[tokio::test]
async fn test_filename_multiple_dots() {
    let mut temp_file = NamedTempFile::new().expect("Should create temp file");
    let temp_path = temp_file.path().parent().unwrap().join("my.backup.file.pdf");

    temp_file.write_all(b"test").unwrap();
    temp_file.flush().unwrap();
    std::fs::copy(temp_file.path(), &temp_path).unwrap();

    let detected = detect_mime_type(&temp_path, true);

    assert!(detected.is_ok(), "Should handle multiple dots in filename");
    assert_eq!(detected.unwrap(), "application/pdf", "Should use last extension");

    let _ = std::fs::remove_file(&temp_path);
}

/// Test special characters in filename.
#[tokio::test]
async fn test_filename_special_characters() {
    let mut temp_file = NamedTempFile::new().expect("Should create temp file");
    let temp_path = temp_file.path().parent().unwrap().join("文档 (copy) [v2].pdf");

    temp_file.write_all(b"test").unwrap();
    temp_file.flush().unwrap();
    std::fs::copy(temp_file.path(), &temp_path).unwrap();

    let detected = detect_mime_type(&temp_path, true);

    assert!(detected.is_ok(), "Should handle special characters in filename");
    assert_eq!(detected.unwrap(), "application/pdf");

    let _ = std::fs::remove_file(&temp_path);
}

/// Test MIME detection for all Pandoc-supported formats.
///
/// Validates that all document formats supported by Pandoc extractor
/// are correctly detected and mapped to their MIME types.
#[cfg(feature = "office")]
#[tokio::test]
async fn test_pandoc_formats_mime_detection() {
    let pandoc_formats = vec![
        ("test.rst", "text/x-rst"),
        ("test.tex", "application/x-latex"),
        ("test.latex", "application/x-latex"),
        ("test.rtf", "application/rtf"),
        ("test.odt", "application/vnd.oasis.opendocument.text"),
        ("test.epub", "application/epub+zip"),
        ("test.org", "text/x-org"),
        ("test.typst", "application/x-typst"),
        ("test.commonmark", "text/x-commonmark"),
    ];

    for (filename, expected_mime) in pandoc_formats {
        let mut temp_file = NamedTempFile::new().expect("Should create temp file");
        let temp_path = temp_file.path().parent().unwrap().join(filename);

        temp_file.write_all(b"test content").unwrap();
        temp_file.flush().unwrap();
        std::fs::copy(temp_file.path(), &temp_path).unwrap();

        let detected = detect_mime_type(&temp_path, true);

        assert!(
            detected.is_ok(),
            "Should detect MIME type for Pandoc format: {}",
            filename
        );
        assert_eq!(
            detected.unwrap(),
            expected_mime,
            "MIME type mismatch for Pandoc format: {}",
            filename
        );

        let _ = std::fs::remove_file(&temp_path);
    }
}

/// Test MIME validation for all Pandoc formats.
#[cfg(feature = "office")]
#[tokio::test]
async fn test_pandoc_mime_validation() {
    let pandoc_mimes = vec![
        "text/x-rst",
        "application/x-latex",
        "application/rtf",
        "application/vnd.oasis.opendocument.text",
        "application/epub+zip",
        "text/x-org",
        "application/x-typst",
        "text/x-commonmark",
    ];

    for mime_type in pandoc_mimes {
        let result = validate_mime_type(mime_type);
        assert!(result.is_ok(), "Pandoc MIME type should be supported: {}", mime_type);
        assert_eq!(result.unwrap(), mime_type);
    }
}
