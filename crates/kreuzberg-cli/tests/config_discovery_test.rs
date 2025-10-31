//! Integration tests for CLI config file discovery.
//!
//! These tests verify that the CLI correctly discovers and loads configuration files
//! in various formats (.toml, .yaml, .yml, .json) with case-insensitive extension
//! matching, explicit --config flag support, and proper error handling.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

/// Get the path to the kreuzberg binary.
fn get_binary_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{}/../../target/debug/kreuzberg", manifest_dir)
}

/// Build the binary before running tests.
fn build_binary() {
    let status = Command::new("cargo")
        .args(["build", "--bin", "kreuzberg"])
        .status()
        .expect("Failed to build kreuzberg binary");

    assert!(status.success(), "Failed to build kreuzberg binary");
}

/// Get the test_documents directory path.
fn get_test_documents_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().unwrap().parent().unwrap().join("test_documents")
}

/// Get a test file path relative to test_documents/.
fn get_test_file(relative_path: &str) -> String {
    get_test_documents_dir()
        .join(relative_path)
        .to_string_lossy()
        .to_string()
}

// ============================================================================
// Config Discovery Tests - .kreuzberg.* files
// ============================================================================

#[test]
fn test_discover_kreuzberg_toml_in_current_directory() {
    build_binary();

    let dir = tempdir().unwrap();
    let config_path = dir.path().join(".kreuzberg.toml");

    // Create a config file with recognizable settings
    fs::write(
        &config_path,
        r#"
use_cache = false
enable_quality_processing = false
"#,
    )
    .unwrap();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    // Run extract command from the directory containing .kreuzberg.toml
    let output = Command::new(get_binary_path())
        .current_dir(dir.path())
        .args(["extract", test_file.as_str()])
        .output()
        .expect("Failed to execute kreuzberg");

    // Should succeed - config file should be discovered and loaded
    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_discover_kreuzberg_yaml_in_current_directory() {
    build_binary();

    let dir = tempdir().unwrap();
    let config_path = dir.path().join(".kreuzberg.yaml");

    // Create a config file with recognizable settings
    fs::write(
        &config_path,
        r#"
use_cache: false
enable_quality_processing: false
"#,
    )
    .unwrap();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    // Run extract command from the directory containing .kreuzberg.yaml
    let output = Command::new(get_binary_path())
        .current_dir(dir.path())
        .args(["extract", test_file.as_str()])
        .output()
        .expect("Failed to execute kreuzberg");

    // Should succeed - config file should be discovered and loaded
    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_discover_kreuzberg_yml_in_current_directory() {
    build_binary();

    let dir = tempdir().unwrap();
    let config_path = dir.path().join(".kreuzberg.yml");

    // Create a config file with recognizable settings
    fs::write(
        &config_path,
        r#"
use_cache: false
enable_quality_processing: false
"#,
    )
    .unwrap();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    // Run extract command from the directory containing .kreuzberg.yml
    let output = Command::new(get_binary_path())
        .current_dir(dir.path())
        .args(["extract", test_file.as_str()])
        .output()
        .expect("Failed to execute kreuzberg");

    // Should succeed - config file should be discovered and loaded
    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_discover_kreuzberg_json_in_current_directory() {
    build_binary();

    let dir = tempdir().unwrap();
    let config_path = dir.path().join(".kreuzberg.json");

    // Create a config file with recognizable settings
    fs::write(
        &config_path,
        r#"{
    "use_cache": false,
    "enable_quality_processing": false
}"#,
    )
    .unwrap();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    // Run extract command from the directory containing .kreuzberg.json
    let output = Command::new(get_binary_path())
        .current_dir(dir.path())
        .args(["extract", test_file.as_str()])
        .output()
        .expect("Failed to execute kreuzberg");

    // Should succeed - config file should be discovered and loaded
    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ============================================================================
// Case-Insensitive Extension Tests
// ============================================================================

#[test]
fn test_case_insensitive_toml_extension() {
    build_binary();

    let dir = tempdir().unwrap();
    let config_path = dir.path().join("custom.TOML");

    fs::write(
        &config_path,
        r#"
use_cache = false
"#,
    )
    .unwrap();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let config_arg = config_path.to_string_lossy().into_owned();

    let output = Command::new(get_binary_path())
        .current_dir(dir.path())
        .args(["extract", "--config", config_arg.as_str(), test_file.as_str()])
        .output()
        .expect("Failed to execute kreuzberg");

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_case_insensitive_yaml_extension() {
    build_binary();

    let dir = tempdir().unwrap();
    let config_path = dir.path().join("custom.Yaml");

    fs::write(
        &config_path,
        r#"
use_cache: false
"#,
    )
    .unwrap();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let config_arg = config_path.to_string_lossy().into_owned();

    let output = Command::new(get_binary_path())
        .current_dir(dir.path())
        .args(["extract", "--config", config_arg.as_str(), test_file.as_str()])
        .output()
        .expect("Failed to execute kreuzberg");

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_case_insensitive_yml_extension() {
    build_binary();

    let dir = tempdir().unwrap();
    let config_path = dir.path().join("custom.YML");

    fs::write(
        &config_path,
        r#"
use_cache: false
"#,
    )
    .unwrap();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let config_arg = config_path.to_string_lossy().into_owned();

    let output = Command::new(get_binary_path())
        .current_dir(dir.path())
        .args(["extract", "--config", config_arg.as_str(), test_file.as_str()])
        .output()
        .expect("Failed to execute kreuzberg");

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_case_insensitive_json_extension() {
    build_binary();

    let dir = tempdir().unwrap();
    let config_path = dir.path().join("custom.JSON");

    fs::write(
        &config_path,
        r#"{
    "use_cache": false
}"#,
    )
    .unwrap();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let config_arg = config_path.to_string_lossy().into_owned();

    let output = Command::new(get_binary_path())
        .current_dir(dir.path())
        .args(["extract", "--config", config_arg.as_str(), test_file.as_str()])
        .output()
        .expect("Failed to execute kreuzberg");

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ============================================================================
// Explicit --config Flag Tests
// ============================================================================

#[test]
fn test_explicit_config_path_toml() {
    build_binary();

    let dir = tempdir().unwrap();
    let config_path = dir.path().join("custom_config.toml");

    fs::write(
        &config_path,
        r#"
use_cache = false
enable_quality_processing = false
"#,
    )
    .unwrap();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let config_arg = config_path.to_string_lossy().into_owned();

    let output = Command::new(get_binary_path())
        .args(["extract", "--config", config_arg.as_str(), test_file.as_str()])
        .output()
        .expect("Failed to execute kreuzberg");

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_explicit_config_path_yaml() {
    build_binary();

    let dir = tempdir().unwrap();
    let config_path = dir.path().join("custom_config.yaml");

    fs::write(
        &config_path,
        r#"
use_cache: false
enable_quality_processing: false
"#,
    )
    .unwrap();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let config_arg = config_path.to_string_lossy().into_owned();

    let output = Command::new(get_binary_path())
        .args(["extract", "--config", config_arg.as_str(), test_file.as_str()])
        .output()
        .expect("Failed to execute kreuzberg");

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_explicit_config_path_json() {
    build_binary();

    let dir = tempdir().unwrap();
    let config_path = dir.path().join("custom_config.json");

    fs::write(
        &config_path,
        r#"{
    "use_cache": false,
    "enable_quality_processing": false
}"#,
    )
    .unwrap();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let config_arg = config_path.to_string_lossy().into_owned();

    let output = Command::new(get_binary_path())
        .args(["extract", "--config", config_arg.as_str(), test_file.as_str()])
        .output()
        .expect("Failed to execute kreuzberg");

    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_invalid_config_extension() {
    build_binary();

    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.txt");

    fs::write(&config_path, "invalid content").unwrap();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let config_arg = config_path.to_string_lossy().into_owned();

    let output = Command::new(get_binary_path())
        .args(["extract", "--config", config_arg.as_str(), test_file.as_str()])
        .output()
        .expect("Failed to execute kreuzberg");

    // Should fail with unsupported extension error
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(".toml") || stderr.contains(".yaml") || stderr.contains(".json"),
        "Error message should mention supported extensions: {}",
        stderr
    );
}

#[test]
fn test_malformed_toml_config() {
    build_binary();

    let dir = tempdir().unwrap();
    let config_path = dir.path().join("bad_config.toml");

    // Invalid TOML syntax
    fs::write(&config_path, "use_cache = [[[[[").unwrap();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let config_arg = config_path.to_string_lossy().into_owned();

    let output = Command::new(get_binary_path())
        .args(["extract", "--config", config_arg.as_str(), test_file.as_str()])
        .output()
        .expect("Failed to execute kreuzberg");

    // Should fail with parsing error
    assert!(!output.status.success());
}

#[test]
fn test_malformed_yaml_config() {
    build_binary();

    let dir = tempdir().unwrap();
    let config_path = dir.path().join("bad_config.yaml");

    // Invalid YAML syntax
    fs::write(&config_path, "use_cache: [[[[[").unwrap();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let config_arg = config_path.to_string_lossy().into_owned();

    let output = Command::new(get_binary_path())
        .args(["extract", "--config", config_arg.as_str(), test_file.as_str()])
        .output()
        .expect("Failed to execute kreuzberg");

    // Should fail with parsing error
    assert!(!output.status.success());
}

#[test]
fn test_malformed_json_config() {
    build_binary();

    let dir = tempdir().unwrap();
    let config_path = dir.path().join("bad_config.json");

    // Invalid JSON syntax
    fs::write(&config_path, r#"{"use_cache": [[[[[}"#).unwrap();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let config_arg = config_path.to_string_lossy().into_owned();

    let output = Command::new(get_binary_path())
        .args(["extract", "--config", config_arg.as_str(), test_file.as_str()])
        .output()
        .expect("Failed to execute kreuzberg");

    // Should fail with parsing error
    assert!(!output.status.success());
}

#[test]
fn test_nonexistent_config_file() {
    build_binary();

    let dir = tempdir().unwrap();
    let config_path = dir.path().join("nonexistent.toml");

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let config_arg = config_path.to_string_lossy().into_owned();

    let output = Command::new(get_binary_path())
        .args(["extract", "--config", config_arg.as_str(), test_file.as_str()])
        .output()
        .expect("Failed to execute kreuzberg");

    // Should fail - cannot read nonexistent file
    assert!(!output.status.success());
}

// ============================================================================
// Default Config Tests
// ============================================================================

#[test]
fn test_default_config_when_no_file_found() {
    build_binary();

    let dir = tempdir().unwrap();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    // Run from empty directory - should use default config
    let output = Command::new(get_binary_path())
        .current_dir(dir.path())
        .args(["extract", test_file.as_str()])
        .output()
        .expect("Failed to execute kreuzberg");

    // Should succeed with default config
    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ============================================================================
// Config Validation Tests
// ============================================================================

#[test]
fn test_invalid_config_values() {
    build_binary();

    let dir = tempdir().unwrap();
    let config_path = dir.path().join("invalid.toml");

    // Invalid config value (negative max_pages)
    fs::write(
        &config_path,
        r#"
max_pages = -1
"#,
    )
    .unwrap();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let config_arg = config_path.to_string_lossy().into_owned();

    let output = Command::new(get_binary_path())
        .args(["extract", "--config", config_arg.as_str(), test_file.as_str()])
        .output()
        .expect("Failed to execute kreuzberg");

    // Should fail with validation error
    assert!(!output.status.success());
}
