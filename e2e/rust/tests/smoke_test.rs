// Auto-generated tests for smoke fixtures.
#![allow(clippy::too_many_lines)]
use e2e_rust::{assertions, resolve_document};
use kreuzberg::core::config::ExtractionConfig;

#[test]
fn test_smoke_docx_basic() {
    // Smoke test: DOCX with formatted text

    let document_path = resolve_document("documents/fake.docx");
    if !document_path.exists() {
        println!(
            "Skipping smoke_docx_basic: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for smoke_docx_basic: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(
        &result,
        &["application/vnd.openxmlformats-officedocument.wordprocessingml.document"],
    );
    assertions::assert_min_content_length(&result, 20);
    assertions::assert_content_contains_any(&result, &["Lorem", "ipsum", "document", "text"]);
}

#[test]
fn test_smoke_html_basic() {
    // Smoke test: HTML converted to Markdown

    let document_path = resolve_document("web/simple_table.html");
    if !document_path.exists() {
        println!(
            "Skipping smoke_html_basic: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for smoke_html_basic: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["text/html"]);
    assertions::assert_min_content_length(&result, 10);
    assertions::assert_content_contains_any(&result, &["#", "**", "simple", "HTML"]);
}

#[test]
fn test_smoke_image_png() {
    // Smoke test: PNG image (without OCR, metadata only)

    let document_path = resolve_document("images/sample.png");
    if !document_path.exists() {
        println!(
            "Skipping smoke_image_png: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for smoke_image_png: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["image/png"]);
    assertions::assert_metadata_expectation(&result, "format", &serde_json::json!("PNG"));
}

#[test]
fn test_smoke_json_basic() {
    // Smoke test: JSON file extraction

    let document_path = resolve_document("data_formats/simple.json");
    if !document_path.exists() {
        println!(
            "Skipping smoke_json_basic: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for smoke_json_basic: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["application/json"]);
    assertions::assert_min_content_length(&result, 5);
}

#[test]
fn test_smoke_pdf_basic() {
    // Smoke test: PDF with simple text extraction

    let document_path = resolve_document("pdfs/fake_memo.pdf");
    if !document_path.exists() {
        println!(
            "Skipping smoke_pdf_basic: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for smoke_pdf_basic: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["application/pdf"]);
    assertions::assert_min_content_length(&result, 50);
    assertions::assert_content_contains_any(&result, &["May 5, 2023", "To Whom it May Concern"]);
}

#[test]
fn test_smoke_txt_basic() {
    // Smoke test: Plain text file

    let document_path = resolve_document("text/report.txt");
    if !document_path.exists() {
        println!(
            "Skipping smoke_txt_basic: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for smoke_txt_basic: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["text/plain"]);
    assertions::assert_min_content_length(&result, 5);
}

#[test]
fn test_smoke_xlsx_basic() {
    // Smoke test: XLSX with basic spreadsheet data including tables

    let document_path = resolve_document("spreadsheets/stanley_cups.xlsx");
    if !document_path.exists() {
        println!(
            "Skipping smoke_xlsx_basic: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for smoke_xlsx_basic: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(
        &result,
        &["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
    );
    assertions::assert_min_content_length(&result, 100);
    assertions::assert_content_contains_all(
        &result,
        &[
            "Team",
            "Location",
            "Stanley Cups",
            "Blues",
            "Flyers",
            "Maple Leafs",
            "STL",
            "PHI",
            "TOR",
        ],
    );
    assertions::assert_table_count(&result, Some(1), None);
    assertions::assert_metadata_expectation(&result, "sheet_count", &serde_json::json!({"gte":2}));
    assertions::assert_metadata_expectation(&result, "sheet_names", &serde_json::json!({"contains":"Stanley Cups"}));
}
