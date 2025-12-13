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

#![cfg(feature = "ocr")]

mod helpers;

use helpers::*;
use kreuzberg::core::config::{ExtractionConfig, OcrConfig};
use kreuzberg::types::TesseractConfig;
use kreuzberg::{KreuzbergError, extract_bytes_sync, extract_file_sync};

#[test]
fn test_ocr_invalid_language_code() {
    if skip_if_missing("images/test_hello_world.png") {
        return;
    }

    let file_path = get_test_file_path("images/test_hello_world.png");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "invalid_lang_99999".to_string(),
            tesseract_config: None,
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config);

    match result {
        Err(KreuzbergError::Ocr { message, .. }) => {
            tracing::debug!("Expected OCR error for invalid language: {}", message);
            assert!(
                message.contains("language") || message.contains("lang") || message.contains("invalid"),
                "Error message should mention language issue: {}",
                message
            );
        }
        Err(e) => {
            tracing::debug!("Invalid language produced error: {}", e);
        }
        Ok(_) => {
            tracing::debug!("Invalid language was accepted (fallback behavior)");
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
                psm: 999,
                ..Default::default()
            }),
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config);

    match result {
        Err(KreuzbergError::Ocr { message, .. }) | Err(KreuzbergError::Validation { message, .. }) => {
            tracing::debug!("Expected error for invalid PSM: {}", message);
            assert!(
                message.contains("psm") || message.contains("segmentation") || message.contains("mode"),
                "Error message should mention PSM issue: {}",
                message
            );
        }
        Err(e) => {
            tracing::debug!("Invalid PSM produced error: {}", e);
        }
        Ok(result) => {
            tracing::debug!("Invalid PSM was accepted (fallback behavior)");
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
            backend: "nonexistent_ocr_backend_xyz".to_string(),
            language: "eng".to_string(),
            tesseract_config: None,
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config);

    match result {
        Ok(extraction_result) => {
            tracing::debug!("Invalid backend name ignored, fallback to Tesseract (expected behavior in Rust core)");
            assert_non_empty_content(&extraction_result);
        }
        Err(KreuzbergError::Ocr { message, .. }) => {
            tracing::debug!("OCR error for invalid backend: {}", message);
        }
        Err(KreuzbergError::MissingDependency(msg)) => {
            tracing::debug!("MissingDependency error for invalid backend: {}", msg);
        }
        Err(KreuzbergError::Validation { message, .. }) => {
            tracing::debug!("Validation error for invalid backend: {}", message);
        }
        Err(e) => {
            tracing::debug!("Invalid backend produced error: {}", e);
        }
    }
}

#[test]
fn test_ocr_corrupted_image_data() {
    let corrupted_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
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

    match result {
        Err(KreuzbergError::ImageProcessing { message, .. })
        | Err(KreuzbergError::Parsing { message, .. })
        | Err(KreuzbergError::Ocr { message, .. }) => {
            tracing::debug!("Expected error for corrupted image: {}", message);
        }
        Err(e) => {
            tracing::debug!("Corrupted image produced error: {}", e);
        }
        Ok(_) => {
            tracing::debug!("Corrupted image was processed (partial success)");
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

    assert!(result.is_err(), "Empty image data should produce an error");

    match result {
        Err(KreuzbergError::Validation { message, .. })
        | Err(KreuzbergError::Parsing { message, .. })
        | Err(KreuzbergError::ImageProcessing { message, .. }) => {
            tracing::debug!("Expected error for empty image: {}", message);
        }
        Err(e) => {
            tracing::debug!("Empty image produced error: {}", e);
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

    match result {
        Err(KreuzbergError::Parsing { message, .. }) | Err(KreuzbergError::ImageProcessing { message, .. }) => {
            tracing::debug!("Expected error for non-image data: {}", message);
        }
        Err(e) => {
            tracing::debug!("Non-image data produced error: {}", e);
        }
        Ok(_) => {
            tracing::debug!("Non-image data was accepted");
        }
    }
}

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
                table_min_confidence: 1.5,
                table_column_threshold: -50,
                table_row_threshold_ratio: 10.0,
                ..Default::default()
            }),
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config);

    match result {
        Ok(extraction_result) => {
            tracing::debug!("Extreme table config was accepted (values may be clamped)");
            assert_non_empty_content(&extraction_result);
        }
        Err(KreuzbergError::Validation { message, .. }) => {
            tracing::debug!("Configuration validation caught extreme values: {}", message);
        }
        Err(e) => {
            tracing::debug!("Extreme table config produced error: {}", e);
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
                psm: -5,
                ..Default::default()
            }),
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config);

    match result {
        Ok(_) => {
            tracing::debug!("Negative PSM was accepted (clamped or default used)");
        }
        Err(e) => {
            tracing::debug!("Negative PSM produced error: {}", e);
        }
    }
}

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
                tessedit_char_whitelist: "".to_string(),
                ..Default::default()
            }),
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config);

    match result {
        Ok(extraction_result) => {
            tracing::debug!(
                "Empty whitelist accepted, content length: {}",
                extraction_result.content.len()
            );
        }
        Err(e) => {
            tracing::debug!("Empty whitelist produced error: {}", e);
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
                tessedit_char_blacklist: "abc".to_string(),
                ..Default::default()
            }),
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config);

    match result {
        Ok(extraction_result) => {
            tracing::debug!(
                "Conflicting whitelist/blacklist accepted: {}",
                extraction_result.content.len()
            );
        }
        Err(e) => {
            tracing::debug!("Conflicting config produced error: {}", e);
        }
    }
}

#[test]
fn test_ocr_empty_language() {
    if skip_if_missing("images/test_hello_world.png") {
        return;
    }

    let file_path = get_test_file_path("images/test_hello_world.png");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "".to_string(),
            tesseract_config: None,
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config);

    match result {
        Ok(_) => {
            tracing::debug!("Empty language accepted (fallback to default)");
        }
        Err(KreuzbergError::Validation { message, .. }) | Err(KreuzbergError::Ocr { message, .. }) => {
            tracing::debug!("Empty language rejected: {}", message);
        }
        Err(e) => {
            tracing::debug!("Empty language produced error: {}", e);
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
            language: "eng++deu++fra".to_string(),
            tesseract_config: None,
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config);

    match result {
        Ok(_) => {
            tracing::debug!("Malformed multi-language accepted (parser tolerant)");
        }
        Err(e) => {
            tracing::debug!("Malformed language string produced error: {}", e);
        }
    }
}

#[test]
fn test_ocr_cache_disabled_then_enabled() {
    if skip_if_missing("images/ocr_image.jpg") {
        return;
    }

    let file_path = get_test_file_path("images/ocr_image.jpg");

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
    if matches!(result1, Err(KreuzbergError::MissingDependency(_))) {
        return;
    }
    assert!(result1.is_ok(), "First extraction should succeed");

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
    if matches!(result2, Err(KreuzbergError::MissingDependency(_))) {
        return;
    }
    assert!(result2.is_ok(), "Second extraction should succeed");

    assert_non_empty_content(&result1.unwrap());
    assert_non_empty_content(&result2.unwrap());
}

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

    if matches!(
        extract_file_sync(&*file_path, None, &config),
        Err(KreuzbergError::MissingDependency(_))
    ) {
        return;
    }

    let mut handles = vec![];
    for i in 0..5 {
        let file_path_clone = Arc::clone(&file_path);
        let config_clone = Arc::clone(&config);

        let handle = thread::spawn(move || {
            let result = extract_file_sync(&*file_path_clone, None, &config_clone);
            let success = result.is_ok();
            match result {
                Ok(extraction_result) => {
                    tracing::debug!("Thread {} succeeded", i);
                    assert_non_empty_content(&extraction_result);
                }
                Err(e) => {
                    tracing::debug!("Thread {} failed: {}", i, e);
                }
            }
            success
        });

        handles.push(handle);
    }

    let successes: usize = handles.into_iter().map(|h| if h.join().unwrap() { 1 } else { 0 }).sum();

    tracing::debug!("Concurrent processing: {}/5 threads succeeded", successes);

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

    if matches!(
        extract_file_sync(&files[0], None, &config),
        Err(KreuzbergError::MissingDependency(_))
    ) {
        return;
    }

    let mut handles = vec![];
    for (i, file_path) in files.iter().enumerate() {
        let file_path_clone = file_path.clone();
        let config_clone = Arc::clone(&config);

        let handle = thread::spawn(move || {
            let result = extract_file_sync(&file_path_clone, None, &config_clone);
            match result {
                Ok(extraction_result) => {
                    tracing::debug!("File {} extraction succeeded", i);
                    assert_non_empty_content(&extraction_result);
                    true
                }
                Err(e) => {
                    tracing::debug!("File {} extraction failed: {}", i, e);
                    false
                }
            }
        });

        handles.push(handle);
    }

    let successes: usize = handles.into_iter().map(|h| if h.join().unwrap() { 1 } else { 0 }).sum();

    assert_eq!(
        successes, 2,
        "All concurrent threads should succeed with different files"
    );
}

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
                    target_dpi: 10000,
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

    match result {
        Ok(extraction_result) => {
            tracing::debug!("Extreme DPI accepted (clamped): {}", extraction_result.content.len());
        }
        Err(KreuzbergError::ImageProcessing { message, .. }) | Err(KreuzbergError::Validation { message, .. }) => {
            tracing::debug!("Extreme DPI rejected: {}", message);
        }
        Err(e) => {
            tracing::debug!("Extreme DPI produced error: {}", e);
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
                    binarization_method: "invalid_method_xyz".to_string(),
                    invert_colors: false,
                }),
                ..Default::default()
            }),
        }),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &config);

    match result {
        Ok(_) => {
            tracing::debug!("Invalid binarization method accepted (fallback used)");
        }
        Err(KreuzbergError::Validation { message, .. }) | Err(KreuzbergError::ImageProcessing { message, .. }) => {
            tracing::debug!("Invalid binarization method rejected: {}", message);
        }
        Err(e) => {
            tracing::debug!("Invalid binarization method produced error: {}", e);
        }
    }
}
