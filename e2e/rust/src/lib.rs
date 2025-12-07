//! Shared helpers for generated Rust E2E tests.

use kreuzberg::types::ExtractionResult;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Path to the workspace root.
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("e2e directory present")
        .parent()
        .expect("workspace root present")
        .to_path_buf()
}

/// Path to the shared test_documents directory.
pub fn test_documents_dir() -> PathBuf {
    workspace_root().join("test_documents")
}

/// Resolve a relative document path under test_documents.
pub fn resolve_document(relative: &str) -> PathBuf {
    test_documents_dir().join(relative)
}

/// Check if an external tool is available on the system PATH.
pub fn external_tool_available(tool: &str) -> bool {
    std::process::Command::new(tool)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Generated assertions shared across tests.
pub mod assertions {
    use super::*;

    /// Assert that the MIME type matches any of the expected patterns.
    pub fn assert_expected_mime(result: &ExtractionResult, expected: &[&str]) {
        if expected.is_empty() {
            return;
        }

        let mime = result.mime_type.as_str();
        let matches = expected.iter().any(|candidate| mime.contains(candidate));
        assert!(matches, "Expected MIME {:?} to match one of {:?}", mime, expected);
    }

    /// Assert that content length is at least `min`.
    pub fn assert_min_content_length(result: &ExtractionResult, min: usize) {
        assert!(
            result.content.len() >= min,
            "Expected content length >= {min}, got {}",
            result.content.len()
        );
    }

    /// Assert that content length is at most `max`.
    pub fn assert_max_content_length(result: &ExtractionResult, max: usize) {
        assert!(
            result.content.len() <= max,
            "Expected content length <= {max}, got {}",
            result.content.len()
        );
    }

    /// Assert that the content contains any of the provided snippets.
    pub fn assert_content_contains_any(result: &ExtractionResult, snippets: &[&str]) {
        if snippets.is_empty() {
            return;
        }

        let lowered = result.content.to_lowercase();
        let preview = result.content.chars().take(160).collect::<String>();
        let found = snippets.iter().any(|snippet| lowered.contains(&snippet.to_lowercase()));

        assert!(
            found,
            "Expected content to contain at least one snippet from {:?}. Preview: {:?}",
            snippets, preview
        );
    }

    /// Assert that the content contains all provided snippets.
    pub fn assert_content_contains_all(result: &ExtractionResult, snippets: &[&str]) {
        if snippets.is_empty() {
            return;
        }

        let lowered = result.content.to_lowercase();
        let all_found = snippets.iter().all(|snippet| lowered.contains(&snippet.to_lowercase()));

        assert!(all_found, "Expected content to contain all snippets {:?}", snippets);
    }

    /// Assert table count boundaries.
    pub fn assert_table_count(result: &ExtractionResult, min: Option<usize>, max: Option<usize>) {
        if let Some(min_tables) = min {
            assert!(
                result.tables.len() >= min_tables,
                "Expected at least {min_tables} tables, found {}",
                result.tables.len()
            );
        }
        if let Some(max_tables) = max {
            assert!(
                result.tables.len() <= max_tables,
                "Expected at most {max_tables} tables, found {}",
                result.tables.len()
            );
        }
    }

    /// Assert detected languages contain expected entries with optional confidence requirements.
    pub fn assert_detected_languages(result: &ExtractionResult, expected: &[&str], min_confidence: Option<f32>) {
        let Some(languages) = result.detected_languages.as_ref() else {
            panic!("Expected detected languages but field is None");
        };

        for lang in expected {
            assert!(
                languages.iter().any(|detected| detected == lang),
                "Expected detected languages to contain {lang}, got {:?}",
                languages
            );
        }

        if let Some(threshold) = min_confidence
            && let Ok(Value::Object(map)) = serde_json::to_value(&result.metadata)
            && let Some(confidence) = map.get("confidence").and_then(Value::as_f64)
        {
            assert!(
                (confidence as f32) >= threshold,
                "Expected confidence >= {threshold}, got {confidence}"
            );
        }
    }

    /// Assert metadata expectations expressed as JSON.
    pub fn assert_metadata_expectation(result: &ExtractionResult, path: &str, expectation: &Value) {
        let metadata = serde_json::to_value(&result.metadata).expect("Metadata should serialize to JSON");
        let value =
            lookup_path(&metadata, path).unwrap_or_else(|| panic!("Metadata path `{path}` missing in {:?}", metadata));

        if let Some(eq) = expectation.get("eq") {
            assert!(
                values_equal(value, eq),
                "Expected metadata `{path}` == {eq:?}, got {value:?}"
            );
        }

        if let Some(gte) = expectation.get("gte") {
            let actual = value
                .as_f64()
                .or_else(|| value.as_i64().map(|n| n as f64))
                .unwrap_or_else(|| panic!("Metadata `{path}` is not numeric: {value:?}"));
            let min = gte
                .as_f64()
                .or_else(|| gte.as_i64().map(|n| n as f64))
                .unwrap_or_else(|| panic!("Expectation `{path}` gte is not numeric: {gte:?}"));
            assert!(actual >= min, "Expected metadata `{path}` >= {min}, got {actual}");
        }

        if let Some(lte) = expectation.get("lte") {
            let actual = value
                .as_f64()
                .or_else(|| value.as_i64().map(|n| n as f64))
                .unwrap_or_else(|| panic!("Metadata `{path}` is not numeric: {value:?}"));
            let max = lte
                .as_f64()
                .or_else(|| lte.as_i64().map(|n| n as f64))
                .unwrap_or_else(|| panic!("Expectation `{path}` lte is not numeric: {lte:?}"));
            assert!(actual <= max, "Expected metadata `{path}` <= {max}, got {actual}");
        }

        if let Some(contains) = expectation.get("contains") {
            match (value.as_str(), contains.as_str()) {
                (Some(actual), Some(expected)) => {
                    assert!(
                        actual.contains(expected),
                        "Expected metadata `{path}` string `{actual}` to contain `{expected}`"
                    );
                }
                _ if value.is_array() && contains.is_string() => {
                    let actual_values = value
                        .as_array()
                        .expect("value is array by branch")
                        .iter()
                        .collect::<Vec<_>>();
                    let expected = contains.as_str().expect("contains is string by branch");
                    assert!(
                        actual_values
                            .iter()
                            .any(|item| { item.as_str().is_some_and(|s| s.contains(expected)) }),
                        "Expected metadata `{path}` to contain `{expected}`, got {actual_values:?}"
                    );
                }
                _ if value.is_array() && contains.is_array() => {
                    let actual_values = value
                        .as_array()
                        .expect("value is array by branch")
                        .iter()
                        .collect::<Vec<_>>();
                    for needle in contains.as_array().expect("contains is array") {
                        assert!(
                            actual_values.iter().any(|item| values_equal(item, needle)),
                            "Expected metadata `{path}` to contain {needle:?}, got {actual_values:?}"
                        );
                    }
                }
                _ => panic!("Metadata `{path}` contains expectation unsupported for value {value:?}"),
            }
        }

        if let Some(exists) = expectation.get("exists").and_then(Value::as_bool) {
            if exists {
                assert!(!value.is_null(), "Expected metadata `{path}` to exist (non-null)");
            } else {
                panic!("`exists: false` is not supported for metadata assertions");
            }
        }
    }

    fn lookup_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
        let mut current = value;
        for segment in path.split('.') {
            current = match current {
                Value::Object(map) => map.get(segment)?,
                _ => return None,
            };
        }
        Some(current)
    }

    fn values_equal(lhs: &Value, rhs: &Value) -> bool {
        match (lhs, rhs) {
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            _ => lhs == rhs,
        }
    }
}
