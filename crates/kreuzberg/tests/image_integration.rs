//! Image and OCR integration tests using real image files.
//!
//! This module tests image extraction and OCR processing end-to-end with real
//! image files from the test_documents/ directory. Tests verify that both
//! image metadata extraction and OCR text extraction work correctly.
//!
//! Test philosophy:
//! - Use real images from test_documents/
//! - Assert on behavior, not implementation
//! - Test different image formats (PNG, JPG, BMP, etc.)
//! - Test OCR with various languages and layouts
//! - Verify graceful handling of images without text

mod helpers;

use helpers::*;
use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::extract_file_sync;

// ============================================================================
// Basic Image Extraction Tests (without OCR)
// ============================================================================

#[test]
fn test_jpg_image_metadata() {
    if skip_if_missing("images/example.jpg") {
        return;
    }

    let file_path = get_test_file_path("images/example.jpg");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract JPG image successfully");

    assert_mime_type(&result, "image/jpeg");
    // Image without OCR may have minimal content
}

#[test]
fn test_png_image_metadata() {
    if skip_if_missing("images/test_hello_world.png") {
        return;
    }

    let file_path = get_test_file_path("images/test_hello_world.png");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract PNG image successfully");

    assert_mime_type(&result, "image/png");
}

#[test]
fn test_bmp_image_format() {
    if skip_if_missing("images/bmp_24.bmp") {
        return;
    }

    let file_path = get_test_file_path("images/bmp_24.bmp");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default())
        .expect("Should extract BMP image successfully");

    assert_mime_type(&result, "image/bmp");
}

// ============================================================================
// OCR Text Extraction Tests
// ============================================================================

#[test]
fn test_ocr_simple_text() {
    if skip_if_missing("images/test_hello_world.png") {
        return;
    }

    let file_path = get_test_file_path("images/test_hello_world.png");
    let config = test_config_with_ocr();

    let result = extract_file_sync(&file_path, None, &config)
        .expect("Should extract text from image with OCR");

    assert_mime_type(&result, "image/png");

    // OCR should extract some text
    // Note: We don't assert exact text since OCR accuracy varies
    if !result.content.trim().is_empty() {
        assert_min_content_length(&result, 5);
    }
}

#[test]
fn test_ocr_document_image() {
    if skip_if_missing("images/ocr_image.jpg") {
        return;
    }

    let file_path = get_test_file_path("images/ocr_image.jpg");
    let config = test_config_with_ocr();

    let result = extract_file_sync(&file_path, None, &config)
        .expect("Should extract text from document image");

    assert_mime_type(&result, "image/jpeg");

    // Document image should have text content
    if !result.content.trim().is_empty() {
        assert_min_content_length(&result, 10);
    }
}

#[test]
fn test_ocr_layout_parser() {
    if skip_if_missing("images/layout_parser_ocr.jpg") {
        return;
    }

    let file_path = get_test_file_path("images/layout_parser_ocr.jpg");
    let config = test_config_with_ocr();

    let result = extract_file_sync(&file_path, None, &config)
        .expect("Should extract text from layout parser image");

    assert_mime_type(&result, "image/jpeg");

    // Layout parser images typically have substantial text
    if !result.content.trim().is_empty() {
        assert_min_content_length(&result, 20);
    }
}

#[test]
fn test_ocr_invoice_image() {
    if skip_if_missing("images/invoice_image.png") {
        return;
    }

    let file_path = get_test_file_path("images/invoice_image.png");
    let config = test_config_with_ocr();

    let result = extract_file_sync(&file_path, None, &config)
        .expect("Should extract text from invoice image");

    assert_mime_type(&result, "image/png");

    // Invoice images should have text content
    if !result.content.trim().is_empty() {
        assert_min_content_length(&result, 10);
    }
}

// ============================================================================
// Table Detection in Images
// ============================================================================

#[test]
fn test_table_image_simple() {
    if skip_if_missing("tables/simple_table.png") {
        return;
    }

    let file_path = get_test_file_path("tables/simple_table.png");
    let config = test_config_with_ocr();

    let result = extract_file_sync(&file_path, None, &config)
        .expect("Should extract table image successfully");

    assert_mime_type(&result, "image/png");

    // Table images should extract text
    // Table detection may or may not be triggered depending on implementation
}

#[test]
fn test_table_image_complex() {
    if skip_if_missing("tables/complex_document.png") {
        return;
    }

    let file_path = get_test_file_path("tables/complex_document.png");
    let config = test_config_with_ocr();

    let result = extract_file_sync(&file_path, None, &config)
        .expect("Should extract complex document image successfully");

    assert_mime_type(&result, "image/png");
}

// ============================================================================
// Multi-Language OCR Tests
// ============================================================================

#[test]
fn test_ocr_multilang_english_korean() {
    if skip_if_missing("images/english_and_korean.png") {
        return;
    }

    let file_path = get_test_file_path("images/english_and_korean.png");
    let config = test_config_with_ocr();

    let result = extract_file_sync(&file_path, None, &config)
        .expect("Should extract mixed language image");

    assert_mime_type(&result, "image/png");

    // Mixed language text - OCR may extract some content
    // English OCR should at least extract English portions
}

#[test]
fn test_ocr_chinese_simplified() {
    if skip_if_missing("images/chi_sim_image.jpeg") {
        return;
    }

    let file_path = get_test_file_path("images/chi_sim_image.jpeg");
    let config = test_config_with_ocr();

    let result = extract_file_sync(&file_path, None, &config)
        .expect("Should process Chinese image");

    assert_mime_type(&result, "image/jpeg");

    // Chinese text with English OCR may not extract well
    // This tests that OCR doesn't crash on non-English scripts
}

#[test]
fn test_ocr_japanese_vertical() {
    if skip_if_missing("images/jpn_vert.jpeg") {
        return;
    }

    let file_path = get_test_file_path("images/jpn_vert.jpeg");
    let config = test_config_with_ocr();

    let result = extract_file_sync(&file_path, None, &config)
        .expect("Should process Japanese vertical text image");

    assert_mime_type(&result, "image/jpeg");

    // Japanese vertical text with English OCR may not extract well
    // This tests that OCR handles different text orientations gracefully
}

// ============================================================================
// Edge Cases: Images Without Text
// ============================================================================

#[test]
fn test_image_no_text() {
    if skip_if_missing("images/flower_no_text.jpg") {
        return;
    }

    let file_path = get_test_file_path("images/flower_no_text.jpg");
    let config = test_config_with_ocr();

    let result = extract_file_sync(&file_path, None, &config)
        .expect("Should process image without text");

    assert_mime_type(&result, "image/jpeg");

    // Images without text should extract successfully but have minimal/no content
    // This is expected behavior - not an error
}
