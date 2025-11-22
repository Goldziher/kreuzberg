// Auto-generated tests for html fixtures.
#![allow(clippy::too_many_lines)]
use e2e_rust::{assertions, resolve_document};
use kreuzberg::core::config::ExtractionConfig;

#[test]
fn test_html_complex_layout() {
    // Large Wikipedia HTML page to validate complex conversion.

    let document_path = resolve_document("web/taylor_swift.html");
    if !document_path.exists() {
        println!("Skipping html_complex_layout: missing document at {}", document_path.display());
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for html_complex_layout: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["text/html"]);
    assertions::assert_min_content_length(&result, 1000);
}

#[test]
fn test_html_simple_table() {
    // HTML table converted to markdown should retain structure.

    let document_path = resolve_document("web/simple_table.html");
    if !document_path.exists() {
        println!("Skipping html_simple_table: missing document at {}", document_path.display());
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for html_simple_table: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["text/html"]);
    assertions::assert_min_content_length(&result, 100);
    assertions::assert_content_contains_all(&result, &["Product", "Category", "Price", "Stock", "Laptop", "Electronics", "Sample Data Table"]);
    assertions::assert_table_count(&result, Some(1), None);
}

