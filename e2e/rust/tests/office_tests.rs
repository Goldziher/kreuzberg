#![allow(clippy::too_many_lines)]
use e2e_rust::{assertions, resolve_document};
use kreuzberg::KreuzbergError;
use kreuzberg::core::config::ExtractionConfig;

#[test]
fn test_office_doc_legacy() {
    // Legacy .doc document conversion via LibreOffice.

    let document_path = resolve_document("legacy_office/unit_test_lists.doc");
    if !document_path.exists() {
        println!(
            "Skipping office_doc_legacy: missing document at {}",
            document_path.display()
        );
        return;
    }
    if !e2e_rust::external_tool_available("libreoffice") {
        println!("Skipping office_doc_legacy: libreoffice not available on PATH");
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(KreuzbergError::MissingDependency(dep)) => {
            println!("Skipping office_doc_legacy: missing dependency {dep}", dep = dep);
            return;
        }
        Err(KreuzbergError::UnsupportedFormat(fmt)) => {
            println!(
                "Skipping office_doc_legacy: unsupported format {fmt} (requires optional tool)",
                fmt = fmt
            );
            return;
        }
        Err(err) => panic!("Extraction failed for office_doc_legacy: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["application/msword"]);
    assertions::assert_min_content_length(&result, 20);
}

#[test]
fn test_office_docx_basic() {
    let document_path = resolve_document("office/document.docx");
    if !document_path.exists() {
        println!(
            "Skipping office_docx_basic: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for office_docx_basic: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(
        &result,
        &["application/vnd.openxmlformats-officedocument.wordprocessingml.document"],
    );
    assertions::assert_min_content_length(&result, 10);
}

#[test]
fn test_office_docx_equations() {
    let document_path = resolve_document("documents/equations.docx");
    if !document_path.exists() {
        println!(
            "Skipping office_docx_equations: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for office_docx_equations: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(
        &result,
        &["application/vnd.openxmlformats-officedocument.wordprocessingml.document"],
    );
    assertions::assert_min_content_length(&result, 20);
}

#[test]
fn test_office_docx_fake() {
    let document_path = resolve_document("documents/fake.docx");
    if !document_path.exists() {
        println!(
            "Skipping office_docx_fake: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for office_docx_fake: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(
        &result,
        &["application/vnd.openxmlformats-officedocument.wordprocessingml.document"],
    );
    assertions::assert_min_content_length(&result, 20);
}

#[test]
fn test_office_docx_formatting() {
    let document_path = resolve_document("documents/unit_test_formatting.docx");
    if !document_path.exists() {
        println!(
            "Skipping office_docx_formatting: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for office_docx_formatting: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(
        &result,
        &["application/vnd.openxmlformats-officedocument.wordprocessingml.document"],
    );
    assertions::assert_min_content_length(&result, 20);
}

#[test]
fn test_office_docx_headers() {
    let document_path = resolve_document("documents/unit_test_headers.docx");
    if !document_path.exists() {
        println!(
            "Skipping office_docx_headers: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for office_docx_headers: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(
        &result,
        &["application/vnd.openxmlformats-officedocument.wordprocessingml.document"],
    );
    assertions::assert_min_content_length(&result, 20);
}

#[test]
fn test_office_docx_lists() {
    let document_path = resolve_document("documents/unit_test_lists.docx");
    if !document_path.exists() {
        println!(
            "Skipping office_docx_lists: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for office_docx_lists: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(
        &result,
        &["application/vnd.openxmlformats-officedocument.wordprocessingml.document"],
    );
    assertions::assert_min_content_length(&result, 20);
}

#[test]
fn test_office_docx_tables() {
    let document_path = resolve_document("documents/docx_tables.docx");
    if !document_path.exists() {
        println!(
            "Skipping office_docx_tables: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for office_docx_tables: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(
        &result,
        &["application/vnd.openxmlformats-officedocument.wordprocessingml.document"],
    );
    assertions::assert_min_content_length(&result, 20);
}

#[test]
fn test_office_ppt_legacy() {
    let document_path = resolve_document("legacy_office/simple.ppt");
    if !document_path.exists() {
        println!(
            "Skipping office_ppt_legacy: missing document at {}",
            document_path.display()
        );
        return;
    }
    if !e2e_rust::external_tool_available("libreoffice") {
        println!("Skipping office_ppt_legacy: libreoffice not available on PATH");
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(KreuzbergError::MissingDependency(dep)) => {
            println!("Skipping office_ppt_legacy: missing dependency {dep}", dep = dep);
            return;
        }
        Err(KreuzbergError::UnsupportedFormat(fmt)) => {
            println!(
                "Skipping office_ppt_legacy: unsupported format {fmt} (requires optional tool)",
                fmt = fmt
            );
            return;
        }
        Err(err) => panic!("Extraction failed for office_ppt_legacy: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["application/vnd.ms-powerpoint"]);
    assertions::assert_min_content_length(&result, 10);
}

#[test]
fn test_office_pptx_basic() {
    let document_path = resolve_document("presentations/simple.pptx");
    if !document_path.exists() {
        println!(
            "Skipping office_pptx_basic: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for office_pptx_basic: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(
        &result,
        &["application/vnd.openxmlformats-officedocument.presentationml.presentation"],
    );
    assertions::assert_min_content_length(&result, 50);
}

#[test]
fn test_office_pptx_images() {
    let document_path = resolve_document("presentations/powerpoint_with_image.pptx");
    if !document_path.exists() {
        println!(
            "Skipping office_pptx_images: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for office_pptx_images: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(
        &result,
        &["application/vnd.openxmlformats-officedocument.presentationml.presentation"],
    );
    assertions::assert_min_content_length(&result, 20);
}

#[test]
fn test_office_pptx_pitch_deck() {
    let document_path = resolve_document("presentations/pitch_deck_presentation.pptx");
    if !document_path.exists() {
        println!(
            "Skipping office_pptx_pitch_deck: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for office_pptx_pitch_deck: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(
        &result,
        &["application/vnd.openxmlformats-officedocument.presentationml.presentation"],
    );
    assertions::assert_min_content_length(&result, 100);
}

#[test]
fn test_office_xls_legacy() {
    let document_path = resolve_document("spreadsheets/test_excel.xls");
    if !document_path.exists() {
        println!(
            "Skipping office_xls_legacy: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for office_xls_legacy: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["application/vnd.ms-excel"]);
    assertions::assert_min_content_length(&result, 10);
}

#[test]
fn test_office_xlsx_basic() {
    let document_path = resolve_document("spreadsheets/stanley_cups.xlsx");
    if !document_path.exists() {
        println!(
            "Skipping office_xlsx_basic: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for office_xlsx_basic: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(
        &result,
        &["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
    );
    assertions::assert_min_content_length(&result, 10);
    assertions::assert_metadata_expectation(&result, "sheet_count", &serde_json::json!({"gte":1}));
}

#[test]
fn test_office_xlsx_multi_sheet() {
    let document_path = resolve_document("spreadsheets/excel_multi_sheet.xlsx");
    if !document_path.exists() {
        println!(
            "Skipping office_xlsx_multi_sheet: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for office_xlsx_multi_sheet: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(
        &result,
        &["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
    );
    assertions::assert_min_content_length(&result, 20);
    assertions::assert_metadata_expectation(&result, "sheet_count", &serde_json::json!({"gte":2}));
}

#[test]
fn test_office_xlsx_office_example() {
    let document_path = resolve_document("office/excel.xlsx");
    if !document_path.exists() {
        println!(
            "Skipping office_xlsx_office_example: missing document at {}",
            document_path.display()
        );
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for office_xlsx_office_example: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(
        &result,
        &["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
    );
    assertions::assert_min_content_length(&result, 10);
}
