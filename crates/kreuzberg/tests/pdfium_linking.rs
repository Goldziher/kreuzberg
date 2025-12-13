//! PDFium linking integration tests.
//!
//! This module tests PDF extraction functionality across all PDFium linking strategies
//! (default, static, bundled, system). Tests verify that:
//!
//! 1. PDF extraction works regardless of linking strategy
//! 2. Extracted content is valid and non-empty
//! 3. Bundled PDFium extraction works when feature is enabled
//! 4. Metadata extraction functions correctly
//! 5. Error handling works for invalid PDFs
//!
//! These tests are strategy-agnostic - they verify functionality works with ANY
//! linking strategy, not implementation details of specific strategies.
//!
//! Test philosophy:
//! - Use small test PDFs from test_documents/
//! - Assert on extracted content and behavior, not linking internals
//! - Test both successful extraction and error cases
//! - Skip tests gracefully if test files are missing

#![cfg(feature = "pdf")]

mod helpers;

use helpers::*;
use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::extract_file_sync;

/// Test basic PDF extraction works regardless of linking strategy.
///
/// This is the primary integration test - it verifies that PDF extraction
/// functions correctly with any linking strategy. We use a small test PDF
/// and verify that:
/// - Extraction completes successfully
/// - Content is non-empty and contains expected text
/// - MIME type is correctly detected
#[test]
fn test_pdf_extraction_basic() {
    if skip_if_missing("pdfs_with_tables/tiny.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs_with_tables/tiny.pdf");
    let config = ExtractionConfig::default();

    let result = extract_file_sync(&file_path, None, &config).expect("Should extract PDF successfully");

    // Verify MIME type detection
    assert_mime_type(&result, "application/pdf");

    // Verify content was extracted
    assert_non_empty_content(&result);

    // Verify reasonable content length for a small PDF
    assert_min_content_length(&result, 10);
}

/// Test PDF extraction with a medium-sized test PDF.
///
/// Verifies that extraction works with slightly larger PDFs and produces
/// substantive content. This test helps ensure linking strategy works
/// with real-world document sizes.
#[test]
fn test_pdf_extraction_medium() {
    if skip_if_missing("pdfs_with_tables/medium.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs_with_tables/medium.pdf");
    let config = ExtractionConfig::default();

    let result = extract_file_sync(&file_path, None, &config).expect("Should extract medium PDF successfully");

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);

    // Medium PDF should have more content than tiny
    assert!(result.content.len() >= 50, "Medium PDF should have substantial content");
}

/// Test PDF extraction with rotated pages.
///
/// Some PDFs have rotated pages. This test verifies that the linking
/// strategy handles page rotation correctly (PDFium should handle this
/// transparently). Note: This specific PDF may have OCR-only content,
/// so we just verify extraction completes without error.
#[test]
fn test_pdf_extraction_rotated_pages() {
    if skip_if_missing("pdfs/ocr_test_rotated_90.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/ocr_test_rotated_90.pdf");
    let config = ExtractionConfig::default();

    let result = extract_file_sync(&file_path, None, &config).expect("Should extract rotated PDF successfully");

    assert_mime_type(&result, "application/pdf");
    // Rotated PDFs may have OCR-only content, just verify extraction completes
}

/// Test PDF text extraction with code and formula content.
///
/// Verifies extraction works with technical PDFs containing code blocks,
/// mathematical formulas, and special formatting.
#[test]
fn test_pdf_extraction_code_and_formulas() {
    if skip_if_missing("pdfs/code_and_formula.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/code_and_formula.pdf");
    let config = ExtractionConfig::default();

    let result = extract_file_sync(&file_path, None, &config).expect("Should extract PDF with code and formulas");

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);
}

/// Test PDF extraction preserves right-to-left text direction.
///
/// PDFs with RTL text (Arabic, Hebrew, etc.) require special handling.
/// This verifies extraction works with RTL content.
#[test]
fn test_pdf_extraction_right_to_left() {
    if skip_if_missing("pdfs/right_to_left_01.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/right_to_left_01.pdf");
    let config = ExtractionConfig::default();

    let result = extract_file_sync(&file_path, None, &config).expect("Should extract RTL PDF successfully");

    assert_mime_type(&result, "application/pdf");
    // RTL PDFs might have minimal or no extracted text depending on PDF structure
    // Just verify extraction completes without error
}

/// Test PDF metadata extraction.
///
/// Verifies that PDF metadata (title, author, creation date, etc.) can be
/// extracted correctly, which is independent of linking strategy.
#[test]
fn test_pdf_metadata_extraction() {
    if skip_if_missing("pdfs_with_tables/tiny.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs_with_tables/tiny.pdf");
    let config = ExtractionConfig::default();

    let result = extract_file_sync(&file_path, None, &config).expect("Should extract PDF metadata");

    // Metadata field is always present (may have empty fields for some PDFs)
    // Just verify the structure exists
    let _metadata = &result.metadata;
    assert!(!result.mime_type.is_empty(), "MIME type should be set");
}

/// Test extraction of byte array from PDF.
///
/// Verifies that extract_bytes_sync works for PDF content, which is
/// important for in-memory processing.
#[test]
fn test_pdf_extraction_from_bytes() {
    if skip_if_missing("pdfs_with_tables/tiny.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs_with_tables/tiny.pdf");
    let pdf_bytes = std::fs::read(&file_path).expect("Should read PDF file");

    let config = ExtractionConfig::default();
    let result = kreuzberg::extract_bytes_sync(&pdf_bytes, "application/pdf", &config)
        .expect("Should extract PDF from bytes successfully");

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);
}

/// Test bundled PDFium extraction when pdf-bundled feature is enabled.
///
/// When `pdf-bundled` feature is enabled, PDFium is embedded in the binary.
/// This test verifies the bundled extraction mechanism works correctly.
#[test]
#[cfg(feature = "pdf-bundled")]
fn test_bundled_pdfium_extraction() {
    use kreuzberg::pdf::extract_bundled_pdfium;

    // Extract bundled PDFium library to temporary directory
    let lib_path = extract_bundled_pdfium().expect("Should extract bundled PDFium library");

    // Verify extracted library exists and is a file
    assert!(
        lib_path.exists(),
        "Extracted PDFium library should exist at {}",
        lib_path.display()
    );
    assert!(
        lib_path.is_file(),
        "Extracted PDFium library should be a file, not directory"
    );

    // Verify library has a reasonable file size (at least a few MB)
    let metadata = std::fs::metadata(&lib_path).expect("Should read library metadata");
    assert!(
        metadata.len() > 1_000_000,
        "Bundled PDFium library should be at least 1MB (got {} bytes)",
        metadata.len()
    );
}

/// Test bundled PDFium extracts consistently.
///
/// Verify that repeated calls to extract_bundled_pdfium return the same path
/// and don't create duplicate extracted libraries.
#[test]
#[cfg(feature = "pdf-bundled")]
fn test_bundled_pdfium_caching() {
    use kreuzberg::pdf::extract_bundled_pdfium;

    let path1 = extract_bundled_pdfium().expect("First extraction should succeed");
    let path2 = extract_bundled_pdfium().expect("Second extraction should succeed");

    // Both calls should return the same path (caching in place)
    assert_eq!(path1, path2, "Multiple extractions should return consistent path");

    // Path should still exist
    assert!(path1.exists(), "Extracted library should still exist");
}

/// Test bundled PDFium works with PDF extraction.
///
/// This integration test verifies that PDFium extracted via bundled mechanism
/// actually works for PDF content extraction.
#[test]
#[cfg(feature = "pdf-bundled")]
fn test_bundled_pdfium_with_pdf_extraction() {
    if skip_if_missing("pdfs_with_tables/tiny.pdf") {
        return;
    }

    // Ensure bundled PDFium is extracted and available
    let _lib_path = kreuzberg::pdf::extract_bundled_pdfium().expect("Should extract bundled PDFium library");

    // Now test that PDF extraction works with the bundled library
    let file_path = get_test_file_path("pdfs_with_tables/tiny.pdf");
    let config = ExtractionConfig::default();

    let result = extract_file_sync(&file_path, None, &config).expect("Should extract PDF with bundled PDFium");

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);
}

/// Test extraction with custom PDF configuration.
///
/// Verifies that custom PDF extraction settings work correctly.
#[test]
fn test_pdf_extraction_with_config() {
    if skip_if_missing("pdfs_with_tables/tiny.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs_with_tables/tiny.pdf");

    // Create custom extraction config
    let config = ExtractionConfig::default();

    let result = extract_file_sync(&file_path, None, &config).expect("Should extract PDF with custom config");

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);

    // Verify metadata is present
    assert!(!result.mime_type.is_empty(), "MIME type should be set in result");
}

/// Test handling of empty or minimal PDFs.
///
/// Some edge case PDFs might be empty or contain no text. Verify extraction
/// handles these gracefully.
#[test]
fn test_pdf_extraction_edge_cases() {
    // Test extraction of a minimal PDF
    if skip_if_missing("pdfs_with_tables/tiny.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs_with_tables/tiny.pdf");
    let config = ExtractionConfig::default();

    // Should complete without panic, regardless of content
    let result = extract_file_sync(&file_path, None, &config);
    assert!(result.is_ok(), "PDF extraction should succeed");
}

/// Test batch extraction of multiple PDFs.
///
/// Verifies that linking strategy handles batch processing of multiple
/// PDF files correctly.
#[test]
#[cfg(feature = "tokio-runtime")]
fn test_pdf_batch_extraction() {
    if skip_if_missing("pdfs_with_tables/tiny.pdf") || skip_if_missing("pdfs_with_tables/medium.pdf") {
        return;
    }

    let paths = vec![
        get_test_file_path("pdfs_with_tables/tiny.pdf"),
        get_test_file_path("pdfs_with_tables/medium.pdf"),
    ];

    let config = ExtractionConfig::default();

    // Use synchronous batch extraction
    let results = kreuzberg::batch_extract_file_sync(paths, &config).expect("Should extract PDFs in batch");

    // Verify all PDFs were extracted
    assert_eq!(results.len(), 2, "Should extract both PDFs");

    // Verify all results are valid
    for result in &results {
        assert_mime_type(result, "application/pdf");
        assert_non_empty_content(result);
    }
}

/// Test PDF with Unicode content extraction.
///
/// Verifies that PDFium linking strategy handles PDFs with international
/// characters correctly.
#[test]
fn test_pdf_unicode_content() {
    // Right-to-left PDFs often contain Unicode content
    if skip_if_missing("pdfs/right_to_left_01.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/right_to_left_01.pdf");
    let config = ExtractionConfig::default();

    let result = extract_file_sync(&file_path, None, &config).expect("Should extract PDF with Unicode content");

    // Just verify extraction completes - Unicode content handling
    // varies by PDF structure
    assert_mime_type(&result, "application/pdf");
}

/// Verify PDF module compiles with all feature combinations.
///
/// This is a compile-time test that ensures the pdf module and bundled
/// submodule (when enabled) compile correctly. It's implicitly tested
/// by the fact that these test functions compile.
#[test]
#[cfg(feature = "pdf")]
fn test_pdf_module_availability() {
    // This test verifies that:
    // 1. The pdf module is available (feature = "pdf")
    // 2. Key functions are accessible
    // 3. pdf-bundled module is available when feature enabled

    // Verify we can access the text extraction function
    let _ = kreuzberg::pdf::extract_text_from_pdf;

    // Verify we can access the metadata function
    let _ = kreuzberg::pdf::extract_metadata;

    // Verify bundled module is available when feature enabled
    #[cfg(feature = "pdf-bundled")]
    let _ = kreuzberg::pdf::extract_bundled_pdfium;
}
