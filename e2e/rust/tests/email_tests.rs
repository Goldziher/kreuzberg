// Auto-generated tests for email fixtures.
#![allow(clippy::too_many_lines)]
use e2e_rust::{assertions, resolve_document};
use kreuzberg::core::config::ExtractionConfig;

#[test]
fn test_email_sample_eml() {
    // Sample EML email file to verify email parsing.

    let document_path = resolve_document("email/sample.eml");
    if !document_path.exists() {
        println!(
            "Skipping email_sample_eml: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for email_sample_eml: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["message/rfc822"]);
    assertions::assert_min_content_length(&result, 20);
}
