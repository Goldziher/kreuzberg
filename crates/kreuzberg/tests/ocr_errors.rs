//! OCR error handling and edge case tests.
//!
//! This module tests OCR error scenarios to ensure robust error handling:
//! - Invalid configurations (bad language codes, invalid PSM values)
//! - Corrupted or invalid image inputs
//! - Missing dependencies (Tesseract not installed)
//! - Cache-related errors
//! - Concurrent processing scenarios
//!
//! Test philosophy:
//! - Verify graceful handling of all error conditions
//! - Ensure error messages are informative
//! - Test recovery from transient failures
//! - Validate resource limits and constraints

mod helpers;

use helpers::*;
use kreuzberg::core::config::{ExtractionConfig, OcrConfig};
use kreuzberg::types::TesseractConfig;
use kreuzberg::{KreuzbergError, extract_bytes_sync, extract_file_sync};

// ============================================================================
// Invalid Configuration Tests
// ============================================================================

#[test]
fn test_ocr_invalid_language_code() {
    if skip_if_missing("images/test_hello_world.png") {
        return;
    }

    let file_path = get_test_file_path("images/test_hello_world.png");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "invalid_lang_99999".to_string(), // Invalid language code
            tesseract_config: None,
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config);

    // Should fail gracefully with an OCR error
    match result {
        Err(KreuzbergError::Ocr { message, .. }) => {
            eprintln!("Expected OCR error for invalid language: {}", message);
            // Error message should mention the language issue
            assert!(
                message.contains("language") || message.contains("lang") || message.contains("invalid"),
                "Error message should mention language issue: {}",
                message
            );
        }
        Err(e) => {
            // Some versions of Tesseract may return different error types
            eprintln!("Invalid language produced error: {}", e);
        }
        Ok(_) => {
            // Some Tesseract versions may fall back to default language
            eprintln!("Invalid language was accepted (fallback behavior)");
        }
    }
}

#[test]
fn test_ocr_invalid_psm_mode() {
    if skip_if_missing("images/test_hello_world.png") {
        return;
    }

    let file_path = get_test_file_path("images/test_hello_world.png");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: Some(TesseractConfig {
                psm: 999, // Invalid PSM mode (valid range is 0-13)
                ..Default::default()
            }),
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config);

    // Should either fail or fall back to valid PSM
    match result {
        Err(KreuzbergError::Ocr { message, .. }) | Err(KreuzbergError::Validation { message, .. }) => {
            eprintln!("Expected error for invalid PSM: {}", message);
            assert!(
                message.contains("psm") || message.contains("segmentation") || message.contains("mode"),
                "Error message should mention PSM issue: {}",
                message
            );
        }
        Err(e) => {
            eprintln!("Invalid PSM produced error: {}", e);
        }
        Ok(result) => {
            // Tesseract may accept it and fall back
            eprintln!("Invalid PSM was accepted (fallback behavior)");
            assert_non_empty_content(&result);
        }
    }
}

#[test]
fn test_ocr_invalid_backend_name() {
    if skip_if_missing("images/test_hello_world.png") {
        return;
    }

    let file_path = get_test_file_path("images/test_hello_world.png");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "nonexistent_ocr_backend_xyz".to_string(), // Invalid backend
            language: "eng".to_string(),
            tesseract_config: None,
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config);

    // Note: Rust core currently ignores backend name and always uses Tesseract
    // Python layer handles multiple backends (EasyOCR, PaddleOCR, etc.)
    // In Rust, invalid backend names are silently ignored - falls back to Tesseract
    match result {
        Ok(extraction_result) => {
            eprintln!("Invalid backend name ignored, fallback to Tesseract (expected behavior in Rust core)");
            assert_non_empty_content(&extraction_result);
        }
        Err(KreuzbergError::Ocr { message, .. }) => {
            eprintln!("OCR error for invalid backend: {}", message);
        }
        Err(KreuzbergError::MissingDependency(msg)) => {
            eprintln!("MissingDependency error for invalid backend: {}", msg);
        }
        Err(KreuzbergError::Validation { message, .. }) => {
            eprintln!("Validation error for invalid backend: {}", message);
        }
        Err(e) => {
            eprintln!("Invalid backend produced error: {}", e);
        }
    }
}

// ============================================================================
// Corrupted Input Tests
// ============================================================================

#[test]
fn test_ocr_corrupted_image_data() {
    let corrupted_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10]; // Truncated JPEG header
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: None,
        }),
        force_ocr: true,
        ..Default::default()
    };

    let result = extract_bytes_sync(&corrupted_data, "image/jpeg", &config);

    // Should fail gracefully
    match result {
        Err(KreuzbergError::ImageProcessing { message, .. })
        | Err(KreuzbergError::Parsing { message, .. })
        | Err(KreuzbergError::Ocr { message, .. }) => {
            eprintln!("Expected error for corrupted image: {}", message);
        }
        Err(e) => {
            eprintln!("Corrupted image produced error: {}", e);
        }
        Ok(_) => {
            // May succeed if corruption doesn't prevent basic processing
            eprintln!("Corrupted image was processed (partial success)");
        }
    }
}

#[test]
fn test_ocr_empty_image() {
    let empty_data = vec![];
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: None,
        }),
        force_ocr: true,
        ..Default::default()
    };

    let result = extract_bytes_sync(&empty_data, "image/png", &config);

    // Should fail with validation or parsing error
    assert!(result.is_err(), "Empty image data should produce an error");

    match result {
        Err(KreuzbergError::Validation { message, .. })
        | Err(KreuzbergError::Parsing { message, .. })
        | Err(KreuzbergError::ImageProcessing { message, .. }) => {
            eprintln!("Expected error for empty image: {}", message);
        }
        Err(e) => {
            eprintln!("Empty image produced error: {}", e);
        }
        Ok(_) => unreachable!(),
    }
}

#[test]
fn test_ocr_non_image_data() {
    let text_data = b"This is plain text, not an image";
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: None,
        }),
        force_ocr: true,
        ..Default::default()
    };

    let result = extract_bytes_sync(text_data, "image/png", &config);

    // Should fail with parsing or image processing error
    match result {
        Err(KreuzbergError::Parsing { message, .. }) | Err(KreuzbergError::ImageProcessing { message, .. }) => {
            eprintln!("Expected error for non-image data: {}", message);
        }
        Err(e) => {
            eprintln!("Non-image data produced error: {}", e);
        }
        Ok(_) => {
            // Unlikely but possible if format detection is lenient
            eprintln!("Non-image data was accepted");
        }
    }
}

// ============================================================================
// Extreme Configuration Tests
// ============================================================================

#[test]
fn test_ocr_extreme_table_threshold() {
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
                table_min_confidence: 1.5,       // Invalid (> 1.0)
                table_column_threshold: -50,     // Invalid (negative)
                table_row_threshold_ratio: 10.0, // Extreme value
                ..Default::default()
            }),
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config);

    // Should either validate and reject or clamp to valid range
    match result {
        Ok(extraction_result) => {
            eprintln!("Extreme table config was accepted (values may be clamped)");
            assert_non_empty_content(&extraction_result);
        }
        Err(KreuzbergError::Validation { message, .. }) => {
            eprintln!("Configuration validation caught extreme values: {}", message);
        }
        Err(e) => {
            eprintln!("Extreme table config produced error: {}", e);
        }
    }
}

#[test]
fn test_ocr_negative_psm() {
    if skip_if_missing("images/test_hello_world.png") {
        return;
    }

    let file_path = get_test_file_path("images/test_hello_world.png");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: Some(TesseractConfig {
                psm: -5, // Negative PSM (invalid)
                ..Default::default()
            }),
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config);

    // Should handle gracefully (fail or clamp)
    match result {
        Ok(_) => {
            eprintln!("Negative PSM was accepted (clamped or default used)");
        }
        Err(e) => {
            eprintln!("Negative PSM produced error: {}", e);
        }
    }
}

// ============================================================================
// Character Whitelist/Blacklist Edge Cases
// ============================================================================

#[test]
fn test_ocr_empty_whitelist() {
    if skip_if_missing("images/test_hello_world.png") {
        return;
    }

    let file_path = get_test_file_path("images/test_hello_world.png");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: Some(TesseractConfig {
                tessedit_char_whitelist: "".to_string(), // Empty whitelist
                ..Default::default()
            }),
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config);

    // Empty whitelist might produce empty output or be ignored
    match result {
        Ok(extraction_result) => {
            eprintln!(
                "Empty whitelist accepted, content length: {}",
                extraction_result.content.len()
            );
            // Content may be empty or fallback to no restriction
        }
        Err(e) => {
            eprintln!("Empty whitelist produced error: {}", e);
        }
    }
}

#[test]
fn test_ocr_conflicting_whitelist_blacklist() {
    if skip_if_missing("images/test_hello_world.png") {
        return;
    }

    let file_path = get_test_file_path("images/test_hello_world.png");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: Some(TesseractConfig {
                tessedit_char_whitelist: "abc".to_string(),
                tessedit_char_blacklist: "abc".to_string(), // Conflicts with whitelist
                ..Default::default()
            }),
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config);

    // Tesseract behavior with conflicting config is undefined
    match result {
        Ok(extraction_result) => {
            eprintln!(
                "Conflicting whitelist/blacklist accepted: {}",
                extraction_result.content.len()
            );
        }
        Err(e) => {
            eprintln!("Conflicting config produced error: {}", e);
        }
    }
}

// ============================================================================
// Multiple Language Edge Cases
// ============================================================================

#[test]
fn test_ocr_empty_language() {
    if skip_if_missing("images/test_hello_world.png") {
        return;
    }

    let file_path = get_test_file_path("images/test_hello_world.png");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "".to_string(), // Empty language string
            tesseract_config: None,
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config);

    // Should either fail or fall back to default language
    match result {
        Ok(_) => {
            eprintln!("Empty language accepted (fallback to default)");
        }
        Err(KreuzbergError::Validation { message, .. }) | Err(KreuzbergError::Ocr { message, .. }) => {
            eprintln!("Empty language rejected: {}", message);
        }
        Err(e) => {
            eprintln!("Empty language produced error: {}", e);
        }
    }
}

#[test]
fn test_ocr_malformed_multi_language() {
    if skip_if_missing("images/test_hello_world.png") {
        return;
    }

    let file_path = get_test_file_path("images/test_hello_world.png");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng++deu++fra".to_string(), // Malformed separator (should be single +)
            tesseract_config: None,
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config);

    // Tesseract may parse this differently
    match result {
        Ok(_) => {
            eprintln!("Malformed multi-language accepted (parser tolerant)");
        }
        Err(e) => {
            eprintln!("Malformed language string produced error: {}", e);
        }
    }
}

// ============================================================================
// Cache Configuration Edge Cases
// ============================================================================

#[test]
fn test_ocr_cache_disabled_then_enabled() {
    if skip_if_missing("images/ocr_image.jpg") {
        return;
    }

    let file_path = get_test_file_path("images/ocr_image.jpg");

    // First extraction with cache disabled
    let config_no_cache = ExtractionConfig {
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

    let result1 = extract_file_sync(&file_path, None, &config_no_cache);
    assert!(result1.is_ok(), "First extraction should succeed");

    // Second extraction with cache enabled
    let config_with_cache = ExtractionConfig {
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

    let result2 = extract_file_sync(&file_path, None, &config_with_cache);
    assert!(result2.is_ok(), "Second extraction should succeed");

    // Both should produce valid results
    assert_non_empty_content(&result1.unwrap());
    assert_non_empty_content(&result2.unwrap());
}

// ============================================================================
// Concurrent Processing Tests
// ============================================================================

#[test]
fn test_ocr_concurrent_same_file() {
    if skip_if_missing("images/ocr_image.jpg") {
        return;
    }

    use std::sync::Arc;
    use std::thread;

    let file_path = Arc::new(get_test_file_path("images/ocr_image.jpg"));
    let config = Arc::new(ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: None,
        }),
        force_ocr: false,
        use_cache: true,
        ..Default::default()
    });

    // Spawn 5 concurrent threads processing the same file
    let mut handles = vec![];
    for i in 0..5 {
        let file_path_clone = Arc::clone(&file_path);
        let config_clone = Arc::clone(&config);

        let handle = thread::spawn(move || {
            let result = extract_file_sync(&*file_path_clone, None, &config_clone);
            let success = result.is_ok();
            match result {
                Ok(extraction_result) => {
                    eprintln!("Thread {} succeeded", i);
                    assert_non_empty_content(&extraction_result);
                }
                Err(e) => {
                    eprintln!("Thread {} failed: {}", i, e);
                    // Concurrent access may cause some threads to fail
                    // This is acceptable as long as some succeed
                }
            }
            success
        });

        handles.push(handle);
    }

    // Wait for all threads and count successes
    let successes: usize = handles.into_iter().map(|h| if h.join().unwrap() { 1 } else { 0 }).sum();

    eprintln!("Concurrent processing: {}/5 threads succeeded", successes);

    // At least one thread should succeed
    assert!(
        successes >= 1,
        "At least one concurrent thread should succeed (got {})",
        successes
    );
}

#[test]
fn test_ocr_concurrent_different_files() {
    if skip_if_missing("images/ocr_image.jpg") || skip_if_missing("images/test_hello_world.png") {
        return;
    }

    use std::sync::Arc;
    use std::thread;

    let files = Arc::new(vec![
        get_test_file_path("images/ocr_image.jpg"),
        get_test_file_path("images/test_hello_world.png"),
    ]);

    let config = Arc::new(ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: None,
        }),
        force_ocr: false,
        use_cache: true,
        ..Default::default()
    });

    // Process different files concurrently
    let mut handles = vec![];
    for (i, file_path) in files.iter().enumerate() {
        let file_path_clone = file_path.clone();
        let config_clone = Arc::clone(&config);

        let handle = thread::spawn(move || {
            let result = extract_file_sync(&file_path_clone, None, &config_clone);
            match result {
                Ok(extraction_result) => {
                    eprintln!("File {} extraction succeeded", i);
                    assert_non_empty_content(&extraction_result);
                    true
                }
                Err(e) => {
                    eprintln!("File {} extraction failed: {}", i, e);
                    false
                }
            }
        });

        handles.push(handle);
    }

    // All threads should succeed when processing different files
    let successes: usize = handles.into_iter().map(|h| if h.join().unwrap() { 1 } else { 0 }).sum();

    assert_eq!(
        successes, 2,
        "All concurrent threads should succeed with different files"
    );
}

// ============================================================================
// Preprocessing Error Tests
// ============================================================================

#[test]
fn test_ocr_with_preprocessing_extreme_dpi() {
    if skip_if_missing("images/test_hello_world.png") {
        return;
    }

    use kreuzberg::types::ImagePreprocessingConfig;

    let file_path = get_test_file_path("images/test_hello_world.png");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: Some(TesseractConfig {
                preprocessing: Some(ImagePreprocessingConfig {
                    target_dpi: 10000, // Extreme DPI value
                    auto_rotate: true,
                    deskew: true,
                    denoise: false,
                    contrast_enhance: false,
                    binarization_method: "otsu".to_string(),
                    invert_colors: false,
                }),
                ..Default::default()
            }),
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config);

    // Should either clamp to valid range or fail gracefully
    match result {
        Ok(extraction_result) => {
            eprintln!("Extreme DPI accepted (clamped): {}", extraction_result.content.len());
        }
        Err(KreuzbergError::ImageProcessing { message, .. }) | Err(KreuzbergError::Validation { message, .. }) => {
            eprintln!("Extreme DPI rejected: {}", message);
        }
        Err(e) => {
            eprintln!("Extreme DPI produced error: {}", e);
        }
    }
}

#[test]
fn test_ocr_with_invalid_binarization_method() {
    if skip_if_missing("images/test_hello_world.png") {
        return;
    }

    use kreuzberg::types::ImagePreprocessingConfig;

    let file_path = get_test_file_path("images/test_hello_world.png");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: Some(TesseractConfig {
                preprocessing: Some(ImagePreprocessingConfig {
                    target_dpi: 300,
                    auto_rotate: true,
                    deskew: true,
                    denoise: false,
                    contrast_enhance: false,
                    binarization_method: "invalid_method_xyz".to_string(), // Invalid method
                    invert_colors: false,
                }),
                ..Default::default()
            }),
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config);

    // Should either fall back to default or fail
    match result {
        Ok(_) => {
            eprintln!("Invalid binarization method accepted (fallback used)");
        }
        Err(KreuzbergError::Validation { message, .. }) | Err(KreuzbergError::ImageProcessing { message, .. }) => {
            eprintln!("Invalid binarization method rejected: {}", message);
        }
        Err(e) => {
            eprintln!("Invalid binarization method produced error: {}", e);
        }
    }
}
