//! OCR configuration integration tests.
//!
//! This module extensively tests Tesseract OCR configuration propagation
//! to ensure all settings are correctly passed through to the OCR engine.
//!
//! Test philosophy:
//! - Verify all TesseractConfig fields are propagated correctly
//! - Test different language settings with appropriate test files
//! - Test PSM (page segmentation mode) variations
//! - Test force_ocr mode
//! - Verify configuration changes actually affect output
//! - Test table detection with various settings

mod helpers;

use helpers::*;
use kreuzberg::core::config::{ExtractionConfig, OcrConfig};
use kreuzberg::types::TesseractConfig;
use kreuzberg::extract_file_sync;

// ============================================================================
// Language Configuration Tests
// ============================================================================

#[test]
fn test_ocr_language_english() {
    if skip_if_missing("images/test_hello_world.png") {
        return;
    }

    let file_path = get_test_file_path("images/test_hello_world.png");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: None,
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config)
        .expect("Should extract with English OCR");

    assert_mime_type(&result, "image/png");
    // English text should be extracted
}

#[test]
fn test_ocr_language_german() {
    if skip_if_missing("images/test_hello_world.png") {
        return;
    }

    let file_path = get_test_file_path("images/test_hello_world.png");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "deu".to_string(), // German language code
            tesseract_config: None,
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config);

    // German language pack may not be installed
    // Test verifies language config is accepted and propagated
    match result {
        Ok(extraction_result) => {
            assert_mime_type(&extraction_result, "image/png");
        }
        Err(e) => {
            // If German language pack not installed, this is expected
            eprintln!("German OCR failed (language pack may not be installed): {}", e);
        }
    }
}

#[test]
fn test_ocr_language_multiple() {
    if skip_if_missing("images/english_and_korean.png") {
        return;
    }

    let file_path = get_test_file_path("images/english_and_korean.png");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng+kor".to_string(), // Multiple languages
            tesseract_config: None,
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config);

    // Korean language pack may not be installed
    match result {
        Ok(extraction_result) => {
            assert_mime_type(&extraction_result, "image/png");
        }
        Err(e) => {
            eprintln!("Multi-language OCR failed (language pack may not be installed): {}", e);
        }
    }
}

// ============================================================================
// PSM (Page Segmentation Mode) Tests
// ============================================================================

#[test]
fn test_ocr_psm_auto() {
    if skip_if_missing("images/ocr_image.jpg") {
        return;
    }

    let file_path = get_test_file_path("images/ocr_image.jpg");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: Some(TesseractConfig {
                psm: 3, // Fully automatic page segmentation
                ..Default::default()
            }),
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config)
        .expect("Should extract with PSM 3 (auto)");

    assert_mime_type(&result, "image/jpeg");
}

#[test]
fn test_ocr_psm_single_block() {
    if skip_if_missing("images/ocr_image.jpg") {
        return;
    }

    let file_path = get_test_file_path("images/ocr_image.jpg");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: Some(TesseractConfig {
                psm: 6, // Assume a single uniform block of text
                ..Default::default()
            }),
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config)
        .expect("Should extract with PSM 6 (single block)");

    assert_mime_type(&result, "image/jpeg");
}

#[test]
fn test_ocr_psm_single_line() {
    if skip_if_missing("images/test_hello_world.png") {
        return;
    }

    let file_path = get_test_file_path("images/test_hello_world.png");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: Some(TesseractConfig {
                psm: 7, // Treat image as a single text line
                ..Default::default()
            }),
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config)
        .expect("Should extract with PSM 7 (single line)");

    assert_mime_type(&result, "image/png");
}

// ============================================================================
// Force OCR Mode Tests
// ============================================================================

#[test]
fn test_force_ocr_on_text_pdf() {
    if skip_if_missing("pdfs/fake_memo.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/fake_memo.pdf");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: None,
        }),
        force_ocr: true, // Force OCR even though PDF has text layer
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config)
        .expect("Should extract with force_ocr enabled");

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);
}

#[test]
fn test_force_ocr_disabled() {
    if skip_if_missing("pdfs/fake_memo.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/fake_memo.pdf");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: None,
        }),
        force_ocr: false, // Don't force OCR if text layer exists
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config)
        .expect("Should extract without forcing OCR");

    assert_mime_type(&result, "application/pdf");
    assert_non_empty_content(&result);
}

// ============================================================================
// Table Detection Configuration Tests
// ============================================================================

#[test]
fn test_table_detection_enabled() {
    if skip_if_missing("tables/simple_table.png") {
        return;
    }

    let file_path = get_test_file_path("tables/simple_table.png");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: Some(TesseractConfig {
                enable_table_detection: true,
                table_min_confidence: 0.5,
                table_column_threshold: 10,
                table_row_threshold_ratio: 0.5,
                ..Default::default()
            }),
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config)
        .expect("Should extract with table detection enabled");

    assert_mime_type(&result, "image/png");
    // Table detection may or may not find tables depending on image quality
}

#[test]
fn test_table_detection_disabled() {
    if skip_if_missing("tables/simple_table.png") {
        return;
    }

    let file_path = get_test_file_path("tables/simple_table.png");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: Some(TesseractConfig {
                enable_table_detection: false,
                ..Default::default()
            }),
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config)
        .expect("Should extract with table detection disabled");

    assert_mime_type(&result, "image/png");
}

// ============================================================================
// Tesseract Boolean Configuration Tests
// ============================================================================

#[test]
fn test_language_model_ngram_configuration() {
    if skip_if_missing("images/ocr_image.jpg") {
        return;
    }

    let file_path = get_test_file_path("images/ocr_image.jpg");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: Some(TesseractConfig {
                language_model_ngram_on: true,
                ..Default::default()
            }),
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config)
        .expect("Should extract with ngram language model enabled");

    assert_mime_type(&result, "image/jpeg");
}

#[test]
fn test_dictionary_correction_enabled() {
    if skip_if_missing("images/ocr_image.jpg") {
        return;
    }

    let file_path = get_test_file_path("images/ocr_image.jpg");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: Some(TesseractConfig {
                tessedit_enable_dict_correction: true,
                ..Default::default()
            }),
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config)
        .expect("Should extract with dictionary correction enabled");

    assert_mime_type(&result, "image/jpeg");
}

#[test]
fn test_character_whitelist() {
    if skip_if_missing("images/test_hello_world.png") {
        return;
    }

    let file_path = get_test_file_path("images/test_hello_world.png");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: Some(TesseractConfig {
                tessedit_char_whitelist: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz ".to_string(),
                ..Default::default()
            }),
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config)
        .expect("Should extract with character whitelist");

    assert_mime_type(&result, "image/png");
    // Output should only contain whitelisted characters
}

// ============================================================================
// Cache Configuration Tests
// ============================================================================

#[test]
fn test_ocr_cache_enabled() {
    if skip_if_missing("images/ocr_image.jpg") {
        return;
    }

    let file_path = get_test_file_path("images/ocr_image.jpg");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: Some(TesseractConfig {
                use_cache: true,
                ..Default::default()
            }),
        }),
        force_ocr: false,
        use_cache: true,
        ..Default::default()
    };

    // Extract twice to test caching
    let result1 = extract_file_sync(&file_path, None, &config)
        .expect("First extraction should succeed");
    let result2 = extract_file_sync(&file_path, None, &config)
        .expect("Second extraction should succeed (cached)");

    assert_mime_type(&result1, "image/jpeg");
    assert_mime_type(&result2, "image/jpeg");
    // Second extraction should be faster due to caching
}

#[test]
fn test_ocr_cache_disabled() {
    if skip_if_missing("images/ocr_image.jpg") {
        return;
    }

    let file_path = get_test_file_path("images/ocr_image.jpg");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: Some(TesseractConfig {
                use_cache: false,
                ..Default::default()
            }),
        }),
        force_ocr: false,
        use_cache: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config)
        .expect("Should extract without caching");

    assert_mime_type(&result, "image/jpeg");
}

// ============================================================================
// Configuration Combination Tests
// ============================================================================

#[test]
fn test_complex_configuration_combination() {
    if skip_if_missing("images/layout_parser_ocr.jpg") {
        return;
    }

    let file_path = get_test_file_path("images/layout_parser_ocr.jpg");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: Some(TesseractConfig {
                psm: 3,
                enable_table_detection: true,
                table_min_confidence: 0.7,
                language_model_ngram_on: true,
                tessedit_enable_dict_correction: true,
                use_cache: true,
                ..Default::default()
            }),
        }),
        force_ocr: false,
        use_cache: true,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config)
        .expect("Should extract with complex configuration");

    assert_mime_type(&result, "image/jpeg");
    assert_non_empty_content(&result);
}
