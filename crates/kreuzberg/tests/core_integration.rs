//! Integration tests for core extraction functionality.
//!
//! These tests verify the end-to-end behavior of the extraction pipeline,
//! config loading, MIME detection, and batch processing.

use kreuzberg::{
    extract_file, extract_bytes, batch_extract_file, batch_extract_bytes,
    extract_file_sync, extract_bytes_sync, batch_extract_file_sync, batch_extract_bytes_sync,
    ExtractionConfig, detect_mime_type, validate_mime_type,
};
use std::fs::{self, File};
use std::io::Write;
use tempfile::tempdir;

/// Test basic file extraction with MIME detection.
#[tokio::test]
async fn test_extract_file_basic() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.txt");
    let mut file = File::create(&file_path).unwrap();
    file.write_all(b"Hello, Kreuzberg!").unwrap();

    let config = ExtractionConfig::default();
    let result = extract_file(&file_path, None, &config).await;

    assert!(result.is_ok());
    let result = result.unwrap();
    assert_eq!(result.content, "Hello, Kreuzberg!");
    assert_eq!(result.mime_type, "text/plain");
}

/// Test extraction with explicit MIME type override.
#[tokio::test]
async fn test_extract_file_with_mime_override() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("data.bin");
    let mut file = File::create(&file_path).unwrap();
    file.write_all(b"Binary content").unwrap();

    let config = ExtractionConfig::default();
    let result = extract_file(&file_path, Some("text/plain"), &config).await;

    assert!(result.is_ok());
    let result = result.unwrap();
    assert_eq!(result.mime_type, "text/plain");
}

/// Test extraction of multiple file types.
#[tokio::test]
async fn test_extract_multiple_file_types() {
    let dir = tempdir().unwrap();
    let config = ExtractionConfig::default();

    // Create test files of different types
    let test_files: Vec<(&str, &[u8], &str)> = vec![
        ("test.txt", b"text content", "text/plain"),
        ("test.json", b"{\"key\": \"value\"}", "application/json"),
        ("test.xml", b"<root>data</root>", "application/xml"),
        ("test.html", b"<html><body>test</body></html>", "text/html"),
    ];

    for (filename, content, expected_mime) in test_files {
        let file_path = dir.path().join(filename);
        fs::write(&file_path, content).unwrap();

        let result = extract_file(&file_path, None, &config).await.unwrap();
        assert_eq!(result.mime_type, expected_mime, "Failed for {}", filename);
        assert!(!result.content.is_empty());
    }
}

/// Test extract_bytes with various MIME types.
#[tokio::test]
async fn test_extract_bytes_various_mime_types() {
    let config = ExtractionConfig::default();

    let test_cases: Vec<(&[u8], &str)> = vec![
        (b"text content", "text/plain"),
        (b"{\"key\": \"value\"}", "application/json"),
        (b"<root>data</root>", "application/xml"),
    ];

    for (content, mime_type) in test_cases {
        let result = extract_bytes(content, mime_type, &config).await;
        assert!(result.is_ok(), "Failed for MIME type: {}", mime_type);
        let result = result.unwrap();
        assert_eq!(result.mime_type, mime_type);
    }
}

/// Test batch extraction with concurrent processing.
#[tokio::test]
async fn test_batch_extract_file_concurrency() {
    let dir = tempdir().unwrap();
    let config = ExtractionConfig::default();

    // Create multiple test files
    let num_files = 10;
    let mut paths = Vec::new();

    for i in 0..num_files {
        let file_path = dir.path().join(format!("test_{}.txt", i));
        fs::write(&file_path, format!("Content {}", i)).unwrap();
        paths.push(file_path);
    }

    // Extract all files concurrently
    let results = batch_extract_file(paths.clone(), &config).await;
    assert!(results.is_ok());

    let results = results.unwrap();
    assert_eq!(results.len(), num_files);

    // Verify results are in correct order
    for (i, result) in results.iter().enumerate() {
        assert!(result.content.contains(&i.to_string()));
        assert_eq!(result.mime_type, "text/plain");
    }
}

/// Test batch extraction with empty input.
#[tokio::test]
async fn test_batch_extract_empty() {
    let config = ExtractionConfig::default();
    let paths: Vec<std::path::PathBuf> = vec![];

    let results = batch_extract_file(paths, &config).await;
    assert!(results.is_ok());
    assert_eq!(results.unwrap().len(), 0);
}

/// Test batch_extract_bytes with concurrent processing.
#[tokio::test]
async fn test_batch_extract_bytes_concurrency() {
    let config = ExtractionConfig::default();

    let contents = vec![
        (b"content 1".as_slice(), "text/plain"),
        (b"content 2".as_slice(), "text/plain"),
        (b"content 3".as_slice(), "text/plain"),
        (b"content 4".as_slice(), "text/plain"),
        (b"content 5".as_slice(), "text/plain"),
    ];

    let results = batch_extract_bytes(contents, &config).await;
    assert!(results.is_ok());

    let results = results.unwrap();
    assert_eq!(results.len(), 5);

    // Verify results are in correct order
    for (i, result) in results.iter().enumerate() {
        let expected_content = format!("content {}", i + 1);
        assert_eq!(result.content, expected_content);
    }
}

/// Test sync wrappers for extraction functions.
#[test]
fn test_sync_wrappers() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("sync_test.txt");
    fs::write(&file_path, "sync content").unwrap();

    let config = ExtractionConfig::default();

    // Test sync file extraction
    let result = extract_file_sync(&file_path, None, &config);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().content, "sync content");

    // Test sync bytes extraction
    let result = extract_bytes_sync(b"test bytes", "text/plain", &config);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().content, "test bytes");

    // Test batch sync
    let paths = vec![file_path];
    let results = batch_extract_file_sync(paths, &config);
    assert!(results.is_ok());
    assert_eq!(results.unwrap().len(), 1);

    // Test batch bytes sync
    let contents = vec![(b"test".as_slice(), "text/plain")];
    let results = batch_extract_bytes_sync(contents, &config);
    assert!(results.is_ok());
    assert_eq!(results.unwrap().len(), 1);
}

/// Test MIME type detection for various extensions.
#[test]
fn test_mime_detection_comprehensive() {
    let dir = tempdir().unwrap();

    let test_cases = vec![
        // Text formats
        ("test.txt", "text/plain"),
        ("test.md", "text/markdown"),
        ("test.html", "text/html"),
        // Data formats
        ("test.json", "application/json"),
        ("test.yaml", "application/x-yaml"),
        ("test.toml", "application/toml"),
        ("test.xml", "application/xml"),
        // Office documents
        ("test.pdf", "application/pdf"),
        ("test.xlsx", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
        ("test.docx", "application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
        ("test.pptx", "application/vnd.openxmlformats-officedocument.presentationml.presentation"),
        // Images
        ("test.png", "image/png"),
        ("test.jpg", "image/jpeg"),
        ("test.gif", "image/gif"),
        // Email
        ("test.eml", "message/rfc822"),
    ];

    for (filename, expected_mime) in test_cases {
        let file_path = dir.path().join(filename);
        File::create(&file_path).unwrap();

        let detected = detect_mime_type(&file_path, true).unwrap();
        assert_eq!(detected, expected_mime, "Failed for {}", filename);

        // Also test validation
        let validated = validate_mime_type(&detected);
        assert!(validated.is_ok(), "Validation failed for {}", expected_mime);
    }
}

/// Test MIME type validation.
#[test]
fn test_mime_validation() {
    // Valid MIME types
    assert!(validate_mime_type("application/pdf").is_ok());
    assert!(validate_mime_type("text/plain").is_ok());
    assert!(validate_mime_type("image/png").is_ok());
    assert!(validate_mime_type("image/custom-format").is_ok()); // Any image/* is valid

    // Invalid MIME types
    assert!(validate_mime_type("video/mp4").is_err());
    assert!(validate_mime_type("application/unknown").is_err());
}

/// Test case-insensitive extension handling.
#[test]
fn test_case_insensitive_extensions() {
    let dir = tempdir().unwrap();

    let test_cases = vec![
        ("test.PDF", "application/pdf"),
        ("test.TXT", "text/plain"),
        ("test.Json", "application/json"),
        ("test.XLSX", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
    ];

    for (filename, expected_mime) in test_cases {
        let file_path = dir.path().join(filename);
        File::create(&file_path).unwrap();

        let detected = detect_mime_type(&file_path, true).unwrap();
        assert_eq!(detected, expected_mime, "Failed for {}", filename);
    }
}

/// Test config loading from TOML file.
#[test]
fn test_config_loading() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("kreuzberg.toml");

    // Write a V4 config
    fs::write(&config_path, r#"
use_cache = false
enable_quality_processing = true
force_ocr = false

[ocr]
backend = "tesseract"
language = "deu"

[chunking]
max_chars = 2000
max_overlap = 300
    "#).unwrap();

    let config = ExtractionConfig::from_toml_file(&config_path).unwrap();

    assert!(!config.use_cache);
    assert!(config.enable_quality_processing);
    assert!(!config.force_ocr);

    let ocr_config = config.ocr.unwrap();
    assert_eq!(ocr_config.backend, "tesseract");
    assert_eq!(ocr_config.language, "deu");

    let chunking_config = config.chunking.unwrap();
    assert_eq!(chunking_config.max_chars, 2000);
    assert_eq!(chunking_config.max_overlap, 300);
}

/// Test config discovery in parent directories.
#[test]
fn test_config_discovery() {
    let dir = tempdir().unwrap();
    let subdir = dir.path().join("subdir");
    fs::create_dir(&subdir).unwrap();

    // Create config in parent directory
    let config_path = dir.path().join("kreuzberg.toml");
    fs::write(&config_path, r#"
use_cache = false
enable_quality_processing = true
    "#).unwrap();

    // Change to subdirectory
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&subdir).unwrap();

    // Discovery should find config in parent
    let config = ExtractionConfig::discover().unwrap();
    assert!(config.is_some());
    assert!(!config.unwrap().use_cache);

    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();
}

/// Test error handling for nonexistent files.
#[tokio::test]
async fn test_nonexistent_file_error() {
    let config = ExtractionConfig::default();
    let result = extract_file("/nonexistent/file.txt", None, &config).await;

    assert!(result.is_err());
    // Should be a validation error, not IO error (file existence check happens first)
    assert!(matches!(result.unwrap_err(), kreuzberg::KreuzbergError::Validation(_)));
}

/// Test error handling for unsupported MIME types.
#[tokio::test]
async fn test_unsupported_mime_type_error() {
    let config = ExtractionConfig::default();
    let result = extract_bytes(b"test", "video/mp4", &config).await;

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), kreuzberg::KreuzbergError::UnsupportedFormat(_)));
}

/// Test pipeline execution (currently stub, will be expanded in Phase 2).
#[tokio::test]
async fn test_pipeline_execution() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("pipeline_test.txt");
    fs::write(&file_path, "pipeline content").unwrap();

    let mut config = ExtractionConfig::default();
    config.enable_quality_processing = true;

    let result = extract_file(&file_path, None, &config).await;
    assert!(result.is_ok());

    // Pipeline is currently a pass-through, but should not error
    let result = result.unwrap();
    assert_eq!(result.content, "pipeline content");
}

/// Test extraction with OCR config (placeholder test for Phase 2).
#[tokio::test]
async fn test_extraction_with_ocr_config() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("ocr_test.txt");
    fs::write(&file_path, "ocr content").unwrap();

    let mut config = ExtractionConfig::default();
    config.ocr = Some(kreuzberg::OcrConfig {
        backend: "tesseract".to_string(),
        language: "eng".to_string(),
    });
    config.force_ocr = true;

    // Should not error even with OCR config (will be used in Phase 3)
    let result = extract_file(&file_path, None, &config).await;
    assert!(result.is_ok());
}

/// Test extraction with chunking config (placeholder test for Phase 2).
#[tokio::test]
async fn test_extraction_with_chunking_config() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("chunking_test.txt");
    fs::write(&file_path, "content for chunking").unwrap();

    let mut config = ExtractionConfig::default();
    config.chunking = Some(kreuzberg::ChunkingConfig {
        max_chars: 1000,
        max_overlap: 200,
    });

    // Should not error even with chunking config (will be used in Phase 2)
    let result = extract_file(&file_path, None, &config).await;
    assert!(result.is_ok());
}
