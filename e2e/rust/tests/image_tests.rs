// Auto-generated tests for image fixtures.
#![allow(clippy::too_many_lines)]
use e2e_rust::{assertions, resolve_document};
use kreuzberg::core::config::ExtractionConfig;

#[test]
fn test_image_metadata_only() {
    // JPEG image to validate metadata extraction without OCR.

    let document_path = resolve_document("images/example.jpg");
    if !document_path.exists() {
        println!(
            "Skipping image_metadata_only: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config: ExtractionConfig = serde_json::from_str(
        r#"{
  "ocr": null
}"#,
    )
    .expect("Fixture config should deserialize");

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for image_metadata_only: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["image/jpeg"]);
    assertions::assert_max_content_length(&result, 100);
}
