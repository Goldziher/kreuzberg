#![allow(clippy::too_many_lines)]
use e2e_rust::{assertions, resolve_document};
use kreuzberg::core::config::ExtractionConfig;

#[test]
fn test_html_complex_layout() {
    // Large Wikipedia HTML page to validate complex conversion.
    // Note: This test runs with increased stack size due to deep HTML nesting

    let document_path = resolve_document("web/taylor_swift.html");
    if !document_path.exists() {
        println!(
            "Skipping html_complex_layout: missing document at {}",
            document_path.display()
        );
        return;
    }

    // Run with increased stack size (8MB instead of default 2MB) to handle deeply nested HTML
    let result = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let config = ExtractionConfig::default();
            kreuzberg::extract_file_sync(&document_path, None, &config)
        })
        .expect("Failed to spawn thread")
        .join()
        .expect("Thread panicked");

    let result = match result {
        Err(err) => panic!("Extraction failed for html_complex_layout: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["text/html"]);
    assertions::assert_min_content_length(&result, 1000);
}

#[test]
fn test_html_simple_table() {
    let document_path = resolve_document("web/simple_table.html");
    if !document_path.exists() {
        println!(
            "Skipping html_simple_table: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for html_simple_table: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["text/html"]);
    assertions::assert_min_content_length(&result, 20);
    assertions::assert_content_contains_all(&result, &["|"]);
}
