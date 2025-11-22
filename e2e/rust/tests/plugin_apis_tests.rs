// Auto-generated tests for plugin API fixtures.
#![allow(clippy::too_many_lines)]

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::plugins::{list_validators, clear_validators};
use kreuzberg::plugins::{list_post_processors, clear_extractors, list_extractors};
use kreuzberg::plugins::{list_ocr_backends, clear_ocr_backends, unregister_ocr_backend};
use kreuzberg::plugins::{unregister_extractor};
use kreuzberg::{detect_mime_type, detect_mime_type_from_bytes, get_extensions_for_mime};

#[test]
fn test_config_discover() {
    // Discover configuration from current or parent directories

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("kreuzberg.toml");
    std::fs::write(&config_path, r#"[chunking]
max_chars = 50
"#).expect("Failed to write config file");

    let subdir = temp_dir.path().join("subdir");
    std::fs::create_dir(&subdir).expect("Failed to create subdirectory");

    let original_dir = std::env::current_dir().expect("Failed to get current dir");
    std::env::set_current_dir(&subdir).expect("Failed to change directory");

    let config = ExtractionConfig::discover()
        .expect("Failed to discover config");
    assert!(config.is_some());
    let config = config.unwrap();

    // Restore original directory
    std::env::set_current_dir(&original_dir).expect("Failed to restore directory");

    // Verify chunking exists
    let _ = &config.chunking;
    assert_eq!(config.chunking.as_ref().unwrap().max_chars, 50);
}

#[test]
fn test_config_from_file() {
    // Load configuration from a TOML file

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("test_config.toml");
    std::fs::write(&config_path, r#"[chunking]
max_chars = 100
max_overlap = 20

[language_detection]
enabled = false
"#).expect("Failed to write config file");

    let config = ExtractionConfig::from_file(&config_path)
        .expect("Failed to load config from file");

    // Verify chunking exists
    let _ = &config.chunking;
    assert_eq!(config.chunking.as_ref().unwrap().max_chars, 100);
    assert_eq!(config.chunking.as_ref().unwrap().max_overlap, 20);
    // Verify language_detection exists
    let _ = &config.language_detection;
    assert_eq!(config.language_detection.as_ref().unwrap().enabled, false);
}

#[test]
fn test_extractors_clear() {
    // Clear all document extractors and verify list is empty

    clear_extractors().expect("Failed to clear registry");
    let result = list_extractors().expect("Failed to list registry");
    assert!(result.is_empty());
}

#[test]
fn test_extractors_list() {
    // List all registered document extractors

    let result = list_extractors().expect("Failed to list registry");
    assert!(result.iter().all(|s| !s.is_empty()));
}

#[test]
fn test_extractors_unregister() {
    // Unregister nonexistent document extractor gracefully

    unregister_extractor("nonexistent-extractor-xyz").expect("Unregister should not fail");
}

#[test]
fn test_mime_detect_bytes() {
    // Detect MIME type from file bytes

    let data = b"%PDF-1.4\n";
    let result = detect_mime_type_from_bytes(data)
        .expect("Failed to detect MIME type from bytes");
    assert!(result.contains("pdf"));
}

#[test]
fn test_mime_detect_path() {
    // Detect MIME type from file path

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, "Hello, world!").expect("Failed to write file");

    let result = detect_mime_type(&file_path, true)
        .expect("Failed to detect MIME type");
    assert!(result.contains("text"));
}

#[test]
fn test_mime_get_extensions() {
    // Get file extensions for a MIME type

    let result = get_extensions_for_mime("application/pdf")
        .expect("Failed to get extensions for MIME type");
    assert!(result.contains(&"pdf".to_string()));
}

#[test]
fn test_ocr_backends_clear() {
    // Clear all OCR backends and verify list is empty

    clear_ocr_backends().expect("Failed to clear registry");
    let result = list_ocr_backends().expect("Failed to list registry");
    assert!(result.is_empty());
}

#[test]
fn test_ocr_backends_list() {
    // List all registered OCR backends

    let result = list_ocr_backends().expect("Failed to list registry");
    assert!(result.iter().all(|s| !s.is_empty()));
}

#[test]
fn test_ocr_backends_unregister() {
    // Unregister nonexistent OCR backend gracefully

    unregister_ocr_backend("nonexistent-backend-xyz").expect("Unregister should not fail");
}

#[test]
fn test_post_processors_clear() {
    // Clear all post-processors and verify list is empty

    // Clear post-processors via registry (no helper function)
    let registry = kreuzberg::plugins::registry::get_post_processor_registry();
    let mut registry = registry.write().expect("Failed to acquire write lock");
    registry.shutdown_all().expect("Failed to clear registry");
    drop(registry);

    let result = list_post_processors().expect("Failed to list registry");
    assert!(result.is_empty());
}

#[test]
fn test_post_processors_list() {
    // List all registered post-processors

    let result = list_post_processors().expect("Failed to list registry");
    assert!(result.iter().all(|s| !s.is_empty()));
}

#[test]
fn test_validators_clear() {
    // Clear all validators and verify list is empty

    clear_validators().expect("Failed to clear registry");
    let result = list_validators().expect("Failed to list registry");
    assert!(result.is_empty());
}

#[test]
fn test_validators_list() {
    // List all registered validators

    let result = list_validators().expect("Failed to list registry");
    assert!(result.iter().all(|s| !s.is_empty()));
}

