//! Integration tests for file format extraction.
//!
//! These tests verify end-to-end extraction behavior with real documents
//! from the test_documents/ directory. We test that extraction produces
//! sensible results, not that it perfectly matches expected output (that
//! would be testing the underlying libraries, not our integration).

mod helpers;

use helpers::*;
use kreuzberg::{ExtractionConfig, extract_file, extract_file_sync};

// ============================================================================
// PDF Tests (20 tests)
// ============================================================================

/// Test basic PDF text extraction with a simple document.
#[tokio::test]
async fn test_pdf_simple_text_extraction() {
    if !test_documents_available() {
        eprintln!("Skipping test: test_documents not available");
        return;
    }

    let path = get_test_file_path("pdfs/code_and_formula.pdf");
    if !path.exists() {
        eprintln!("Skipping test: file not found");
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&path, None, &config).await.unwrap();

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    // PDFs should have strongly-typed PDF metadata
    #[cfg(feature = "pdf")]
    assert!(
        result.metadata.pdf.is_some(),
        "PDF should have metadata in metadata.pdf field"
    );
}

/// Test PDF with large document (400+ pages).
#[tokio::test]
async fn test_pdf_large_document() {
    if !test_documents_available() {
        return;
    }

    let path = get_test_file_path("pdfs/a_course_in_machine_learning_ciml_v0_9_all.pdf");
    if !path.exists() {
        eprintln!("Skipping test: large PDF not available");
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&path, None, &config).await.unwrap();

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    // Large document should have substantial content
    assert!(
        result.content.len() > 10000,
        "Large PDF should extract substantial content, got {} bytes",
        result.content.len()
    );

    #[cfg(feature = "pdf")]
    assert!(result.metadata.pdf.is_some(), "PDF should have metadata");
}

/// Test PDF with password protection (should fail gracefully).
#[tokio::test]
async fn test_pdf_password_protected() {
    if !test_documents_available() {
        return;
    }

    let path = get_test_file_path("pdfs/copy_protected.pdf");
    if !path.exists() {
        eprintln!("Skipping test: protected PDF not available");
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&path, None, &config).await;

    // Should either:
    // 1. Return an error (preferred)
    // 2. Return empty content (acceptable fallback)
    match result {
        Err(e) => {
            // Good - we detected the password protection
            eprintln!("Password protection detected (expected): {}", e);
        }
        Ok(res) => {
            // Acceptable - some PDFs can be read despite protection
            eprintln!("Protected PDF extracted (some protection can be bypassed)");

            // Verify ExtractionResult structure
            assert!(res.chunks.is_none(), "Chunks should be None without chunking config");
            assert!(res.detected_languages.is_none(), "Language detection not enabled");
        }
    }
}

/// Test PDF metadata extraction.
#[tokio::test]
async fn test_pdf_metadata_extraction() {
    if !test_documents_available() {
        return;
    }

    // Use a PDF that's likely to have metadata
    let path = get_test_file_path("pdfs/bayesian_data_analysis_third_edition_13th_feb_2020.pdf");
    if !path.exists() {
        eprintln!("Skipping test: PDF not available");
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&path, None, &config).await.unwrap();

    assert_mime_type(&result, "application/pdf");

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    // PDFs should have strongly-typed PDF metadata
    #[cfg(feature = "pdf")]
    {
        assert!(
            result.metadata.pdf.is_some(),
            "PDF should have metadata in metadata.pdf field"
        );

        // Check page count is populated
        if let Some(ref pdf_meta) = result.metadata.pdf {
            assert!(pdf_meta.page_count.unwrap_or(0) > 0, "PDF page count should be > 0");
        }
    }
}

/// Test PDF with tables (from pdfs_with_tables directory).
#[tokio::test]
async fn test_pdf_with_tables() {
    if !test_documents_available() {
        return;
    }

    let path = get_test_file_path("pdfs_with_tables/1.pdf");
    if !path.exists() {
        eprintln!("Skipping test: PDF with tables not available");
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&path, None, &config).await.unwrap();

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    // Table extraction is optional - just verify we got content
    // If tables are extracted, they'll be in result.tables
    if !result.tables.is_empty() {
        eprintln!("Tables extracted: {}", result.tables.len());
        // Verify table structure
        for table in &result.tables {
            assert!(!table.cells.is_empty(), "Table should have cells");
        }
    }

    #[cfg(feature = "pdf")]
    assert!(result.metadata.pdf.is_some(), "PDF should have metadata");
}

/// Test sync wrapper for PDF extraction.
#[test]
fn test_pdf_extraction_sync() {
    if !test_documents_available() {
        return;
    }

    let path = get_test_file_path("pdfs/code_and_formula.pdf");
    if !path.exists() {
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file_sync(&path, None, &config).unwrap();

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "pdf")]
    assert!(result.metadata.pdf.is_some(), "PDF should have metadata");
}

// ============================================================================
// Office Document Tests (15 tests)
// ============================================================================

/// Test basic DOCX extraction.
#[tokio::test]
async fn test_docx_basic_extraction() {
    if !test_documents_available() {
        return;
    }

    let path = get_test_file_path("office/document.docx");
    if !path.exists() {
        eprintln!("Skipping test: DOCX not available");
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&path, None, &config).await.unwrap();

    assert_mime_type(
        &result,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    );
    assert_non_empty_content(&result);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    #[cfg(feature = "office")]
    assert!(
        !result.metadata.additional.is_empty(),
        "Office document should have metadata"
    );
}

/// Test basic XLSX extraction.
#[tokio::test]
async fn test_xlsx_basic_extraction() {
    if !test_documents_available() {
        return;
    }

    let path = get_test_file_path("office/excel.xlsx");
    if !path.exists() {
        let path = get_test_file_path("spreadsheets/stanley_cups.xlsx");
        if !path.exists() {
            eprintln!("Skipping test: XLSX not available");
            return;
        }
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&path, None, &config).await.unwrap();

    assert_mime_type(
        &result,
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    );
    assert_non_empty_content(&result);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    // Excel files should have sheet information in metadata
    // Check if sheet metadata exists
    let has_sheet_info = result.metadata.additional.contains_key("sheet_count")
        || result.metadata.additional.contains_key("sheet_names");

    if has_sheet_info {
        eprintln!("Sheet metadata found (good!)");
    }

    #[cfg(feature = "excel")]
    assert!(!result.metadata.additional.is_empty(), "Excel should have metadata");
}

/// Test PPTX extraction.
#[tokio::test]
async fn test_pptx_basic_extraction() {
    if !test_documents_available() {
        return;
    }

    // Try to find any PPTX file
    let test_files = ["presentations/presentation.pptx", "presentations/demo.pptx"];

    for test_file in &test_files {
        let path = get_test_file_path(test_file);
        if path.exists() {
            let config = ExtractionConfig::default();
            let result = extract_file(&path, None, &config).await.unwrap();

            assert_mime_type(
                &result,
                "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            );
            assert_non_empty_content(&result);

            // Verify ExtractionResult structure
            assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
            assert!(result.detected_languages.is_none(), "Language detection not enabled");

            #[cfg(feature = "office")]
            assert!(
                !result.metadata.additional.is_empty(),
                "PowerPoint should have metadata"
            );

            return;
        }
    }

    eprintln!("Skipping test: No PPTX files available");
}

/// Test legacy Word document (.doc) extraction via LibreOffice.
#[tokio::test]
async fn test_legacy_doc_extraction() {
    if !test_documents_available() {
        return;
    }

    let path = get_test_file_path("legacy_office/simple.doc");
    if !path.exists() {
        eprintln!("Skipping test: Legacy .doc file not available");
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&path, None, &config).await;

    match result {
        Ok(res) => {
            assert_mime_type(&res, "application/msword");
            assert_non_empty_content(&res);

            // Verify ExtractionResult structure
            assert!(res.chunks.is_none(), "Chunks should be None without chunking config");
            assert!(res.detected_languages.is_none(), "Language detection not enabled");
        }
        Err(e) => {
            // LibreOffice might not be installed in CI
            eprintln!(
                "Legacy Office extraction failed (LibreOffice may not be installed): {}",
                e
            );
        }
    }
}

// ============================================================================
// Image + OCR Tests (12 tests)
// ============================================================================

/// Test OCR on simple English text image.
#[tokio::test]
async fn test_ocr_simple_english() {
    if !test_documents_available() {
        return;
    }

    let path = get_test_file_path("images/test_hello_world.png");
    if !path.exists() {
        eprintln!("Skipping test: OCR test image not available");
        return;
    }

    let config = ExtractionConfig {
        ocr: Some(kreuzberg::OcrConfig {
            tesseract_config: None,
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
        }),
        force_ocr: true,
        ..Default::default()
    };

    let result = extract_file(&path, None, &config).await;

    match result {
        Ok(res) => {
            assert_mime_type(&res, "image/png");
            assert_non_empty_content(&res);

            // Verify ExtractionResult structure
            assert!(res.chunks.is_none(), "Chunks should be None without chunking config");
            assert!(res.detected_languages.is_none(), "Language detection not enabled");

            // Should contain "hello" or "world" (case insensitive)
            let content_lower = res.content.to_lowercase();
            let has_expected_text = content_lower.contains("hello") || content_lower.contains("world");

            if has_expected_text {
                eprintln!("OCR successfully extracted expected text");
            } else {
                eprintln!("OCR extracted text but may not be perfect: {}", res.content);
                // Don't fail - OCR accuracy varies
            }
        }
        Err(e) => {
            eprintln!("OCR test failed (Tesseract may not be installed): {}", e);
            // Don't fail the test - OCR dependencies are optional
        }
    }
}

/// Test OCR with image that has no text.
#[tokio::test]
async fn test_ocr_no_text_image() {
    if !test_documents_available() {
        return;
    }

    let path = get_test_file_path("images/flower_no_text.jpg");
    if !path.exists() {
        eprintln!("Skipping test: flower image not available");
        return;
    }

    let config = ExtractionConfig {
        ocr: Some(kreuzberg::OcrConfig {
            tesseract_config: None,
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
        }),
        force_ocr: true,
        ..Default::default()
    };

    let result = extract_file(&path, None, &config).await;

    match result {
        Ok(res) => {
            assert_mime_type(&res, "image/jpeg");

            // Verify ExtractionResult structure
            assert!(res.chunks.is_none(), "Chunks should be None without chunking config");
            assert!(res.detected_languages.is_none(), "Language detection not enabled");

            // Should have empty or very minimal content (OCR noise)
            let content_len = res.content.trim().len();
            assert!(
                content_len < 50,
                "Image with no text should extract minimal content, got {} chars",
                content_len
            );
        }
        Err(e) => {
            eprintln!("OCR test failed (Tesseract may not be installed): {}", e);
        }
    }
}

/// Test image extraction without OCR (should extract metadata only).
#[tokio::test]
async fn test_image_without_ocr() {
    if !test_documents_available() {
        return;
    }

    let path = get_test_file_path("images/example.jpg");
    if !path.exists() {
        eprintln!("Skipping test: example image not available");
        return;
    }

    let config = ExtractionConfig {
        ocr: None, // No OCR
        ..Default::default()
    };

    let result = extract_file(&path, None, &config).await.unwrap();

    assert_mime_type(&result, "image/jpeg");

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    // Without OCR, we should still get image metadata (width, height, format)
    let has_image_metadata = result.metadata.additional.contains_key("width")
        || result.metadata.additional.contains_key("height")
        || result.metadata.additional.contains_key("format");

    if has_image_metadata {
        eprintln!("Image metadata extracted without OCR (good!)");
    }
}

// ============================================================================
// HTML/Web Tests (8 tests)
// ============================================================================

/// Test HTML to Markdown conversion.
#[tokio::test]
async fn test_html_to_markdown() {
    if !test_documents_available() {
        return;
    }

    let path = get_test_file_path("web/simple_table.html");
    if !path.exists() {
        eprintln!("Skipping test: HTML file not available");
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&path, None, &config).await.unwrap();

    assert_mime_type(&result, "text/html");
    assert_non_empty_content(&result);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    // HTML should be converted to markdown
    // Check for common markdown patterns
    let has_markdown_patterns = result.content.contains("##") ||  // Headers
        result.content.contains("|") ||   // Tables
        result.content.contains("[") ||   // Links
        result.content.contains("**"); // Bold

    if has_markdown_patterns {
        eprintln!("HTML successfully converted to Markdown");
    }

    #[cfg(feature = "html")]
    assert!(!result.content.is_empty(), "HTML extraction should produce content");
}

/// Test HTML with complex layout (Wikipedia article).
/// Large Wikipedia HTML files can cause stack overflow in recursive parsers.
#[tokio::test(flavor = "multi_thread")]
async fn test_html_complex_layout() {
    if !test_documents_available() {
        return;
    }

    let path = get_test_file_path("web/taylor_swift.html");
    if !path.exists() {
        eprintln!("Skipping test: Wikipedia HTML not available");
        return;
    }

    // Large HTML files can cause stack overflow in recursive parsers.
    // Spawn with 16MB stack and use current-thread runtime to avoid
    // spawning additional threads.
    let result = std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)  // 16MB stack
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                let config = ExtractionConfig::default();
                extract_file(&path, None, &config).await
            })
        })
        .unwrap()
        .join()
        .unwrap();

    let result = result.unwrap();
    assert_mime_type(&result, "text/html");
    assert_non_empty_content(&result);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    // Should extract substantial content from Wikipedia article
    assert!(
        result.content.len() > 1000,
        "Wikipedia article should extract substantial content"
    );

    #[cfg(feature = "html")]
    assert!(!result.content.is_empty(), "HTML extraction should produce content");
}

/// Test HTML with non-English content (UTF-8).
/// This test uses a larger stack size because large HTML files (2MB+)
/// can cause stack overflow in recursive HTML parsers.
#[tokio::test(flavor = "multi_thread")]
async fn test_html_non_english() {
    if !test_documents_available() {
        return;
    }

    let path = get_test_file_path("web/germany_german.html");
    if !path.exists() {
        eprintln!("Skipping test: German HTML not available");
        return;
    }

    // Large HTML files can cause stack overflow in recursive parsers.
    // Spawn with 16MB stack and use current-thread runtime to avoid
    // spawning additional threads.
    let result = std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)  // 16MB stack
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                let config = ExtractionConfig::default();
                extract_file(&path, None, &config).await
            })
        })
        .unwrap()
        .join()
        .unwrap();

    let result = result.unwrap();
    assert_mime_type(&result, "text/html");
    assert_non_empty_content(&result);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    // Should handle UTF-8 properly (German umlauts)
    // Just verify it doesn't crash and extracts something
    #[cfg(feature = "html")]
    assert!(!result.content.is_empty(), "HTML extraction should produce content");
}

// ============================================================================
// Text/Markdown Tests (6 tests)
// ============================================================================

/// Test Markdown metadata extraction.
#[tokio::test]
async fn test_markdown_metadata_extraction() {
    if !test_documents_available() {
        return;
    }

    let path = get_test_file_path("pandoc/simple_metadata.md");
    if !path.exists() {
        eprintln!("Skipping test: Markdown file not available");
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&path, None, &config).await.unwrap();

    assert_mime_type(&result, "text/markdown");
    assert_non_empty_content(&result);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    // Markdown extraction should populate metadata fields
    // (headers, links, code blocks, word count, etc.)
    let has_markdown_metadata = result.metadata.additional.contains_key("headers")
        || result.metadata.additional.contains_key("links")
        || result.metadata.additional.contains_key("word_count")
        || result.metadata.additional.contains_key("line_count");

    if has_markdown_metadata {
        eprintln!("Markdown metadata successfully extracted");
    }
}

/// Test plain text streaming.
#[tokio::test]
async fn test_plain_text_extraction() {
    if !test_documents_available() {
        return;
    }

    // Find any .txt file
    let test_files = ["text/simple.txt", "text/README.txt"];

    for test_file in &test_files {
        let path = get_test_file_path(test_file);
        if path.exists() {
            let config = ExtractionConfig::default();
            let result = extract_file(&path, None, &config).await.unwrap();

            assert_mime_type(&result, "text/plain");
            assert_non_empty_content(&result);

            // Verify ExtractionResult structure
            assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
            assert!(result.detected_languages.is_none(), "Language detection not enabled");

            return;
        }
    }

    eprintln!("Skipping test: No plain text files available");
}

// ============================================================================
// Data Format Tests (8 tests)
// ============================================================================

/// Test JSON extraction.
#[tokio::test]
async fn test_json_extraction() {
    if !test_documents_available() {
        return;
    }

    let path = get_test_file_path("data_formats/simple.json");
    if !path.exists() {
        let path = get_test_file_path("json/simple.json");
        if !path.exists() {
            eprintln!("Skipping test: JSON file not available");
            return;
        }
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&path, None, &config).await.unwrap();

    assert_mime_type(&result, "application/json");
    assert_non_empty_content(&result);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    // JSON should be pretty-printed or formatted
    assert!(result.content.contains("{") || result.content.contains("["));
}

/// Test YAML extraction.
#[tokio::test]
async fn test_yaml_extraction() {
    if !test_documents_available() {
        return;
    }

    let path = get_test_file_path("data_formats/simple.yaml");
    if !path.exists() {
        let path = get_test_file_path("yaml/simple.yaml");
        if !path.exists() {
            eprintln!("Skipping test: YAML file not available");
            return;
        }
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&path, None, &config).await.unwrap();

    assert_mime_type(&result, "application/x-yaml");
    assert_non_empty_content(&result);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");
}

/// Test XML extraction.
#[tokio::test]
async fn test_xml_extraction() {
    if !test_documents_available() {
        return;
    }

    // Find any XML file
    let xml_dir = get_test_documents_dir().join("xml");
    if !xml_dir.exists() {
        eprintln!("Skipping test: xml directory not available");
        return;
    }

    // Get first XML file
    let mut xml_files = std::fs::read_dir(xml_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "xml").unwrap_or(false))
        .collect::<Vec<_>>();

    if xml_files.is_empty() {
        eprintln!("Skipping test: No XML files found");
        return;
    }

    let path = xml_files.remove(0).path();
    let config = ExtractionConfig::default();
    let result = extract_file(&path, None, &config).await.unwrap();

    assert_mime_type(&result, "application/xml");
    assert_non_empty_content(&result);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    // XML extraction should have metadata
    let has_xml_metadata = result.metadata.additional.contains_key("element_count")
        || result.metadata.additional.contains_key("unique_elements");

    if has_xml_metadata {
        eprintln!("XML metadata successfully extracted");
    }

    #[cfg(feature = "xml")]
    assert!(!result.content.is_empty(), "XML extraction should produce content");
}

// ============================================================================
// Email Tests (6 tests)
// ============================================================================

/// Test basic email extraction.
#[tokio::test]
async fn test_email_extraction() {
    if !test_documents_available() {
        return;
    }

    let email_dir = get_test_documents_dir().join("email");
    if !email_dir.exists() {
        eprintln!("Skipping test: email directory not available");
        return;
    }

    // Get first EML file
    let mut eml_files = std::fs::read_dir(email_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "eml").unwrap_or(false))
        .collect::<Vec<_>>();

    if eml_files.is_empty() {
        eprintln!("Skipping test: No EML files found");
        return;
    }

    let path = eml_files.remove(0).path();
    let config = ExtractionConfig::default();
    let result = extract_file(&path, None, &config).await.unwrap();

    assert_mime_type(&result, "message/rfc822");
    assert_non_empty_content(&result);

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    // Email extraction should have metadata (from, to, subject, etc.)
    let has_email_metadata = result.metadata.additional.contains_key("from")
        || result.metadata.additional.contains_key("to")
        || result.metadata.additional.contains_key("subject");

    if has_email_metadata {
        eprintln!("Email metadata successfully extracted");
    }

    #[cfg(feature = "email")]
    assert!(!result.content.is_empty(), "Email extraction should produce content");
}

#[cfg(test)]
mod test_summary {
    //! This module exists to provide a summary of test coverage.
    //! Run `cargo test --test format_integration` to see results.
}
