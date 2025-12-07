//! FFI config loading integration tests.
//!
//! Tests the FFI layer for configuration loading from files and discovery.

use std::ffi::{CStr, CString};
use std::fs;
use std::os::raw::c_char;
use tempfile::TempDir;

unsafe extern "C" {
    fn kreuzberg_load_extraction_config_from_file(file_path: *const c_char) -> *mut c_char;
    fn kreuzberg_free_string(s: *mut c_char);
    fn kreuzberg_last_error() -> *const c_char;
}

/// Helper to convert *const c_char to String
unsafe fn c_str_to_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        unsafe { Some(CStr::from_ptr(ptr).to_string_lossy().into_owned()) }
    }
}

/// Helper to get last error message
unsafe fn get_last_error() -> Option<String> {
    let error_ptr = unsafe { kreuzberg_last_error() };
    unsafe { c_str_to_string(error_ptr) }
}

/// Test successful config loading from TOML file.
#[test]
fn test_load_config_from_toml_file_succeeds() {
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

    unsafe {
        let path_cstr = CString::new(config_path.to_str().unwrap()).unwrap();
        let result = kreuzberg_load_extraction_config_from_file(path_cstr.as_ptr());

        assert!(!result.is_null(), "Result should not be null");

        let json_str = c_str_to_string(result).expect("Should have valid JSON");
        kreuzberg_free_string(result);

        let json_value: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert!(json_value.get("ocr").is_some(), "Should have OCR config");
        assert!(json_value.get("chunking").is_some(), "Should have chunking config");
    }
}

/// Test successful config loading from YAML file.
#[test]
fn test_load_config_from_yaml_file_succeeds() {
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

    unsafe {
        let path_cstr = CString::new(config_path.to_str().unwrap()).unwrap();
        let result = kreuzberg_load_extraction_config_from_file(path_cstr.as_ptr());

        assert!(!result.is_null(), "Result should not be null");

        let json_str = c_str_to_string(result).expect("Should have valid JSON");
        kreuzberg_free_string(result);

        let json_value: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert!(json_value.get("ocr").is_some(), "Should have OCR config");
        assert!(json_value.get("chunking").is_some(), "Should have chunking config");
    }
}

/// Test successful config loading from JSON file.
#[test]
fn test_load_config_from_json_file_succeeds() {
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

    unsafe {
        let path_cstr = CString::new(config_path.to_str().unwrap()).unwrap();
        let result = kreuzberg_load_extraction_config_from_file(path_cstr.as_ptr());

        assert!(!result.is_null(), "Result should not be null");

        let json_str = c_str_to_string(result).expect("Should have valid JSON");
        kreuzberg_free_string(result);

        let json_value: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert!(json_value.get("ocr").is_some(), "Should have OCR config");
        assert!(json_value.get("chunking").is_some(), "Should have chunking config");
    }
}

/// Test config loading fails gracefully with invalid file path.
#[test]
fn test_load_config_from_invalid_path_fails_gracefully() {
    unsafe {
        let invalid_path = CString::new("/nonexistent/path/config.toml").unwrap();
        let result = kreuzberg_load_extraction_config_from_file(invalid_path.as_ptr());

        assert!(result.is_null(), "Result should be null for invalid path");

        let error = get_last_error();
        assert!(error.is_some(), "Should have error message");
        let error_msg = error.unwrap();
        assert!(
            error_msg.contains("No such file") || error_msg.contains("not found"),
            "Error should mention file not found: {}",
            error_msg
        );
    }
}

/// Test config loading fails gracefully with null pointer.
#[test]
fn test_load_config_from_null_pointer_fails_gracefully() {
    unsafe {
        let result = kreuzberg_load_extraction_config_from_file(std::ptr::null());

        assert!(result.is_null(), "Result should be null for null pointer");

        let error = get_last_error();
        assert!(error.is_some(), "Should have error message");
        let error_msg = error.unwrap();
        assert!(
            error_msg.contains("null") || error_msg.contains("invalid"),
            "Error should mention null/invalid: {}",
            error_msg
        );
    }
}

/// Test config loading fails gracefully with malformed TOML.
#[test]
fn test_load_config_from_malformed_toml_fails_gracefully() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let malformed_toml = r#"
[ocr
enabled = true
"#;

    fs::write(&config_path, malformed_toml).unwrap();

    unsafe {
        let path_cstr = CString::new(config_path.to_str().unwrap()).unwrap();
        let result = kreuzberg_load_extraction_config_from_file(path_cstr.as_ptr());

        assert!(result.is_null(), "Result should be null for malformed TOML");

        let error = get_last_error();
        assert!(error.is_some(), "Should have error message");
        let error_msg = error.unwrap();
        assert!(
            error_msg.contains("parse") || error_msg.contains("invalid") || error_msg.contains("TOML"),
            "Error should mention parsing issue: {}",
            error_msg
        );
    }
}

/// Test config loading fails gracefully with malformed JSON.
#[test]
fn test_load_config_from_malformed_json_fails_gracefully() {
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

    unsafe {
        let path_cstr = CString::new(config_path.to_str().unwrap()).unwrap();
        let result = kreuzberg_load_extraction_config_from_file(path_cstr.as_ptr());

        assert!(result.is_null(), "Result should be null for malformed JSON");

        let error = get_last_error();
        assert!(error.is_some(), "Should have error message");
        let error_msg = error.unwrap();
        assert!(
            error_msg.contains("parse") || error_msg.contains("invalid") || error_msg.contains("JSON"),
            "Error should mention parsing issue: {}",
            error_msg
        );
    }
}

/// Test config loading with invalid UTF-8 in path.
#[test]
fn test_load_config_with_invalid_utf8_fails_gracefully() {
    unsafe {
        let invalid_bytes = vec![0xFF, 0xFE, 0xFD];
        let invalid_cstr = CString::new(invalid_bytes).unwrap_or_else(|_| CString::new("").unwrap());

        let result = kreuzberg_load_extraction_config_from_file(invalid_cstr.as_ptr());

        assert!(result.is_null(), "Result should be null for invalid UTF-8");

        let error = get_last_error();
        assert!(error.is_some(), "Should have error message");
    }
}

/// Test config loading with empty file.
#[test]
fn test_load_config_from_empty_file_uses_defaults() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    fs::write(&config_path, "").unwrap();

    unsafe {
        let path_cstr = CString::new(config_path.to_str().unwrap()).unwrap();
        let result = kreuzberg_load_extraction_config_from_file(path_cstr.as_ptr());

        assert!(!result.is_null(), "Result should not be null for empty file");

        let json_str = c_str_to_string(result).expect("Should have valid JSON");
        kreuzberg_free_string(result);

        let json_value: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert!(json_value.is_object(), "Should be a JSON object");
    }
}

/// Test format detection from file extension.
#[test]
fn test_config_format_detection_from_extension() {
    let temp_dir = TempDir::new().unwrap();

    let yml_path = temp_dir.path().join("config.yml");
    let yaml_content = "ocr:\n  enabled: true";
    fs::write(&yml_path, yaml_content).unwrap();

    unsafe {
        let path_cstr = CString::new(yml_path.to_str().unwrap()).unwrap();
        let result = kreuzberg_load_extraction_config_from_file(path_cstr.as_ptr());
        assert!(!result.is_null(), ".yml extension should be recognized");
        kreuzberg_free_string(result);
    }

    let json_path = temp_dir.path().join("config.json");
    let json_content = r#"{"ocr": {"enabled": true}}"#;
    fs::write(&json_path, json_content).unwrap();

    unsafe {
        let path_cstr = CString::new(json_path.to_str().unwrap()).unwrap();
        let result = kreuzberg_load_extraction_config_from_file(path_cstr.as_ptr());
        assert!(!result.is_null(), ".json extension should be recognized");
        kreuzberg_free_string(result);
    }
}
