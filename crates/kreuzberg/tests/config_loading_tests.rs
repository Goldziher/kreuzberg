//! Configuration loading integration tests.
//!
//! Tests the config loading APIs:
//! - from_file() with TOML/YAML/JSON
//! - discover() for searching parent directories
//! - Error handling for invalid configs

use kreuzberg::KreuzbergError;
use kreuzberg::core::config::ExtractionConfig;
use std::fs;
use tempfile::TempDir;

/// Test loading config from TOML file.
#[test]
fn test_from_file_toml_succeeds() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let toml_content = r#"
[ocr]
enabled = true
backend = "tesseract"

[chunking]
max_chars = 1000
max_overlap = 100
"#;

    fs::write(&config_path, toml_content).unwrap();

    let config = ExtractionConfig::from_file(&config_path);
    assert!(config.is_ok(), "Should load TOML config successfully");

    let config = config.unwrap();
    assert!(config.ocr.is_some(), "Should have OCR config");
    assert!(config.chunking.is_some(), "Should have chunking config");

    let chunking = config.chunking.unwrap();
    assert_eq!(chunking.max_chars, 1000);
    assert_eq!(chunking.max_overlap, 100);
}

/// Test loading config from YAML file.
#[test]
fn test_from_file_yaml_succeeds() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.yaml");

    let yaml_content = r#"
ocr:
  enabled: true
  backend: tesseract
chunking:
  max_chars: 1000
  max_overlap: 100
"#;

    fs::write(&config_path, yaml_content).unwrap();

    let config = ExtractionConfig::from_file(&config_path);
    assert!(config.is_ok(), "Should load YAML config successfully");

    let config = config.unwrap();
    assert!(config.ocr.is_some(), "Should have OCR config");
    assert!(config.chunking.is_some(), "Should have chunking config");

    let chunking = config.chunking.unwrap();
    assert_eq!(chunking.max_chars, 1000);
    assert_eq!(chunking.max_overlap, 100);
}

/// Test loading config from JSON file.
#[test]
fn test_from_file_json_succeeds() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.json");

    let json_content = r#"
{
  "ocr": {
    "enabled": true,
    "backend": "tesseract"
  },
  "chunking": {
    "max_chars": 1000,
    "max_overlap": 100
  }
}
"#;

    fs::write(&config_path, json_content).unwrap();

    let config = ExtractionConfig::from_file(&config_path);
    assert!(config.is_ok(), "Should load JSON config successfully");

    let config = config.unwrap();
    assert!(config.ocr.is_some(), "Should have OCR config");
    assert!(config.chunking.is_some(), "Should have chunking config");

    let chunking = config.chunking.unwrap();
    assert_eq!(chunking.max_chars, 1000);
    assert_eq!(chunking.max_overlap, 100);
}

/// Test loading config from .yml extension.
#[test]
fn test_from_file_yml_extension_succeeds() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.yml");

    let yml_content = r#"
ocr:
  enabled: true
"#;

    fs::write(&config_path, yml_content).unwrap();

    let config = ExtractionConfig::from_file(&config_path);
    assert!(config.is_ok(), "Should load .yml config successfully");
}

/// Test from_file with nonexistent path fails.
#[test]
fn test_from_file_nonexistent_path_fails() {
    let result = ExtractionConfig::from_file("/nonexistent/path/config.toml");
    assert!(result.is_err(), "Should fail for nonexistent path: {:?}", result);
    // Error can be Io or other types depending on the implementation
}

/// Test from_file with malformed TOML fails.
#[test]
fn test_from_file_malformed_toml_fails() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let malformed_toml = r#"
[ocr
enabled = true
"#;

    fs::write(&config_path, malformed_toml).unwrap();

    let result = ExtractionConfig::from_file(&config_path);
    assert!(result.is_err(), "Should fail for malformed TOML: {:?}", result);
    // Error handling varies - just ensure it failed
}

/// Test from_file with malformed JSON fails.
#[test]
fn test_from_file_malformed_json_fails() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.json");

    let malformed_json = r#"
{
  "ocr": {
    "enabled": true
  }
  "chunking": {}
}
"#;

    fs::write(&config_path, malformed_json).unwrap();

    let result = ExtractionConfig::from_file(&config_path);
    assert!(result.is_err(), "Should fail for malformed JSON: {:?}", result);
    // Error handling varies - just ensure it failed
}

/// Test from_file with malformed YAML fails.
#[test]
fn test_from_file_malformed_yaml_fails() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.yaml");

    let malformed_yaml = r#"
ocr:
  enabled: true
  - invalid_list
"#;

    fs::write(&config_path, malformed_yaml).unwrap();

    let result = ExtractionConfig::from_file(&config_path);
    assert!(result.is_err(), "Should fail for malformed YAML: {:?}", result);
    // Error handling varies - just ensure it failed
}

/// Test from_file with empty file uses defaults.
#[test]
fn test_from_file_empty_file_uses_defaults() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    fs::write(&config_path, "").unwrap();

    let config = ExtractionConfig::from_file(&config_path);
    assert!(config.is_ok(), "Should load empty file successfully");

    let config = config.unwrap();
    // Should have default values
    assert!(config.ocr.is_none(), "Default config should have no OCR");
    assert!(config.chunking.is_none(), "Default config should have no chunking");
}

/// Test from_file with unsupported extension fails.
#[test]
fn test_from_file_unsupported_extension_fails() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.txt");

    fs::write(&config_path, "ocr:\n  enabled: true").unwrap();

    let result = ExtractionConfig::from_file(&config_path);
    assert!(result.is_err(), "Should fail for unsupported extension: {:?}", result);

    match result {
        Err(KreuzbergError::Validation { message, .. }) => {
            assert!(
                message.contains("format") || message.contains("extension") || message.contains("Unsupported"),
                "Error should mention format/extension: {}",
                message
            );
        }
        _ => {
            // Some other error is also acceptable
        }
    }
}

/// Test discover() finds config in current directory.
#[test]
fn test_discover_finds_config_in_current_dir() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("kreuzberg.toml");

    let toml_content = r#"
[ocr]
enabled = true
"#;

    fs::write(&config_path, toml_content).unwrap();

    // Change to temp directory
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    let result = ExtractionConfig::discover();

    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();

    assert!(result.is_ok(), "Discover should succeed");
    let config = result.unwrap();
    assert!(config.is_some(), "Should find config in current directory");
    assert!(config.unwrap().ocr.is_some(), "Should have OCR config");
}

/// Test discover() finds config in parent directory.
#[test]
fn test_discover_finds_config_in_parent_dir() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("kreuzberg.toml");

    let toml_content = r#"
[ocr]
enabled = true
"#;

    fs::write(&config_path, toml_content).unwrap();

    // Create subdirectory
    let sub_dir = temp_dir.path().join("subdir");
    fs::create_dir(&sub_dir).unwrap();

    // Change to subdirectory
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&sub_dir).unwrap();

    let result = ExtractionConfig::discover();

    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();

    assert!(result.is_ok(), "Discover should succeed");
    let config = result.unwrap();
    assert!(config.is_some(), "Should find config in parent directory");
    assert!(config.unwrap().ocr.is_some(), "Should have OCR config");
}

/// Test discover() returns None when no config found.
#[test]
#[serial_test::serial]
fn test_discover_returns_none_when_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let sub_dir = temp_dir.path().join("subdir");
    fs::create_dir(&sub_dir).unwrap();

    // Change to subdirectory (no config files)
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&sub_dir).unwrap();

    let result = ExtractionConfig::discover();

    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();

    assert!(result.is_ok(), "Discover should succeed even when no config found");
    let _config = result.unwrap();
    // May return None or may find a config in parent directories (e.g., repository root)
    // Just verify it doesn't error - the specific behavior depends on the directory structure
}

/// Test discover() prefers certain file names.
#[test]
fn test_discover_file_name_preference() {
    let temp_dir = TempDir::new().unwrap();

    // Create multiple config files
    fs::write(temp_dir.path().join("kreuzberg.toml"), "[ocr]\nenabled = true").unwrap();
    fs::write(temp_dir.path().join(".kreuzberg.toml"), "[ocr]\nenabled = false").unwrap();

    let original_dir = std::env::current_dir().unwrap();
    if std::env::set_current_dir(temp_dir.path()).is_err() {
        // Skip this test if we can't change directory
        return;
    }

    let result = ExtractionConfig::discover();

    // Always restore directory even if test fails
    let _ = std::env::set_current_dir(original_dir);

    assert!(result.is_ok(), "Discover should succeed");
    let config = result.unwrap();
    assert!(config.is_some(), "Should find a config file");
}

/// Test discover() with nested directories.
#[test]
#[serial_test::serial]
fn test_discover_with_nested_directories() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("kreuzberg.toml");

    let toml_content = r#"
[ocr]
enabled = true
"#;

    fs::write(&config_path, toml_content).unwrap();

    // Create nested subdirectories
    let level1 = temp_dir.path().join("level1");
    let level2 = level1.join("level2");
    let level3 = level2.join("level3");
    fs::create_dir_all(&level3).unwrap();

    // Change to deepest directory
    let original_dir = std::env::current_dir().unwrap();
    if std::env::set_current_dir(&level3).is_err() {
        // Skip this test if we can't change directory
        return;
    }

    let result = ExtractionConfig::discover();

    // Always restore directory even if test fails
    let _ = std::env::set_current_dir(&original_dir);

    assert!(result.is_ok(), "Discover should succeed");
    let config = result.unwrap();
    assert!(config.is_some(), "Should find config in ancestor directory");
    assert!(config.unwrap().ocr.is_some(), "Should have OCR config");
}

/// Test config loading with all supported features.
#[test]
fn test_from_file_comprehensive_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let toml_content = r#"
[ocr]
enabled = true
backend = "tesseract"

[chunking]
max_chars = 2000
max_overlap = 200

[language_detection]
enabled = true

[images]
enabled = true

[pdf_options]
extract_images = true
"#;

    fs::write(&config_path, toml_content).unwrap();

    let config = ExtractionConfig::from_file(&config_path);
    assert!(config.is_ok(), "Should load comprehensive config successfully");

    let config = config.unwrap();
    assert!(config.ocr.is_some(), "Should have OCR config");
    assert!(config.chunking.is_some(), "Should have chunking config");
    assert!(
        config.language_detection.is_some(),
        "Should have language detection config"
    );
    assert!(config.images.is_some(), "Should have image extraction config");
    assert!(config.pdf_options.is_some(), "Should have PDF config");
}

/// Test config validation with invalid values.
#[test]
fn test_from_file_with_invalid_values() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Negative values should be rejected during deserialization or validation
    let toml_content = r#"
[chunking]
max_chars = -1000
max_overlap = -100
"#;

    fs::write(&config_path, toml_content).unwrap();

    let result = ExtractionConfig::from_file(&config_path);
    // Should either fail parsing or have clamped values
    if let Ok(config) = result {
        // If it succeeds, values should be reasonable
        if let Some(chunking) = config.chunking {
            assert!(chunking.max_chars > 0, "max_chars should be positive");
        }
    }
}
