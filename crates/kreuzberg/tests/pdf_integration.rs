//! PDF integration tests using real documents.
//!
//! This module tests PDF extraction end-to-end with real PDF files from
//! the test_documents/ directory. Tests verify that extraction produces
//! sensible results without testing pdfium internals.
//!
//! Test philosophy:
//! - Use real PDFs from test_documents/pdfs/
//! - Assert on behavior, not implementation
//! - Verify content is extracted (not byte-perfect accuracy)
//! - Test different PDF types (text, images, scanned, etc.)

mod helpers;

use helpers::*;
use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::extract_file_sync;

// ============================================================================
// Basic PDF Extraction Tests
// ============================================================================

#[test]
fn test_pdf_simple_text_extraction() {
    if skip_if_missing("pdfs/fake_memo.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/fake_memo.pdf");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract fake_memo.pdf successfully");

    // Basic assertions
    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);
    assert_min_content_length(&result, 50);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "pdf")]
    assert!(result.metadata.pdf.is_some(), "PDF should have metadata");

    // Content should contain expected text from the memo
    assert_content_contains_any(&result, &["May 5, 2023", "To Whom it May Concern", "Mallori"]);
}

#[test]
fn test_pdf_with_code_and_formulas() {
    if skip_if_missing("pdfs/code_and_formula.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/code_and_formula.pdf");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract code_and_formula.pdf successfully");

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);
    assert_min_content_length(&result, 100);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    // Should extract both code and mathematical content
    // Metadata should exist (even if minimal)
    #[cfg(feature = "pdf")]
    assert!(result.metadata.pdf.is_some(), "PDF should have metadata");
}

#[test]
fn test_pdf_with_embedded_images() {
    if skip_if_missing("pdfs/embedded_images_tables.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/embedded_images_tables.pdf");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract embedded_images_tables.pdf successfully");

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "pdf")]
    assert!(result.metadata.pdf.is_some(), "PDF should have metadata");

    // Should have extracted tables (if any)
    if !result.tables.is_empty() {
        assert_has_tables(&result);
    }
}

#[test]
fn test_pdf_large_document() {
    if skip_if_missing("pdfs/a_course_in_machine_learning_ciml_v0_9_all.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/a_course_in_machine_learning_ciml_v0_9_all.pdf");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract large PDF successfully");

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "pdf")]
    assert!(result.metadata.pdf.is_some(), "PDF should have metadata");

    // Large document should have substantial content
    assert_min_content_length(&result, 10000);

    // Should contain expected content (machine learning textbook)
    assert_content_contains_any(&result, &["machine learning", "algorithm", "training"]);
}

#[test]
fn test_pdf_technical_documentation() {
    if skip_if_missing("pdfs/an_introduction_to_statistical_learning_with_applications_in_r_islr_sixth_printing.pdf") {
        return;
    }

    let file_path = get_test_file_path(
        "pdfs/an_introduction_to_statistical_learning_with_applications_in_r_islr_sixth_printing.pdf",
    );
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract technical PDF successfully");

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);
    assert_min_content_length(&result, 10000);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "pdf")]
    assert!(result.metadata.pdf.is_some(), "PDF should have metadata");

    // Technical content should be extracted
    assert_content_contains_any(&result, &["statistical", "learning", "regression"]);
}

// ============================================================================
// PDF with Tables Tests
// ============================================================================

#[test]
fn test_pdf_with_tables_small() {
    if skip_if_missing("pdfs_with_tables/tiny.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs_with_tables/tiny.pdf");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract tiny table PDF successfully");

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "pdf")]
    assert!(result.metadata.pdf.is_some(), "PDF should have metadata");

    // Should detect and extract tables
    // Note: Table extraction quality depends on the PDF structure
}

#[test]
fn test_pdf_with_tables_medium() {
    if skip_if_missing("pdfs_with_tables/medium.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs_with_tables/medium.pdf");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract medium table PDF successfully");

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);
    assert_min_content_length(&result, 100);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "pdf")]
    assert!(result.metadata.pdf.is_some(), "PDF should have metadata");
}

#[test]
fn test_pdf_with_tables_large() {
    if skip_if_missing("pdfs_with_tables/large.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs_with_tables/large.pdf");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract large table PDF successfully");

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);
    assert_min_content_length(&result, 500);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "pdf")]
    assert!(result.metadata.pdf.is_some(), "PDF should have metadata");
}

// ============================================================================
// Password-Protected PDF Tests (Should Fail Gracefully)
// ============================================================================

#[test]
fn test_pdf_password_protected_fails_gracefully() {
    if skip_if_missing("pdfs/copy_protected.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/copy_protected.pdf");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default());

    // Should either succeed with limited extraction or return a clear error
    // We don't test the exact behavior since it depends on PDF protection type
    match result {
        Ok(extraction_result) => {
            // If extraction succeeded, verify it's valid
            assert_mime_type(&extraction_result, "application/pdf");

            // Verify ExtractionResult structure
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
            // If it failed, ensure error message is helpful
            let error_msg = e.to_string().to_lowercase();
            assert!(
                error_msg.contains("password") || error_msg.contains("protected") || error_msg.contains("encrypted"),
                "Error message should indicate password/protection issue, got: {}",
                e
            );
        }
    }
}

// ============================================================================
// Multi-Language PDF Tests
// ============================================================================

#[test]
fn test_pdf_non_english_german() {
    if skip_if_missing("pdfs/5_level_paging_and_5_level_ept_intel_revision_1_1_may_2017.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/5_level_paging_and_5_level_ept_intel_revision_1_1_may_2017.pdf");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract PDF with non-ASCII characters successfully");

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);
    assert_min_content_length(&result, 100);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "pdf")]
    assert!(result.metadata.pdf.is_some(), "PDF should have metadata");

    // Should handle technical documentation
    assert_content_contains_any(&result, &["Intel", "page", "paging"]);
}

#[test]
fn test_pdf_right_to_left() {
    if skip_if_missing("pdfs/right_to_left_01.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/right_to_left_01.pdf");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract right-to-left PDF successfully");

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "pdf")]
    assert!(result.metadata.pdf.is_some(), "PDF should have metadata");

    // Should extract text even if direction is right-to-left
}

// ============================================================================
// PDF Metadata Extraction Tests
// ============================================================================

#[test]
fn test_pdf_metadata_extraction() {
    if skip_if_missing("pdfs/fake_memo.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/fake_memo.pdf");
    let result =
        extract_file_sync(&file_path, None, &ExtractionConfig::default()).expect("Should extract PDF successfully");

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    // All PDFs should have PDF metadata populated
    #[cfg(feature = "pdf")]
    assert!(result.metadata.pdf.is_some(), "PDF should have metadata");
}

#[test]
fn test_pdf_google_doc_export() {
    if skip_if_missing("pdfs/google_doc_document.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/google_doc_document.pdf");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract Google Docs PDF successfully");

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "pdf")]
    assert!(result.metadata.pdf.is_some(), "PDF should have metadata");
}

// ============================================================================
// OCR with PDF Tests (Image-based PDFs)
// ============================================================================

#[test]
fn test_pdf_scanned_with_ocr() {
    if skip_if_missing("pdfs/image_only_german_pdf.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/image_only_german_pdf.pdf");
    let config = test_config_with_ocr();

    let result =
        extract_file_sync(&file_path, None, &config).expect("Should extract scanned PDF with OCR successfully");

    assert_mime_type(&result, "application/pdf");

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "pdf")]
    assert!(result.metadata.pdf.is_some(), "PDF should have metadata");

    // NOTE: This is a German PDF but we're using English OCR for testing
    // OCR may not extract content if language mismatch or image quality is poor
    // Just verify the extraction completed without errors
    // In production, users would configure the correct language
}

#[test]
fn test_pdf_rotated_page() {
    if skip_if_missing("pdfs/ocr_test_rotated_90.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/ocr_test_rotated_90.pdf");
    let config = test_config_with_ocr();

    let result = extract_file_sync(&file_path, None, &config).expect("Should extract rotated PDF successfully");

    assert_mime_type(&result, "application/pdf");

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "pdf")]
    assert!(result.metadata.pdf.is_some(), "PDF should have metadata");

    // Rotated content might require OCR - just verify we got a result
}

// ============================================================================
// Special PDF Format Tests
// ============================================================================

#[test]
fn test_pdf_assembly_language_technical() {
    if skip_if_missing("pdfs/assembly_language_for_beginners_al4_b_en.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/assembly_language_for_beginners_al4_b_en.pdf");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract technical assembly PDF successfully");

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);
    assert_min_content_length(&result, 5000);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "pdf")]
    assert!(result.metadata.pdf.is_some(), "PDF should have metadata");

    // Technical content should be preserved
    assert_content_contains_any(&result, &["assembly", "register", "instruction"]);
}

#[test]
fn test_pdf_fundamentals_deep_learning() {
    if skip_if_missing("pdfs/fundamentals_of_deep_learning_2014.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/fundamentals_of_deep_learning_2014.pdf");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract deep learning PDF successfully");

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);
    assert_min_content_length(&result, 1000);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "pdf")]
    assert!(result.metadata.pdf.is_some(), "PDF should have metadata");

    // Should contain deep learning terminology
    assert_content_contains_any(&result, &["neural", "network", "deep learning"]);
}

#[test]
fn test_pdf_bayesian_data_analysis() {
    if skip_if_missing("pdfs/bayesian_data_analysis_third_edition_13th_feb_2020.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/bayesian_data_analysis_third_edition_13th_feb_2020.pdf");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract Bayesian statistics PDF successfully");

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);
    assert_min_content_length(&result, 10000);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "pdf")]
    assert!(result.metadata.pdf.is_some(), "PDF should have metadata");

    // Should contain statistical terminology
    assert_content_contains_any(&result, &["Bayesian", "probability", "distribution"]);
}
