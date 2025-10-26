//! Shared test helpers for integration tests.
//!
//! This module provides common utilities for loading test files,
//! making assertions, and setting up test environments.

#![allow(dead_code)]

use kreuzberg::types::ExtractionResult;
use std::path::PathBuf;

/// Get the test_documents directory path.
///
/// This assumes the test is running from the workspace root.
pub fn get_test_documents_dir() -> PathBuf {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    workspace_root.join("test_documents")
}

/// Get the full path to a test file.
///
/// # Arguments
///
/// * `relative_path` - Path relative to test_documents/
pub fn get_test_file_path(relative_path: &str) -> PathBuf {
    get_test_documents_dir().join(relative_path)
}

/// Assert that extraction result contains non-empty content.
///
/// This is a common assertion for most extraction tests - we want
/// to verify that *something* was extracted, even if we don't know
/// the exact content.
pub fn assert_non_empty_content(result: &ExtractionResult) {
    assert!(
        !result.content.trim().is_empty(),
        "Extraction result should have non-empty content, got: '{}'",
        result.content
    );
}

/// Assert that extraction result has expected MIME type.
pub fn assert_mime_type(result: &ExtractionResult, expected: &str) {
    assert_eq!(
        result.mime_type, expected,
        "Expected MIME type '{}', got '{}'",
        expected, result.mime_type
    );
}

/// Skip test if file doesn't exist (for optional test files).
///
/// Returns true if test should be skipped.
pub fn skip_if_missing(relative_path: &str) -> bool {
    let path = get_test_file_path(relative_path);
    if !path.exists() {
        eprintln!("Skipping test: file not found at {}", path.display());
        return true;
    }
    false
}

/// Check if test documents directory exists and has files.
///
/// This is useful for CI environments where test_documents might
/// be a git submodule that hasn't been initialized.
pub fn test_documents_available() -> bool {
    let dir = get_test_documents_dir();
    dir.exists() && dir.read_dir().map(|mut d| d.next().is_some()).unwrap_or(false)
}

/// Assert that content length is above a minimum threshold.
///
/// This is useful for smoke testing - ensuring substantial content
/// was extracted without needing to verify exact text.
pub fn assert_min_content_length(result: &ExtractionResult, min_length: usize) {
    assert!(
        result.content.len() >= min_length,
        "Expected content length >= {}, got {}. Content preview: '{}'",
        min_length,
        result.content.len(),
        result.content.chars().take(200).collect::<String>()
    );
}

/// Assert that content contains at least one of the given substrings.
pub fn assert_content_contains_any(result: &ExtractionResult, substrings: &[&str]) {
    let found = substrings.iter().any(|s| result.content.contains(s));
    assert!(
        found,
        "Expected content to contain at least one of {:?}, but found none",
        substrings
    );
}

/// Assert that extraction result has at least one table.
pub fn assert_has_tables(result: &ExtractionResult) {
    assert!(
        !result.tables.is_empty(),
        "Expected result to have tables, but found none"
    );
}

/// Create a test configuration with OCR enabled.
pub fn test_config_with_ocr() -> kreuzberg::core::config::ExtractionConfig {
    use kreuzberg::core::config::{ExtractionConfig, OcrConfig};

    ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: None,
        }),
        force_ocr: false,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_test_documents_dir() {
        let dir = get_test_documents_dir();
        assert!(dir.to_string_lossy().ends_with("test_documents"));
    }

    #[test]
    fn test_test_documents_available() {
        let available = test_documents_available();
        if !available {
            eprintln!("Warning: test_documents directory not available");
            eprintln!("This is expected in CI without git submodules initialized");
        }
    }
}
