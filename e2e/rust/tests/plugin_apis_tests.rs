// Auto-generated tests for plugin API fixtures.
#![allow(clippy::too_many_lines)]

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::{list_validators, clear_validators};
use kreuzberg::{list_post_processors, clear_post_processors};
use kreuzberg::{list_ocr_backends, clear_ocr_backends, unregister_ocr_backend};
use kreuzberg::{list_document_extractors, clear_document_extractors, unregister_document_extractor};
use kreuzberg::{detect_mime_type, detect_mime_type_from_path, get_extensions_for_mime};
use std::path::Path;

#[test]
fn test_config_discover() {
    // Discover configuration from current or parent directories

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("kreuzberg.toml");
    std::fs::write(&config_path, r#"[chunking]\nmax_chars = 50\n"#).expect("Failed to write config file");

    let subdir = temp_dir.path().join("subdir");
    std::fs::create_dir(&subdir).expect("Failed to create subdirectory");

    let _guard = temp_cwd::TempCwd::new(&subdir).expect("Failed to change directory");
    let config = ExtractionConfig::discover()
        .expect("Failed to discover config");
    assert!(config.is_some());
    let config = config.unwrap();

    // Verify chunking exists
    let _ = &config.chunking;
    assert_eq!(config.chunking.max_chars, 50);
}

#[test]
fn test_config_from_file() {
    // Load configuration from a TOML file

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("test_config.toml");
    std::fs::write(&config_path, r#"[chunking]\nmax_chars = 100\nmax_overlap = 20\n\n[language_detection]\nenabled = false\n"#).expect("Failed to write config file");

    let config = ExtractionConfig::from_file(&config_path)
        .expect("Failed to load config from file");

    // Verify chunking exists
    let _ = &config.chunking;
    assert_eq!(config.chunking.max_chars, 100);
    assert_eq!(config.chunking.max_overlap, 20);
    // Verify language_detection exists
    let _ = &config.language_detection;
    assert_eq!(config.language_detection.enabled, false);
}

#[test]
fn test_extractors_clear() {
    // Clear all document extractors and verify list is empty

    clear_document_extractors();
    let result = list_document_extractors();
    assert!(result.is_empty());
}

#[test]
fn test_extractors_list() {
    // List all registered document extractors

    let result = list_document_extractors();
    assert!(result.iter().all(|s| !s.is_empty()));
}

#[test]
fn test_extractors_unregister() {
    // Unregister nonexistent document extractor gracefully

    unregister_document_extractor("nonexistent-extractor-xyz");
}

#[test]
fn test_mime_detect_bytes() {
    // Detect MIME type from file bytes

    let data = hex::decode("%PDF-1.4\n").expect("Failed to decode hex");
    let result = detect_mime_type(&data);
    assert!(result.contains("pdf"));
}

#[test]
fn test_mime_detect_path() {
    // Detect MIME type from file path

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, "Hello, world!").expect("Failed to write file");

    let result = detect_mime_type_from_path(&file_path)
        .expect("Failed to detect MIME type");
    assert!(result.contains("text"));
}

#[test]
fn test_mime_get_extensions() {
    // Get file extensions for a MIME type

    let result = get_extensions_for_mime("application/pdf");
    assert!(result.contains(&"pdf".to_string()));
}

#[test]
fn test_ocr_backends_clear() {
    // Clear all OCR backends and verify list is empty

    clear_ocr_backends();
    let result = list_ocr_backends();
    assert!(result.is_empty());
}

#[test]
fn test_ocr_backends_list() {
    // List all registered OCR backends

    let result = list_ocr_backends();
    assert!(result.iter().all(|s| !s.is_empty()));
}

#[test]
fn test_ocr_backends_unregister() {
    // Unregister nonexistent OCR backend gracefully

    unregister_ocr_backend("nonexistent-backend-xyz");
}

#[test]
fn test_post_processors_clear() {
    // Clear all post-processors and verify list is empty

    clear_post_processors();
    let result = list_post_processors();
    assert!(result.is_empty());
}

#[test]
fn test_post_processors_list() {
    // List all registered post-processors

    let result = list_post_processors();
    assert!(result.iter().all(|s| !s.is_empty()));
}

#[test]
fn test_validators_clear() {
    // Clear all validators and verify list is empty

    clear_validators();
    let result = list_validators();
    assert!(result.is_empty());
}

#[test]
fn test_validators_list() {
    // List all registered validators

    let result = list_validators();
    assert!(result.iter().all(|s| !s.is_empty()));
}

