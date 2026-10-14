//! Integration tests for PaddleOCR functionality.
//!
//! These tests require:
//! - Network access to download models from HuggingFace
//! - ONNX Runtime installed on the system
//!
//! Run with: `cargo test -p xberg --features paddle-ocr --test paddle_ocr_integration -- --ignored`
//!
//! Model-manager-level tests use only the public `ModelManager` surface
//! (`new`, `ensure_all_models`, `manifest`) plus documented on-disk cache
//! layout; the previous version of this suite relied on `pub(crate)` helpers
//! (`ensure_models_exist`, `ensure_v2_det_model`, `resolve_rec_model`,
//! `cache_stats`, `are_models_cached`) that the alef-migration cleanup
//! deliberately narrowed to crate-internal visibility, which the rewritten
//! suite below no longer needs.

#![allow(deprecated, clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test/bench binaries print by design; org logging policy exempts tests
#![cfg(paddle_ocr)]

mod helpers;

use std::path::PathBuf;

use helpers::extract_uri_document;
use xberg::core::config::{ExtractionConfig, OcrConfig};
use xberg::paddle_ocr::{ModelManager, PaddleOcrBackend, PaddleOcrConfig};
use xberg::plugins::OcrBackend;
use xberg::types::ExtractedDocument;

#[path = "paddle_ocr_integration/tier_quality.rs"]
mod tier_quality;

/// Helper to get the test documents directory
fn test_documents_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test_documents")
}

/// Helper to get a temporary cache directory for tests
fn test_cache_dir() -> PathBuf {
    std::env::temp_dir().join("xberg_paddle_test")
}

/// Test that model manager can download models from HuggingFace.
///
/// This test downloads actual models and verifies they are cached correctly,
/// using only the public `ModelManager` surface (`new`, `ensure_all_models`)
/// plus the documented on-disk cache layout (`v2/det/<tier>`, `v2/cls`, etc.).
/// It's ignored by default since it requires network access and a large
/// (all-tier) download.
#[tokio::test]
#[ignore = "requires network access and a large (all-tier) download"]
async fn test_model_download_from_huggingface() {
    let cache_dir = test_cache_dir();

    let _ = std::fs::remove_dir_all(&cache_dir);

    let manager = ModelManager::new(cache_dir.clone());

    let result = manager.ensure_all_models();
    assert!(result.is_ok(), "Model download failed: {:?}", result.err());

    let det_dir = cache_dir.join("v2").join("det").join("server");
    let cls_dir = cache_dir.join("v2").join("cls");
    let ori_dir = cache_dir.join("v2").join("doc_ori");

    assert!(det_dir.join("model.onnx").exists(), "Detection ONNX file not found");
    assert!(
        cls_dir.join("model.onnx").exists(),
        "Classification ONNX file not found"
    );
    assert!(
        ori_dir.join("model.onnx").exists(),
        "Document orientation ONNX file not found"
    );

    let manifest = ModelManager::manifest();
    assert!(
        manifest.len() >= 3,
        "Expected at least 3 manifest entries, got {}",
        manifest.len()
    );

    println!("Manifest entries: {}", manifest.len());
    println!("Detection model: {:?}", det_dir);
    println!("Classification model: {:?}", cls_dir);
    println!("Document orientation model: {:?}", ori_dir);
}

/// Test OCR on a simple English "Hello World" image.
///
/// This test requires ONNX Runtime and downloaded models.
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_ocr_hello_world_english() {
    let image_path = test_documents_dir().join("images/test_hello_world.png");
    assert!(image_path.exists(), "Test image not found: {:?}", image_path);

    let image_bytes = std::fs::read(&image_path).expect("Failed to read image");

    let config = PaddleOcrConfig::new("en").with_cache_dir(test_cache_dir());

    let backend = PaddleOcrBackend::with_config(config).expect("Failed to create backend");

    let ocr_config = OcrConfig {
        backend: "paddle-ocr".to_string(),
        language: vec!["en".to_string()],
        ..Default::default()
    };

    let result: xberg::Result<ExtractedDocument> = backend.process_image(&image_bytes, &ocr_config).await;
    assert!(result.is_ok(), "OCR failed: {:?}", result.err());

    let extraction: ExtractedDocument = result.unwrap();
    let text = extraction.content.to_lowercase();

    println!("OCR result: {}", extraction.content);

    assert!(
        text.contains("hello") || text.contains("helo"),
        "Expected 'hello' in OCR result: {}",
        text
    );
    assert!(
        text.contains("world") || text.contains("worid"),
        "Expected 'world' in OCR result: {}",
        text
    );
}

/// Requested languages the selected recognition model cannot cover must surface
/// as a `ProcessingWarning` instead of silently dropping their text (#1346),
/// and OCR metadata must report the model actually used rather than every
/// requested language.
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_ocr_uncovered_language_emits_warning() {
    let image_path = test_documents_dir().join("images/test_hello_world.png");
    assert!(image_path.exists(), "Test image not found: {:?}", image_path);

    let image_bytes = std::fs::read(&image_path).expect("Failed to read image");

    let config = PaddleOcrConfig::new("en").with_cache_dir(test_cache_dir());
    let backend = PaddleOcrBackend::with_config(config).expect("Failed to create backend");

    let ocr_config = OcrConfig {
        backend: "paddle-ocr".to_string(),
        language: vec!["en".to_string(), "ru".to_string()],
        ..Default::default()
    };

    let extraction: ExtractedDocument = backend
        .process_image(&image_bytes, &ocr_config)
        .await
        .expect("OCR failed");

    assert_eq!(
        extraction.processing_warnings.len(),
        1,
        "expected one uncovered-language warning, got: {:?}",
        extraction.processing_warnings
    );
    let warning = &extraction.processing_warnings[0];
    assert_eq!(warning.source, "paddle-ocr");
    assert!(
        warning.message.contains("[ru]") && warning.message.contains("'en'"),
        "warning should name the uncovered language and the selected model: {}",
        warning.message
    );

    let language = match &extraction.metadata.format {
        Some(xberg::types::FormatMetadata::Ocr(ocr)) => ocr.language.clone(),
        other => panic!("expected OCR metadata, got: {:?}", other),
    };
    assert_eq!(language, "en", "metadata must report the model actually used");
}

/// Test PP-OCRv6 recognition on English across all three v6 tiers.
///
/// v6 routes English (a v6-unified family) to the unified recognition model at the
/// configured tier (medium/small/tiny), with a v6 detector and the shared PP-LCNet
/// classifier. Verifies the version-aware wiring end to end against the live models.
#[tokio::test]
#[ignore = "requires network access and ONNX Runtime (PP-OCRv6 model download)"]
async fn test_ocr_pp_ocrv6_english_tiers() {
    let image_path = test_documents_dir().join("images/test_hello_world.png");
    assert!(image_path.exists(), "Test image not found: {:?}", image_path);
    let image_bytes = std::fs::read(&image_path).expect("Failed to read image");

    for tier in ["medium", "small", "tiny"] {
        let config = PaddleOcrConfig::new("en")
            .with_model_version("pp-ocrv6")
            .with_model_tier(tier)
            .with_cache_dir(test_cache_dir());

        let backend = PaddleOcrBackend::with_config(config).expect("Failed to create backend");

        let ocr_config = OcrConfig {
            backend: "paddle-ocr".to_string(),
            language: vec!["en".to_string()],
            ..Default::default()
        };

        let result: xberg::Result<ExtractedDocument> = backend.process_image(&image_bytes, &ocr_config).await;
        assert!(result.is_ok(), "PP-OCRv6 {tier} OCR failed: {:?}", result.err());

        let text = result.unwrap().content.to_lowercase();
        println!("PP-OCRv6 {tier} OCR result: {text}");
        assert!(
            text.contains("hello") || text.contains("helo"),
            "Expected 'hello' in PP-OCRv6 {tier} result: {text}"
        );
        assert!(
            text.contains("world") || text.contains("worid"),
            "Expected 'world' in PP-OCRv6 {tier} result: {text}"
        );
    }
}

/// Test OCR on a complex English document (newspaper).
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_ocr_newspaper_english() {
    let image_path = test_documents_dir().join("images/ocr_image.jpg");
    assert!(image_path.exists(), "Test image not found: {:?}", image_path);

    let image_bytes = std::fs::read(&image_path).expect("Failed to read image");

    let config = PaddleOcrConfig::new("en").with_cache_dir(test_cache_dir());

    let backend = PaddleOcrBackend::with_config(config).expect("Failed to create backend");

    let ocr_config = OcrConfig {
        backend: "paddle-ocr".to_string(),
        language: vec!["en".to_string()],
        ..Default::default()
    };

    let result: xberg::Result<ExtractedDocument> = backend.process_image(&image_bytes, &ocr_config).await;
    assert!(result.is_ok(), "OCR failed: {:?}", result.err());

    let extraction: ExtractedDocument = result.unwrap();
    let text = extraction.content.to_uppercase();

    println!(
        "OCR result (first 500 chars): {}",
        &extraction.content[..extraction.content.len().min(500)]
    );

    assert!(
        text.contains("NASDAQ") || text.contains("NASOAQ"),
        "Expected 'NASDAQ' in OCR result"
    );
    assert!(
        text.contains("AMEX") || text.contains("STOCK"),
        "Expected 'AMEX' or 'STOCK' in OCR result"
    );
}

/// Test OCR on Chinese text image.
///
/// Note: Uses per-family PP-OCRv5 recognition models.
/// This test verifies the pipeline handles non-English images without crashing,
/// but requires the Chinese recognition model to be cached for accurate results.
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_ocr_chinese_text() {
    let image_path = test_documents_dir().join("images/chi_sim_image.jpeg");
    assert!(image_path.exists(), "Test image not found: {:?}", image_path);

    let image_bytes = std::fs::read(&image_path).expect("Failed to read image");

    let config = PaddleOcrConfig::new("ch").with_cache_dir(test_cache_dir());

    let backend = PaddleOcrBackend::with_config(config).expect("Failed to create backend");

    let ocr_config = OcrConfig {
        backend: "paddle-ocr".to_string(),
        language: vec!["ch".to_string()],
        ..Default::default()
    };

    let result: xberg::Result<ExtractedDocument> = backend.process_image(&image_bytes, &ocr_config).await;
    assert!(result.is_ok(), "OCR failed: {:?}", result.err());

    let extraction: ExtractedDocument = result.unwrap();

    println!("OCR result: {}", extraction.content);

    assert!(
        !extraction.content.is_empty(),
        "Expected non-empty OCR result for Chinese image"
    );
}

/// Test that the backend correctly reports supported languages.
#[test]
fn test_supported_languages() {
    let backend = PaddleOcrBackend::new().expect("Failed to create backend");

    assert!(backend.supports_language("ch"));
    assert!(backend.supports_language("en"));
    assert!(backend.supports_language("japan"));
    assert!(backend.supports_language("korean"));

    assert!(backend.supports_language("chi_sim"));
    assert!(backend.supports_language("eng"));
    assert!(backend.supports_language("jpn"));
    assert!(backend.supports_language("fra"));
    assert!(backend.supports_language("deu"));

    assert!(!backend.supports_language("xyz"));
    assert!(!backend.supports_language("klingon"));
}

/// Test that empty image returns an error.
#[tokio::test]
async fn test_empty_image_error() {
    let backend = PaddleOcrBackend::new().expect("Failed to create backend");

    let ocr_config = OcrConfig {
        backend: "paddle-ocr".to_string(),
        language: vec!["en".to_string()],
        ..Default::default()
    };

    let result: xberg::Result<ExtractedDocument> = backend.process_image(&[], &ocr_config).await;
    assert!(result.is_err(), "Expected error for empty image");
}

/// Test that invalid image data returns an error (requires ONNX Runtime).
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_invalid_image_error() {
    let config = PaddleOcrConfig::new("en").with_cache_dir(test_cache_dir());
    let backend = PaddleOcrBackend::with_config(config).expect("Failed to create backend");

    let ocr_config = OcrConfig {
        backend: "paddle-ocr".to_string(),
        language: vec!["en".to_string()],
        ..Default::default()
    };

    let invalid_bytes = vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];

    let result: xberg::Result<ExtractedDocument> = backend.process_image(&invalid_bytes, &ocr_config).await;
    assert!(result.is_err(), "Expected error for invalid image data");
}

/// Test processing an image file directly.
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_process_image_file() {
    let image_path = test_documents_dir().join("images/test_hello_world.png");
    assert!(image_path.exists(), "Test image not found: {:?}", image_path);

    let config = PaddleOcrConfig::new("en").with_cache_dir(test_cache_dir());
    let backend = PaddleOcrBackend::with_config(config).expect("Failed to create backend");

    let ocr_config = OcrConfig {
        backend: "paddle-ocr".to_string(),
        language: vec!["en".to_string()],
        ..Default::default()
    };

    let result: xberg::Result<ExtractedDocument> = backend.process_image_file(&image_path, &ocr_config).await;
    assert!(result.is_ok(), "OCR failed: {:?}", result.err());

    let extraction: ExtractedDocument = result.unwrap();
    let text = extraction.content.to_lowercase();

    assert!(
        text.contains("hello") || text.contains("helo"),
        "Expected 'hello' in OCR result"
    );
}

/// Test that explicit cache_dir in config overrides default.
#[test]
fn test_cache_dir_explicit_config() {
    let config = PaddleOcrConfig::new("en").with_cache_dir(PathBuf::from("/explicit/path"));
    let resolved = config.resolve_cache_dir();

    assert_eq!(resolved, PathBuf::from("/explicit/path"));
}

/// Test that OCR elements have proper geometry (quadrilateral bounding boxes).
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_paddle_ocr_elements_geometry() {
    use xberg::types::OcrBoundingGeometry;

    let image_path = test_documents_dir().join("images/test_hello_world.png");
    assert!(image_path.exists(), "Test image not found: {:?}", image_path);

    let image_bytes = std::fs::read(&image_path).expect("Failed to read image");

    let config = PaddleOcrConfig::new("en").with_cache_dir(test_cache_dir());
    let backend = PaddleOcrBackend::with_config(config).expect("Failed to create backend");

    let ocr_config = OcrConfig {
        backend: "paddle-ocr".to_string(),
        language: vec!["en".to_string()],
        ..Default::default()
    };

    let result: xberg::Result<ExtractedDocument> = backend.process_image(&image_bytes, &ocr_config).await;
    assert!(result.is_ok(), "OCR failed: {:?}", result.err());

    let extraction: ExtractedDocument = result.unwrap();

    assert!(
        extraction.ocr_elements.is_some(),
        "Expected ocr_elements to be populated"
    );

    let elements = extraction.ocr_elements.as_ref().unwrap();
    assert!(!elements.is_empty(), "Expected at least one OCR element");

    for element in elements {
        match &element.geometry {
            OcrBoundingGeometry::Quadrilateral { points } => {
                assert_eq!(points.len(), 4, "Quadrilateral should have 4 points");
                println!("Quadrilateral with 4 points");
            }
            OcrBoundingGeometry::Rectangle {
                left,
                top,
                width,
                height,
            } => {
                assert!(*width > 0, "Width should be positive");
                assert!(*height > 0, "Height should be positive");
                println!("Rectangle at ({}, {}) size {}x{}", left, top, width, height);
            }
        }
    }

    println!("Found {} OCR elements with valid geometry", elements.len());
}

/// Test that OCR elements have confidence scores.
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_paddle_ocr_elements_confidence() {
    let image_path = test_documents_dir().join("images/test_hello_world.png");
    assert!(image_path.exists(), "Test image not found: {:?}", image_path);

    let image_bytes = std::fs::read(&image_path).expect("Failed to read image");

    let config = PaddleOcrConfig::new("en").with_cache_dir(test_cache_dir());
    let backend = PaddleOcrBackend::with_config(config).expect("Failed to create backend");

    let ocr_config = OcrConfig {
        backend: "paddle-ocr".to_string(),
        language: vec!["en".to_string()],
        ..Default::default()
    };

    let result: xberg::Result<ExtractedDocument> = backend.process_image(&image_bytes, &ocr_config).await;
    assert!(result.is_ok(), "OCR failed: {:?}", result.err());

    let extraction: ExtractedDocument = result.unwrap();

    assert!(
        extraction.ocr_elements.is_some(),
        "Expected ocr_elements to be populated"
    );

    let elements = extraction.ocr_elements.as_ref().unwrap();
    assert!(!elements.is_empty(), "Expected at least one OCR element");

    for element in elements {
        assert!(
            element.confidence.recognition >= 0.0 && element.confidence.recognition <= 1.0,
            "Recognition confidence should be between 0 and 1, got {}",
            element.confidence.recognition
        );

        if let Some(det_conf) = element.confidence.detection {
            assert!(
                (0.0..=1.0).contains(&det_conf),
                "Detection confidence should be between 0 and 1, got {}",
                det_conf
            );
        }

        println!(
            "Element '{}' has recognition confidence: {:.2}%",
            element.text,
            element.confidence.recognition * 100.0
        );
    }
}

/// Test rotation detection via angle classification.
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_paddle_ocr_rotation_detection() {
    let image_path = test_documents_dir().join("images/ocr_image.jpg");
    assert!(image_path.exists(), "Test image not found: {:?}", image_path);

    let image_bytes = std::fs::read(&image_path).expect("Failed to read image");

    let config = PaddleOcrConfig::new("en").with_cache_dir(test_cache_dir());

    let backend = PaddleOcrBackend::with_config(config).expect("Failed to create backend");

    let ocr_config = OcrConfig {
        backend: "paddle-ocr".to_string(),
        language: vec!["en".to_string()],
        ..Default::default()
    };

    let result: xberg::Result<ExtractedDocument> = backend.process_image(&image_bytes, &ocr_config).await;
    assert!(result.is_ok(), "OCR failed: {:?}", result.err());

    let extraction: ExtractedDocument = result.unwrap();

    assert!(
        extraction.ocr_elements.is_some(),
        "Expected ocr_elements to be populated"
    );

    let elements = extraction.ocr_elements.as_ref().unwrap();

    let elements_with_rotation = elements.iter().filter(|e| e.rotation.is_some()).count();

    println!(
        "Found {} elements total, {} with rotation info",
        elements.len(),
        elements_with_rotation
    );

    for element in elements.iter().filter(|e| e.rotation.is_some()) {
        let rotation = element.rotation.as_ref().unwrap();
        assert!(
            rotation.angle_degrees >= 0.0 && rotation.angle_degrees < 360.0,
            "Rotation angle should be between 0 and 360, got {}",
            rotation.angle_degrees
        );
    }
}

/// Test table reconstruction from OCR elements.
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_paddle_ocr_table_reconstruction() {
    let image_path = test_documents_dir().join("images/simple_table.png");
    assert!(image_path.exists(), "Test image not found: {:?}", image_path);

    let image_bytes = std::fs::read(&image_path).expect("Failed to read image");

    let config = PaddleOcrConfig::new("en")
        .with_cache_dir(test_cache_dir())
        .with_table_detection(true);

    let backend = PaddleOcrBackend::with_config(config).expect("Failed to create backend");

    let ocr_config = OcrConfig {
        backend: "paddle-ocr".to_string(),
        language: vec!["en".to_string()],
        ..Default::default()
    };

    let result: xberg::Result<ExtractedDocument> = backend.process_image(&image_bytes, &ocr_config).await;
    assert!(result.is_ok(), "OCR failed: {:?}", result.err());

    let extraction: ExtractedDocument = result.unwrap();

    println!(
        "OCR result (first 500 chars): {}",
        &extraction.content[..extraction.content.len().min(500)]
    );

    if !extraction.tables.is_empty() {
        println!("Found {} tables", extraction.tables.len());
        for (i, table) in extraction.tables.iter().enumerate() {
            println!(
                "Table {}: {} rows x {} cols",
                i,
                table.cells.len(),
                table.cells.first().map(|r| r.len()).unwrap_or(0)
            );
        }
    }

    if let Some(elements) = &extraction.ocr_elements {
        println!("Found {} OCR elements", elements.len());

        let non_empty_elements = elements.iter().filter(|e| !e.text.is_empty()).count();
        assert!(non_empty_elements > 0, "Expected at least one element with text");
    }
}

/// A detected table must not also appear line by line in the content of a standalone image (#1571).
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_paddle_ocr_table_text_not_duplicated_in_content() {
    let image_path = test_documents_dir().join("images/simple_table.png");
    assert!(image_path.exists(), "Test image not found: {:?}", image_path);

    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "paddle-ocr".to_string(),
            language: vec!["en".to_string()],
            paddle_ocr_settings: Some(PaddleOcrConfig::new("en").with_table_detection(true)),
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };

    let result = extract_uri_document(&image_path, None, &config)
        .await
        .expect("OCR extraction must succeed");

    assert_eq!(result.tables.len(), 1, "table detection must fire on this fixture");
    assert_eq!(
        result.content.matches("Banana").count(),
        1,
        "the table row must appear once, not also as a standalone line: {}",
        result.content
    );
}

/// A scanned PDF page that is only a table must show the table text once, as the table, on the
/// force-OCR route (#1571): the page text must not refill the page the table emptied.
#[cfg(feature = "pdf")]
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_paddle_ocr_full_page_table_not_duplicated_in_pdf_content() {
    use xberg::core::config::OutputFormat;

    let pdf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ocr/shaded_table_scan.pdf");

    for output_format in [OutputFormat::Plain, OutputFormat::Markdown] {
        let is_markdown = output_format == OutputFormat::Markdown;
        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "paddle-ocr".to_string(),
                language: vec!["en".to_string()],
                paddle_ocr_settings: Some(PaddleOcrConfig::new("en").with_table_detection(true)),
                ..Default::default()
            }),
            force_ocr: true,
            output_format,
            use_cache: false,
            ..Default::default()
        };

        let result = extract_uri_document(&pdf_path, None, &config)
            .await
            .expect("OCR extraction must succeed");

        assert_eq!(result.tables.len(), 1, "table detection must fire on this fixture");
        assert_eq!(
            result.content.matches("APPLES").count(),
            1,
            "the table must appear once, not also as page text: {}",
            result.content
        );
        if is_markdown {
            assert!(
                result.tables[0].table_id.is_some(),
                "the table must stay part of the structured document"
            );
            assert!(
                result.content.starts_with("| Six-Year Summary"),
                "the page must render as the table: {}",
                result.content
            );
        }
    }
}

/// A blank page whose text is recovered from its embedded images keeps that text when the retry
/// also finds a table, and prints the table once (#2014). The fixture draws corpus objects
/// `5883a36a12` and `0c1a8b71c1` under a zero-area clip, so the page renders blank and only the
/// embedded-image retry reads it. `Banana` appears only in the table. ~keep
#[cfg(feature = "pdf")]
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_paddle_ocr_keeps_embedded_image_retry_text_beside_its_table() {
    use xberg::core::config::OutputFormat;

    let pdf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ocr/clipped_table_and_receipt.pdf");

    for output_format in [OutputFormat::Plain, OutputFormat::Markdown] {
        let is_markdown = output_format == OutputFormat::Markdown;
        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "paddle-ocr".to_string(),
                language: vec!["en".to_string()],
                paddle_ocr_config: Some(serde_json::json!({"enable_table_detection": true})),
                ..Default::default()
            }),
            force_ocr: true,
            output_format,
            use_cache: false,
            ..Default::default()
        };

        let result = extract_uri_document(&pdf_path, None, &config)
            .await
            .expect("OCR extraction must succeed");

        assert_eq!(result.tables.len(), 1, "the retry must find exactly one table");
        let table_render = if is_markdown {
            result.tables[0].markdown.clone()
        } else {
            result.tables[0]
                .cells
                .iter()
                .map(|row| row.join(" "))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let (prose, rendered_table) = result
            .content
            .rsplit_once("\n\n")
            .expect("the recovered prose and table must be separate document blocks");
        assert_eq!(rendered_table, table_render);
        assert_eq!(
            result.content.matches("Banana").count(),
            1,
            "the retry's table must appear in the content exactly once: {}",
            result.content
        );
        assert_eq!(
            prose.matches("CIMB NIAGA").count(),
            1,
            "the recovered prose must appear once"
        );
    }
}

/// The mixed route must preserve prose recovered by the embedded-image retry when the same
/// retry also contributes a table (#2021). ~keep
#[cfg(feature = "pdf")]
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_paddle_ocr_mixed_route_keeps_embedded_image_retry_text_beside_its_table() {
    use xberg::core::config::OutputFormat;

    let pdf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ocr/clipped_table_and_receipt.pdf");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "paddle-ocr".to_string(),
            language: vec!["en".to_string()],
            paddle_ocr_config: Some(serde_json::json!({"enable_table_detection": true})),
            ..Default::default()
        }),
        force_ocr_pages: Some(vec![1]),
        output_format: OutputFormat::Markdown,
        use_cache: false,
        ..Default::default()
    };

    let result = extract_uri_document(&pdf_path, None, &config)
        .await
        .expect("OCR extraction must succeed");

    assert_eq!(result.tables.len(), 1, "the retry must find exactly one table");
    assert_eq!(
        result.content.matches("CIMB NIAGA").count(),
        1,
        "the recovered prose must remain beside the table: {}",
        result.content
    );
    assert_eq!(
        result.content.matches("Banana").count(),
        1,
        "the retry's table must appear exactly once: {}",
        result.content
    );
    let pages = result.pages.as_ref().expect("mixed PDF extraction must expose pages");
    assert_eq!(pages.len(), 1);
    assert_eq!(
        pages[0].content.matches("CIMB NIAGA").count(),
        1,
        "the recovered prose must remain in pages[].content: {}",
        pages[0].content
    );
}

/// A blank page whose only embedded image is a table keeps that table, printed once, when the
/// table holds every line the retry recovered (#2014). The fixture draws corpus object
/// `5883a36a12` under a zero-area clip, so only the embedded-image retry reads it. ~keep
#[cfg(feature = "pdf")]
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_paddle_ocr_keeps_an_embedded_image_retry_table_that_holds_all_of_its_text() {
    use xberg::core::config::OutputFormat;

    let pdf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ocr/clipped_table.pdf");

    for output_format in [OutputFormat::Plain, OutputFormat::Markdown] {
        let is_markdown = output_format == OutputFormat::Markdown;
        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "paddle-ocr".to_string(),
                language: vec!["en".to_string()],
                paddle_ocr_config: Some(serde_json::json!({"enable_table_detection": true})),
                ..Default::default()
            }),
            force_ocr: true,
            output_format,
            use_cache: false,
            ..Default::default()
        };

        let result = extract_uri_document(&pdf_path, None, &config)
            .await
            .expect("OCR extraction must succeed");

        assert_eq!(
            result.tables.len(),
            1,
            "the retry's table must be kept: {}",
            result.content
        );
        assert_eq!(
            result.content.matches("Banana").count(),
            1,
            "the retry's table must appear in the content exactly once: {}",
            result.content
        );
        let expected = if is_markdown {
            result.tables[0].markdown.clone()
        } else {
            result.tables[0]
                .cells
                .iter()
                .map(|row| row.join(" "))
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert_eq!(result.content, expected);
    }
}

/// Default cache resolution follows the Hugging Face conventions since the
/// model cache moved to hf-hub: `HF_HUB_CACHE` wins when set, and an explicit
/// `cache_dir` on the config overrides the environment entirely.
#[test]
#[allow(unsafe_code)]
fn test_cache_dir_default() {
    let original = std::env::var("HF_HUB_CACHE").ok();
    let hub_cache = std::env::temp_dir().join("xberg-test-hf-hub-cache");

    unsafe {
        std::env::set_var("HF_HUB_CACHE", &hub_cache);
    }

    let resolved = PaddleOcrConfig::new("en").resolve_cache_dir();
    let explicit = PaddleOcrConfig::new("en")
        .with_cache_dir(PathBuf::from("/explicit/override"))
        .resolve_cache_dir();

    unsafe {
        match original {
            Some(val) => std::env::set_var("HF_HUB_CACHE", val),
            None => std::env::remove_var("HF_HUB_CACHE"),
        }
    }

    assert_eq!(resolved, hub_cache, "default resolution must honor HF_HUB_CACHE");
    assert_eq!(
        explicit,
        PathBuf::from("/explicit/override"),
        "an explicit cache_dir must win over the environment"
    );
}
