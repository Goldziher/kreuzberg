use std::ffi::{CStr, CString};
use std::fs;
use std::os::raw::c_char;
use std::ptr;
use tempfile::TempDir;

// Import the FFI functions
unsafe extern "C" {
    fn kreuzberg_config_from_file(path: *const c_char) -> *mut std::ffi::c_void;

    fn kreuzberg_config_discover() -> *mut std::ffi::c_void;

    fn kreuzberg_last_error() -> *const c_char;

    fn kreuzberg_free_config(config: *mut std::ffi::c_void);
}

#[test]
fn test_config_from_file_toml() {
    unsafe {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("kreuzberg.toml");

        let config_content = r#"
[ocr]
enabled = true
backend = "tesseract"

[chunking]
enabled = false
        "#;

        fs::write(&config_path, config_content).unwrap();

        let path_str = CString::new(config_path.to_str().unwrap()).unwrap();
        let config_ptr = kreuzberg_config_from_file(path_str.as_ptr());

        assert!(!config_ptr.is_null(), "Config should be loaded successfully");

        kreuzberg_free_config(config_ptr);
    }
}

#[test]
fn test_config_from_file_yaml() {
    unsafe {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("kreuzberg.yaml");

        let config_content = r#"
ocr:
  enabled: true
  backend: tesseract

chunking:
  enabled: false
        "#;

        fs::write(&config_path, config_content).unwrap();

        let path_str = CString::new(config_path.to_str().unwrap()).unwrap();
        let config_ptr = kreuzberg_config_from_file(path_str.as_ptr());

        assert!(!config_ptr.is_null(), "Config should be loaded successfully");

        kreuzberg_free_config(config_ptr);
    }
}

#[test]
fn test_config_from_file_json() {
    unsafe {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("kreuzberg.json");

        let config_content = r#"
{
  "ocr": {
    "enabled": true,
    "backend": "tesseract"
  },
  "chunking": {
    "enabled": false
  }
}
        "#;

        fs::write(&config_path, config_content).unwrap();

        let path_str = CString::new(config_path.to_str().unwrap()).unwrap();
        let config_ptr = kreuzberg_config_from_file(path_str.as_ptr());

        assert!(!config_ptr.is_null(), "Config should be loaded successfully");

        kreuzberg_free_config(config_ptr);
    }
}

#[test]
fn test_config_from_file_null_path() {
    unsafe {
        let config_ptr = kreuzberg_config_from_file(ptr::null());

        assert!(config_ptr.is_null(), "Should return NULL for NULL path");

        let error = kreuzberg_last_error();
        assert!(!error.is_null());
        let error_str = CStr::from_ptr(error).to_str().unwrap();
        assert!(error_str.contains("NULL"), "Error should mention NULL: {}", error_str);
    }
}

#[test]
fn test_config_from_file_nonexistent() {
    unsafe {
        let path = CString::new("/nonexistent/path/kreuzberg.toml").unwrap();
        let config_ptr = kreuzberg_config_from_file(path.as_ptr());

        assert!(config_ptr.is_null(), "Should return NULL for nonexistent file");

        let error = kreuzberg_last_error();
        assert!(!error.is_null());
        let error_str = CStr::from_ptr(error).to_str().unwrap();
        assert!(
            error_str.contains("IO") || error_str.contains("not found") || error_str.contains("No such"),
            "Error should indicate file not found: {}",
            error_str
        );
    }
}

#[test]
fn test_config_from_file_invalid_toml() {
    unsafe {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("invalid.toml");

        let config_content = r#"
[ocr
enabled = true  # Missing closing bracket
        "#;

        fs::write(&config_path, config_content).unwrap();

        let path_str = CString::new(config_path.to_str().unwrap()).unwrap();
        let config_ptr = kreuzberg_config_from_file(path_str.as_ptr());

        assert!(config_ptr.is_null(), "Should return NULL for invalid TOML");

        let error = kreuzberg_last_error();
        assert!(!error.is_null());
    }
}

#[test]
fn test_config_from_file_invalid_json() {
    unsafe {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("invalid.json");

        let config_content = r#"
{
  "ocr": {
    "enabled": true,
  }  // Trailing comma is invalid in strict JSON
}
        "#;

        fs::write(&config_path, config_content).unwrap();

        let path_str = CString::new(config_path.to_str().unwrap()).unwrap();
        let config_ptr = kreuzberg_config_from_file(path_str.as_ptr());

        assert!(config_ptr.is_null(), "Should return NULL for invalid JSON");

        let error = kreuzberg_last_error();
        assert!(!error.is_null());
    }
}

#[test]
fn test_config_from_file_no_extension() {
    unsafe {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("kreuzberg");

        fs::write(&config_path, "some content").unwrap();

        let path_str = CString::new(config_path.to_str().unwrap()).unwrap();
        let config_ptr = kreuzberg_config_from_file(path_str.as_ptr());

        assert!(config_ptr.is_null(), "Should return NULL for file without extension");

        let error = kreuzberg_last_error();
        assert!(!error.is_null());
        let error_str = CStr::from_ptr(error).to_str().unwrap();
        assert!(
            error_str.contains("extension") || error_str.contains("format"),
            "Error should mention extension: {}",
            error_str
        );
    }
}

#[test]
fn test_config_from_file_invalid_utf8_path() {
    unsafe {
        let invalid_path = b"/tmp/test\xFF\xFEinvalid.toml\0";

        let config_ptr = kreuzberg_config_from_file(invalid_path.as_ptr() as *const c_char);

        assert!(config_ptr.is_null(), "Should return NULL for invalid UTF-8 path");

        let error = kreuzberg_last_error();
        assert!(!error.is_null());
        let error_str = CStr::from_ptr(error).to_str().unwrap();
        assert!(error_str.contains("UTF-8"));
    }
}

#[test]
fn test_config_discover_not_found() {
    unsafe {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();

        std::env::set_current_dir(&temp_dir).unwrap();

        let config_ptr = kreuzberg_config_discover();

        assert!(config_ptr.is_null(), "Should return NULL when no config found");

        let error = kreuzberg_last_error();
        if !error.is_null() {
            let error_str = CStr::from_ptr(error).to_str().unwrap();
            assert!(
                error_str.is_empty() || error_str.contains("not found") || error_str.contains("IO"),
                "Error should be empty or indicate not found: {}",
                error_str
            );
        }

        std::env::set_current_dir(original_dir).unwrap();
    }
}

#[test]
fn test_config_discover_toml() {
    unsafe {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("kreuzberg.toml");

        let config_content = r#"
[ocr]
enabled = true
        "#;

        fs::write(&config_path, config_content).unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let config_ptr = kreuzberg_config_discover();

        assert!(!config_ptr.is_null(), "Should discover config in current directory");

        kreuzberg_free_config(config_ptr);

        std::env::set_current_dir(original_dir).unwrap();
    }
}

#[test]
fn test_config_discover_yaml() {
    unsafe {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("kreuzberg.yaml");

        let config_content = r#"
ocr:
  enabled: true
        "#;

        fs::write(&config_path, config_content).unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let config_ptr = kreuzberg_config_discover();

        assert!(!config_ptr.is_null(), "Should discover YAML config");

        kreuzberg_free_config(config_ptr);

        std::env::set_current_dir(original_dir).unwrap();
    }
}

#[test]
fn test_config_discover_parent_directory() {
    unsafe {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("kreuzberg.toml");

        let config_content = r#"
[ocr]
enabled = true
        "#;

        fs::write(&config_path, config_content).unwrap();

        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&subdir).unwrap();

        let config_ptr = kreuzberg_config_discover();

        assert!(!config_ptr.is_null(), "Should discover config in parent directory");

        kreuzberg_free_config(config_ptr);

        std::env::set_current_dir(original_dir).unwrap();
    }
}

#[test]
fn test_config_discover_preference_order() {
    unsafe {
        let temp_dir = TempDir::new().unwrap();

        fs::write(temp_dir.path().join("kreuzberg.toml"), "[ocr]\nenabled = true").unwrap();
        fs::write(temp_dir.path().join("kreuzberg.yaml"), "ocr:\n  enabled: false").unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let config_ptr = kreuzberg_config_discover();

        assert!(!config_ptr.is_null(), "Should discover a config file");

        kreuzberg_free_config(config_ptr);

        std::env::set_current_dir(original_dir).unwrap();
    }
}
