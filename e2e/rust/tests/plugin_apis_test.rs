#![allow(clippy::too_many_lines)]

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::plugins::unregister_extractor;
use kreuzberg::plugins::{clear_extractors, list_extractors, list_post_processors};
use kreuzberg::plugins::{clear_ocr_backends, list_ocr_backends, unregister_ocr_backend};
use kreuzberg::plugins::{clear_validators, list_validators};
use kreuzberg::{detect_mime_type, detect_mime_type_from_bytes, get_extensions_for_mime};

#[test]
fn test_config_discover() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("kreuzberg.toml");
    std::fs::write(
        &config_path,
        r#"[chunking]
max_chars = 50
"#,
    )
    .expect("Failed to write config file");

    let subdir = temp_dir.path().join("subdir");
    std::fs::create_dir(&subdir).expect("Failed to create subdirectory");

    let original_dir = std::env::current_dir().expect("Failed to get current dir");
    std::env::set_current_dir(&subdir).expect("Failed to change directory");

    let config = ExtractionConfig::discover().expect("Failed to discover config");
    assert!(config.is_some());
    let config = config.unwrap();

    std::env::set_current_dir(&original_dir).expect("Failed to restore directory");

    let _ = &config.chunking;
    assert_eq!(config.chunking.as_ref().unwrap().max_chars, 50);
}

#[test]
fn test_config_from_file() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("test_config.toml");
    std::fs::write(
        &config_path,
        r#"[chunking]
max_chars = 100
max_overlap = 20

[language_detection]
enabled = false
"#,
    )
    .expect("Failed to write config file");

    let config = ExtractionConfig::from_file(&config_path).expect("Failed to load config from file");

    let _ = &config.chunking;
    assert_eq!(config.chunking.as_ref().unwrap().max_chars, 100);
    assert_eq!(config.chunking.as_ref().unwrap().max_overlap, 20);
    let _ = &config.language_detection;
    assert_eq!(config.language_detection.as_ref().unwrap().enabled, false);
}

#[test]
fn test_extractors_clear() {
    clear_extractors().expect("Failed to clear registry");
    let result = list_extractors().expect("Failed to list registry");
    assert!(result.is_empty());
}

#[test]
fn test_extractors_list() {
    let result = list_extractors().expect("Failed to list registry");
    assert!(result.iter().all(|s| !s.is_empty()));
}

#[test]
fn test_extractors_unregister() {
    unregister_extractor("nonexistent-extractor-xyz").expect("Unregister should not fail");
}

#[test]
fn test_mime_detect_bytes() {
    let data = b"%PDF-1.4\n";
    let result = detect_mime_type_from_bytes(data).expect("Failed to detect MIME type from bytes");
    assert!(result.contains("pdf"));
}

#[test]
fn test_mime_detect_path() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, "Hello, world!").expect("Failed to write file");

    let result = detect_mime_type(&file_path, true).expect("Failed to detect MIME type");
    assert!(result.contains("text"));
}

#[test]
fn test_mime_get_extensions() {
    let result = get_extensions_for_mime("application/pdf").expect("Failed to get extensions for MIME type");
    assert!(result.contains(&"pdf".to_string()));
}

#[test]
fn test_ocr_backends_clear() {
    clear_ocr_backends().expect("Failed to clear registry");
    let result = list_ocr_backends().expect("Failed to list registry");
    assert!(result.is_empty());
}

#[test]
fn test_ocr_backends_list() {
    let result = list_ocr_backends().expect("Failed to list registry");
    assert!(result.iter().all(|s| !s.is_empty()));
}

#[test]
fn test_ocr_backends_unregister() {
    unregister_ocr_backend("nonexistent-backend-xyz").expect("Unregister should not fail");
}

#[test]
fn test_post_processors_clear() {
    let registry = kreuzberg::plugins::registry::get_post_processor_registry();
    let mut registry = registry.write().expect("Failed to acquire write lock");
    registry.shutdown_all().expect("Failed to clear registry");
    drop(registry);

    let result = list_post_processors().expect("Failed to list registry");
    assert!(result.is_empty());
}

#[test]
fn test_post_processors_list() {
    let result = list_post_processors().expect("Failed to list registry");
    assert!(result.iter().all(|s| !s.is_empty()));
}

#[test]
fn test_validators_clear() {
    clear_validators().expect("Failed to clear registry");
    let result = list_validators().expect("Failed to list registry");
    assert!(result.is_empty());
}

#[test]
fn test_validators_list() {
    let result = list_validators().expect("Failed to list registry");
    assert!(result.iter().all(|s| !s.is_empty()));
}
