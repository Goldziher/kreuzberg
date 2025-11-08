#![allow(clippy::too_many_lines)]
use e2e_rust::{assertions, resolve_document};
use kreuzberg::core::config::ExtractionConfig;

#[test]
fn test_xml_plant_catalog() {
    let document_path = resolve_document("xml/plant_catalog.xml");
    if !document_path.exists() {
        println!(
            "Skipping xml_plant_catalog: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for xml_plant_catalog: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["application/xml"]);
    assertions::assert_min_content_length(&result, 100);
    assertions::assert_metadata_expectation(&result, "element_count", &serde_json::json!({"gte":1}));
}
