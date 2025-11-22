// Auto-generated tests for pdf fixtures.
#![allow(clippy::too_many_lines)]
use e2e_rust::{assertions, resolve_document};
use kreuzberg::core::config::ExtractionConfig;

#[test]
fn test_pdf_assembly_technical() {
    // Assembly language technical manual with large body of text.

    let document_path = resolve_document("pdfs/assembly_language_for_beginners_al4_b_en.pdf");
    if !document_path.exists() {
        println!("Skipping pdf_assembly_technical: missing document at {}", document_path.display());
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for pdf_assembly_technical: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["application/pdf"]);
    assertions::assert_min_content_length(&result, 5000);
    assertions::assert_content_contains_any(&result, &["assembly", "register", "instruction"]);
    assertions::assert_metadata_expectation(&result, "format_type", &serde_json::json!({"eq":"pdf"}));
}

#[test]
fn test_pdf_bayesian_data_analysis() {
    // Bayesian data analysis textbook PDF with large content volume.

    let document_path = resolve_document("pdfs/bayesian_data_analysis_third_edition_13th_feb_2020.pdf");
    if !document_path.exists() {
        println!("Skipping pdf_bayesian_data_analysis: missing document at {}", document_path.display());
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for pdf_bayesian_data_analysis: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["application/pdf"]);
    assertions::assert_min_content_length(&result, 10000);
    assertions::assert_content_contains_any(&result, &["Bayesian", "probability", "distribution"]);
    assertions::assert_metadata_expectation(&result, "format_type", &serde_json::json!({"eq":"pdf"}));
}

#[test]
fn test_pdf_code_and_formula() {
    // PDF containing code snippets and formulas should retain substantial content.

    let document_path = resolve_document("pdfs/code_and_formula.pdf");
    if !document_path.exists() {
        println!("Skipping pdf_code_and_formula: missing document at {}", document_path.display());
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for pdf_code_and_formula: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["application/pdf"]);
    assertions::assert_min_content_length(&result, 100);
}

#[test]
fn test_pdf_deep_learning() {
    // Deep learning textbook PDF to ensure long-form extraction quality.

    let document_path = resolve_document("pdfs/fundamentals_of_deep_learning_2014.pdf");
    if !document_path.exists() {
        println!("Skipping pdf_deep_learning: missing document at {}", document_path.display());
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for pdf_deep_learning: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["application/pdf"]);
    assertions::assert_min_content_length(&result, 1000);
    assertions::assert_content_contains_any(&result, &["neural", "network", "deep learning"]);
    assertions::assert_metadata_expectation(&result, "format_type", &serde_json::json!({"eq":"pdf"}));
}

#[test]
fn test_pdf_embedded_images() {
    // PDF with embedded images should extract text and tables when present.

    let document_path = resolve_document("pdfs/embedded_images_tables.pdf");
    if !document_path.exists() {
        println!("Skipping pdf_embedded_images: missing document at {}", document_path.display());
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for pdf_embedded_images: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["application/pdf"]);
    assertions::assert_min_content_length(&result, 50);
    assertions::assert_table_count(&result, Some(0), None);
}

#[test]
fn test_pdf_google_doc() {
    // Google Docs exported PDF to verify conversion fidelity.

    let document_path = resolve_document("pdfs/google_doc_document.pdf");
    if !document_path.exists() {
        println!("Skipping pdf_google_doc: missing document at {}", document_path.display());
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for pdf_google_doc: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["application/pdf"]);
    assertions::assert_min_content_length(&result, 50);
    assertions::assert_metadata_expectation(&result, "format_type", &serde_json::json!({"eq":"pdf"}));
}

#[test]
fn test_pdf_large_ciml() {
    // Large machine learning textbook PDF to stress extraction length.

    let document_path = resolve_document("pdfs/a_course_in_machine_learning_ciml_v0_9_all.pdf");
    if !document_path.exists() {
        println!("Skipping pdf_large_ciml: missing document at {}", document_path.display());
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for pdf_large_ciml: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["application/pdf"]);
    assertions::assert_min_content_length(&result, 10000);
    assertions::assert_content_contains_any(&result, &["machine learning", "algorithm", "training"]);
    assertions::assert_metadata_expectation(&result, "format_type", &serde_json::json!({"eq":"pdf"}));
}

#[test]
fn test_pdf_non_english_german() {
    // German technical PDF to ensure non-ASCII content extraction.

    let document_path = resolve_document("pdfs/5_level_paging_and_5_level_ept_intel_revision_1_1_may_2017.pdf");
    if !document_path.exists() {
        println!("Skipping pdf_non_english_german: missing document at {}", document_path.display());
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for pdf_non_english_german: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["application/pdf"]);
    assertions::assert_min_content_length(&result, 100);
    assertions::assert_content_contains_any(&result, &["Intel", "paging"]);
    assertions::assert_metadata_expectation(&result, "format_type", &serde_json::json!({"eq":"pdf"}));
}

#[test]
fn test_pdf_right_to_left() {
    // Right-to-left language PDF to verify RTL extraction.

    let document_path = resolve_document("pdfs/right_to_left_01.pdf");
    if !document_path.exists() {
        println!("Skipping pdf_right_to_left: missing document at {}", document_path.display());
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for pdf_right_to_left: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["application/pdf"]);
    assertions::assert_min_content_length(&result, 50);
    assertions::assert_metadata_expectation(&result, "format_type", &serde_json::json!({"eq":"pdf"}));
}

#[test]
fn test_pdf_simple_text() {
    // Simple text-heavy PDF should extract content without OCR or tables.

    let document_path = resolve_document("pdfs/fake_memo.pdf");
    if !document_path.exists() {
        println!("Skipping pdf_simple_text: missing document at {}", document_path.display());
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for pdf_simple_text: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["application/pdf"]);
    assertions::assert_min_content_length(&result, 50);
    assertions::assert_content_contains_any(&result, &["May 5, 2023", "To Whom it May Concern", "Mallori"]);
}

#[test]
fn test_pdf_tables_large() {
    // Large PDF with extensive tables to stress table extraction.

    let document_path = resolve_document("pdfs_with_tables/large.pdf");
    if !document_path.exists() {
        println!("Skipping pdf_tables_large: missing document at {}", document_path.display());
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for pdf_tables_large: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["application/pdf"]);
    assertions::assert_min_content_length(&result, 500);
    assertions::assert_table_count(&result, Some(1), None);
}

#[test]
fn test_pdf_tables_medium() {
    // Medium-sized PDF with multiple tables.

    let document_path = resolve_document("pdfs_with_tables/medium.pdf");
    if !document_path.exists() {
        println!("Skipping pdf_tables_medium: missing document at {}", document_path.display());
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for pdf_tables_medium: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["application/pdf"]);
    assertions::assert_min_content_length(&result, 100);
    assertions::assert_table_count(&result, Some(1), None);
}

#[test]
fn test_pdf_tables_small() {
    // Small PDF containing tables to validate table extraction.

    let document_path = resolve_document("pdfs_with_tables/tiny.pdf");
    if !document_path.exists() {
        println!("Skipping pdf_tables_small: missing document at {}", document_path.display());
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for pdf_tables_small: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["application/pdf"]);
    assertions::assert_min_content_length(&result, 50);
    assertions::assert_content_contains_all(&result, &["Table 1", "Selected Numbers", "Celsius", "Fahrenheit", "Water Freezing Point", "Water Boiling Point"]);
    assertions::assert_table_count(&result, Some(1), None);
}

#[test]
fn test_pdf_technical_stat_learning() {
    // Technical statistical learning PDF requiring substantial extraction.

    let document_path = resolve_document("pdfs/an_introduction_to_statistical_learning_with_applications_in_r_islr_sixth_printing.pdf");
    if !document_path.exists() {
        println!("Skipping pdf_technical_stat_learning: missing document at {}", document_path.display());
        return;
    }
    let config = ExtractionConfig::default();

    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for pdf_technical_stat_learning: {err:?}"),
        Ok(result) => result,
    };

    assertions::assert_expected_mime(&result, &["application/pdf"]);
    assertions::assert_min_content_length(&result, 10000);
    assertions::assert_content_contains_any(&result, &["statistical", "regression", "learning"]);
    assertions::assert_metadata_expectation(&result, "format_type", &serde_json::json!({"eq":"pdf"}));
}

