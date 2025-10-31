//! Integration tests for CLI commands (extract, detect, batch).
//!
//! These tests verify that the CLI commands work correctly end-to-end,
//! including input validation, file processing, and output formatting.

use std::path::PathBuf;
use std::process::Command;

/// Get the path to the kreuzberg binary.
fn get_binary_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{}/../../target/debug/kreuzberg", manifest_dir)
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

/// Build the binary before running tests.
fn build_binary() {
    let status = Command::new("cargo")
        .args(&["build", "--bin", "kreuzberg"])
        .status()
        .expect("Failed to build kreuzberg binary");

    assert!(status.success(), "Failed to build kreuzberg binary");
}

// ============================================================================
// Extract Command Tests
// ============================================================================

#[test]
fn test_extract_text_file() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let output = Command::new(get_binary_path())
        .args(&["extract", &test_file])
        .output()
        .expect("Failed to execute extract command");

    assert!(
        output.status.success(),
        "Extract command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "Extract output should not be empty");
}

#[test]
fn test_extract_with_json_output() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let output = Command::new(get_binary_path())
        .args(&["extract", &test_file, "--format", "json"])
        .output()
        .expect("Failed to execute extract command");

    assert!(
        output.status.success(),
        "Extract command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify it's valid JSON
    let json_result: serde_json::Result<serde_json::Value> = serde_json::from_str(&stdout);
    assert!(json_result.is_ok(), "Output should be valid JSON, got: {}", stdout);

    // Verify JSON has expected fields
    let json = json_result.unwrap();
    assert!(json.get("content").is_some(), "JSON should have 'content' field");
    assert!(json.get("mime_type").is_some(), "JSON should have 'mime_type' field");
}

#[test]
fn test_extract_with_chunking() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let output = Command::new(get_binary_path())
        .args(&[
            "extract",
            &test_file,
            "--chunk-size",
            "100",
            "--chunk-overlap",
            "20",
            "--format",
            "json",
        ])
        .output()
        .expect("Failed to execute extract command");

    assert!(
        output.status.success(),
        "Extract with chunking failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Should be valid JSON");

    // Verify chunks field exists and is an array
    assert!(json.get("chunks").is_some(), "JSON should have 'chunks' field");
    assert!(json["chunks"].is_array(), "'chunks' should be an array");
}

// ============================================================================
// Extract Command - Validation Tests
// ============================================================================

#[test]
fn test_extract_file_not_found() {
    build_binary();

    let output = Command::new(get_binary_path())
        .args(&["extract", "/nonexistent/file.txt"])
        .output()
        .expect("Failed to execute extract command");

    assert!(!output.status.success(), "Extract should fail for nonexistent file");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("File not found"),
        "Error should mention file not found, got: {}",
        stderr
    );
}

#[test]
fn test_extract_directory_not_file() {
    build_binary();

    let output = Command::new(get_binary_path())
        .args(&["extract", "/tmp"])
        .output()
        .expect("Failed to execute extract command");

    assert!(!output.status.success(), "Extract should fail for directory");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not a file") || stderr.contains("regular file"),
        "Error should mention path is not a file, got: {}",
        stderr
    );
}

#[test]
fn test_extract_invalid_chunk_size_zero() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let output = Command::new(get_binary_path())
        .args(&["extract", &test_file, "--chunk-size", "0"])
        .output()
        .expect("Failed to execute extract command");

    assert!(!output.status.success(), "Extract should fail for chunk size 0");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid chunk size") || stderr.contains("must be greater than 0"),
        "Error should mention invalid chunk size, got: {}",
        stderr
    );
}

#[test]
fn test_extract_invalid_chunk_size_too_large() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let output = Command::new(get_binary_path())
        .args(&["extract", &test_file, "--chunk-size", "2000000"])
        .output()
        .expect("Failed to execute extract command");

    assert!(!output.status.success(), "Extract should fail for chunk size > 1M");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid chunk size") || stderr.contains("1,000,000"),
        "Error should mention chunk size limit, got: {}",
        stderr
    );
}

#[test]
fn test_extract_invalid_overlap_equals_chunk_size() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let output = Command::new(get_binary_path())
        .args(&["extract", &test_file, "--chunk-size", "100", "--chunk-overlap", "100"])
        .output()
        .expect("Failed to execute extract command");

    assert!(
        !output.status.success(),
        "Extract should fail when overlap equals chunk size"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid chunk overlap") || stderr.contains("must be less than chunk size"),
        "Error should mention overlap constraint, got: {}",
        stderr
    );
}

// ============================================================================
// Detect Command Tests
// ============================================================================

#[test]
fn test_detect_mime_type() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let output = Command::new(get_binary_path())
        .args(&["detect", &test_file])
        .output()
        .expect("Failed to execute detect command");

    assert!(
        output.status.success(),
        "Detect command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "Detect output should not be empty");
    assert!(
        stdout.contains("text/plain") || stdout.contains("text"),
        "Should detect text MIME type, got: {}",
        stdout
    );
}

#[test]
fn test_detect_with_json_output() {
    build_binary();

    let test_file = get_test_file("text/simple.txt");
    if !PathBuf::from(&test_file).exists() {
        tracing::debug!("Skipping test: {} not found", test_file);
        return;
    }

    let output = Command::new(get_binary_path())
        .args(&["detect", &test_file, "--format", "json"])
        .output()
        .expect("Failed to execute detect command");

    assert!(
        output.status.success(),
        "Detect command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify it's valid JSON
    let json_result: serde_json::Result<serde_json::Value> = serde_json::from_str(&stdout);
    assert!(json_result.is_ok(), "Output should be valid JSON, got: {}", stdout);

    // Verify JSON has expected fields
    let json = json_result.unwrap();
    assert!(json.get("mime_type").is_some(), "JSON should have 'mime_type' field");
    assert!(json.get("path").is_some(), "JSON should have 'path' field");
}

#[test]
fn test_detect_file_not_found() {
    build_binary();

    let output = Command::new(get_binary_path())
        .args(&["detect", "/nonexistent/file.txt"])
        .output()
        .expect("Failed to execute detect command");

    assert!(!output.status.success(), "Detect should fail for nonexistent file");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("File not found"),
        "Error should mention file not found, got: {}",
        stderr
    );
}

// ============================================================================
// Batch Command Tests
// ============================================================================

#[test]
fn test_batch_multiple_files() {
    build_binary();

    let file1 = get_test_file("text/simple.txt");
    let file2 = get_test_file("text/simple.txt"); // Use same file twice for simplicity

    if !PathBuf::from(&file1).exists() {
        tracing::debug!("Skipping test: {} not found", file1);
        return;
    }

    let output = Command::new(get_binary_path())
        .args(&["batch", &file1, &file2, "--format", "json"])
        .output()
        .expect("Failed to execute batch command");

    assert!(
        output.status.success(),
        "Batch command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify it's valid JSON array
    let json_result: serde_json::Result<serde_json::Value> = serde_json::from_str(&stdout);
    assert!(json_result.is_ok(), "Output should be valid JSON, got: {}", stdout);

    let json = json_result.unwrap();
    assert!(json.is_array(), "Batch output should be a JSON array");
    assert_eq!(json.as_array().unwrap().len(), 2, "Should have 2 results");
}

#[test]
fn test_batch_with_missing_file() {
    build_binary();

    let valid_file = get_test_file("text/simple.txt");

    if !PathBuf::from(&valid_file).exists() {
        tracing::debug!("Skipping test: {} not found", valid_file);
        return;
    }

    let output = Command::new(get_binary_path())
        .args(&["batch", &valid_file, "/nonexistent/file.txt"])
        .output()
        .expect("Failed to execute batch command");

    assert!(!output.status.success(), "Batch should fail when one file is missing");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("File not found") || stderr.contains("Invalid file"),
        "Error should mention file not found, got: {}",
        stderr
    );
}

// ============================================================================
// Help Command Tests
// ============================================================================

#[test]
fn test_extract_help() {
    build_binary();

    let output = Command::new(get_binary_path())
        .args(&["extract", "--help"])
        .output()
        .expect("Failed to execute extract --help");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Extract text from a document"));
    assert!(stdout.contains("--chunk-size"));
    assert!(stdout.contains("--chunk-overlap"));
}

#[test]
fn test_detect_help() {
    build_binary();

    let output = Command::new(get_binary_path())
        .args(&["detect", "--help"])
        .output()
        .expect("Failed to execute detect --help");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Detect MIME type"));
}

#[test]
fn test_batch_help() {
    build_binary();

    let output = Command::new(get_binary_path())
        .args(&["batch", "--help"])
        .output()
        .expect("Failed to execute batch --help");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Batch extract from multiple documents"));
}
