#![allow(clippy::too_many_lines)]
use e2e_rust::{assertions, resolve_document};
use kreuzberg::core::config::ExtractionConfig;

#[test]
fn test_structured_json_basic() {
    let document_path = resolve_document("json/sample_document.json");
    if !document_path.exists() {
        println!(
            "Skipping structured_json_basic: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for structured_json_basic: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["application/json"]);
    assertions::assert_min_content_length(&result, 20);
    assertions::assert_content_contains_any(&result, &["Sample Document", "Test Author"]);
}

#[test]
fn test_structured_json_simple() {
    let document_path = resolve_document("data_formats/simple.json");
    if !document_path.exists() {
        println!(
            "Skipping structured_json_simple: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for structured_json_simple: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["application/json"]);
    assertions::assert_min_content_length(&result, 10);
    assertions::assert_content_contains_any(&result, &["{", "name"]);
}

#[test]
fn test_structured_yaml_simple() {
    let document_path = resolve_document("data_formats/simple.yaml");
    if !document_path.exists() {
        println!(
            "Skipping structured_yaml_simple: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for structured_yaml_simple: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["application/x-yaml"]);
    assertions::assert_min_content_length(&result, 10);
}
