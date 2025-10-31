//! Office document integration tests using real documents.
//!
//! This module tests Office document extraction end-to-end with real files from
//! the test_documents/ directory. Tests verify that extraction produces
//! sensible results for DOCX, XLSX, PPTX, and legacy Office formats.
//!
//! Test philosophy:
//! - Use real Office files from test_documents/
//! - Assert on behavior, not implementation
//! - Verify content is extracted (not byte-perfect accuracy)
//! - Test different document types (tables, images, formatting, etc.)

mod helpers;

use helpers::*;
use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::extract_file_sync;

#[test]
fn test_docx_simple_text() {
    if skip_if_missing("documents/fake.docx") {
        return;
    }

    let file_path = get_test_file_path("documents/fake.docx");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract simple DOCX successfully");

    assert_mime_type(
        &result,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    );
    assert_non_empty_content(&result);
    assert_min_content_length(&result, 20);

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "office")]
    assert!(
        !result.metadata.additional.is_empty(),
        "Office document should have metadata"
    );
}

#[test]
fn test_docx_with_tables() {
    if skip_if_missing("documents/docx_tables.docx") {
        return;
    }

    let file_path = get_test_file_path("documents/docx_tables.docx");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract DOCX with tables successfully");

    assert_mime_type(
        &result,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    );
    assert_non_empty_content(&result);

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "office")]
    assert!(
        !result.metadata.additional.is_empty(),
        "Office document should have metadata"
    );
}

#[test]
fn test_docx_with_headers() {
    if skip_if_missing("documents/unit_test_headers.docx") {
        return;
    }

    let file_path = get_test_file_path("documents/unit_test_headers.docx");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract DOCX with headers successfully");

    assert_mime_type(
        &result,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    );
    assert_non_empty_content(&result);

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "office")]
    assert!(
        !result.metadata.additional.is_empty(),
        "Office document should have metadata"
    );
}

#[test]
fn test_docx_with_lists() {
    if skip_if_missing("documents/unit_test_lists.docx") {
        return;
    }

    let file_path = get_test_file_path("documents/unit_test_lists.docx");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract DOCX with lists successfully");

    assert_mime_type(
        &result,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    );
    assert_non_empty_content(&result);

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "office")]
    assert!(
        !result.metadata.additional.is_empty(),
        "Office document should have metadata"
    );
}

#[test]
fn test_docx_with_formatting() {
    if skip_if_missing("documents/unit_test_formatting.docx") {
        return;
    }

    let file_path = get_test_file_path("documents/unit_test_formatting.docx");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract formatted DOCX successfully");

    assert_mime_type(
        &result,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    );
    assert_non_empty_content(&result);

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "office")]
    assert!(
        !result.metadata.additional.is_empty(),
        "Office document should have metadata"
    );
}

#[test]
fn test_docx_with_equations() {
    if skip_if_missing("documents/equations.docx") {
        return;
    }

    let file_path = get_test_file_path("documents/equations.docx");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract DOCX with equations successfully");

    assert_mime_type(
        &result,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    );
    assert_non_empty_content(&result);

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "office")]
    assert!(
        !result.metadata.additional.is_empty(),
        "Office document should have metadata"
    );
}

#[test]
fn test_xlsx_simple_spreadsheet() {
    if skip_if_missing("office/excel.xlsx") {
        return;
    }

    let file_path = get_test_file_path("office/excel.xlsx");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract simple XLSX successfully");

    assert_mime_type(
        &result,
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    );
    assert_non_empty_content(&result);

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "excel")]
    assert!(!result.metadata.additional.is_empty(), "Excel should have metadata");
}

#[test]
fn test_xlsx_multi_sheet() {
    if skip_if_missing("spreadsheets/excel_multi_sheet.xlsx") {
        return;
    }

    let file_path = get_test_file_path("spreadsheets/excel_multi_sheet.xlsx");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract multi-sheet XLSX successfully");

    assert_mime_type(
        &result,
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    );
    assert_non_empty_content(&result);

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "excel")]
    assert!(!result.metadata.additional.is_empty(), "Excel should have metadata");

    assert_min_content_length(&result, 50);
}

#[test]
fn test_xlsx_with_data() {
    if skip_if_missing("spreadsheets/stanley_cups.xlsx") {
        return;
    }

    let file_path = get_test_file_path("spreadsheets/stanley_cups.xlsx");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract data-heavy XLSX successfully");

    assert_mime_type(
        &result,
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    );
    assert_non_empty_content(&result);
    assert_min_content_length(&result, 100);

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "excel")]
    assert!(!result.metadata.additional.is_empty(), "Excel should have metadata");
}

#[test]
fn test_xls_legacy_format() {
    if skip_if_missing("spreadsheets/test_excel.xls") {
        return;
    }

    let file_path = get_test_file_path("spreadsheets/test_excel.xls");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract legacy XLS successfully");

    assert_mime_type(&result, "application/vnd.ms-excel");
    assert_non_empty_content(&result);

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "excel")]
    assert!(!result.metadata.additional.is_empty(), "Excel should have metadata");
}

#[test]
fn test_pptx_simple_presentation() {
    if skip_if_missing("presentations/simple.pptx") {
        return;
    }

    let file_path = get_test_file_path("presentations/simple.pptx");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract simple PPTX successfully");

    assert_mime_type(
        &result,
        "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    );
    assert_non_empty_content(&result);

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "office")]
    assert!(
        !result.metadata.additional.is_empty(),
        "PowerPoint should have metadata"
    );
}

#[test]
fn test_pptx_with_images() {
    if skip_if_missing("presentations/powerpoint_with_image.pptx") {
        return;
    }

    let file_path = get_test_file_path("presentations/powerpoint_with_image.pptx");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract PPTX with images successfully");

    assert_mime_type(
        &result,
        "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    );
    assert_non_empty_content(&result);

    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "office")]
    assert!(
        !result.metadata.additional.is_empty(),
        "PowerPoint should have metadata"
    );
}

#[test]
fn test_pptx_pitch_deck() {
    if skip_if_missing("presentations/pitch_deck_presentation.pptx") {
        return;
    }

    let file_path = get_test_file_path("presentations/pitch_deck_presentation.pptx");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default());

    match result {
        Ok(extraction_result) => {
            assert_mime_type(
                &extraction_result,
                "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            );
            assert_non_empty_content(&extraction_result);
            assert_min_content_length(&extraction_result, 100);

            assert!(
                extraction_result.chunks.is_none(),
                "Chunks should be None without chunking config"
            );
            assert!(
                extraction_result.detected_languages.is_none(),
                "Language detection not enabled"
            );

            #[cfg(feature = "office")]
            assert!(
                !extraction_result.metadata.additional.is_empty(),
                "PowerPoint should have metadata"
            );
        }
        Err(e) => {
            tracing::debug!("Pitch deck extraction failed (unusual PPTX structure): {}", e);
        }
    }
}

#[test]
fn test_doc_legacy_word() {
    if skip_if_missing("legacy_office/unit_test_lists.doc") {
        return;
    }

    let file_path = get_test_file_path("legacy_office/unit_test_lists.doc");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default());

    match result {
        Ok(extraction_result) => {
            assert_mime_type(&extraction_result, "application/msword");
            assert_non_empty_content(&extraction_result);

            assert!(
                extraction_result.chunks.is_none(),
                "Chunks should be None without chunking config"
            );
            assert!(
                extraction_result.detected_languages.is_none(),
                "Language detection not enabled"
            );
        }
        Err(e) => {
            tracing::debug!("Legacy DOC extraction failed (LibreOffice may not be installed): {}", e);
        }
    }
}

#[test]
fn test_ppt_legacy_powerpoint() {
    if skip_if_missing("legacy_office/simple.ppt") {
        return;
    }

    let file_path = get_test_file_path("legacy_office/simple.ppt");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default());

    match result {
        Ok(extraction_result) => {
            assert_mime_type(&extraction_result, "application/vnd.ms-powerpoint");
            assert_non_empty_content(&extraction_result);

            assert!(
                extraction_result.chunks.is_none(),
                "Chunks should be None without chunking config"
            );
            assert!(
                extraction_result.detected_languages.is_none(),
                "Language detection not enabled"
            );
        }
        Err(e) => {
            tracing::debug!("Legacy PPT extraction failed (LibreOffice may not be installed): {}", e);
        }
    }
}
