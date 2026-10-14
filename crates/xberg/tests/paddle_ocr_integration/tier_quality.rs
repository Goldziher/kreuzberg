use super::*;

/// Compute Text F1 score: token-level precision/recall between predicted and reference text.
fn compute_tf1(predicted: &str, reference: &str) -> f64 {
    let pred_tokens: Vec<&str> = predicted.split_whitespace().collect();
    let ref_tokens: Vec<&str> = reference.split_whitespace().collect();

    if pred_tokens.is_empty() && ref_tokens.is_empty() {
        return 1.0;
    }
    if pred_tokens.is_empty() || ref_tokens.is_empty() {
        return 0.0;
    }

    let pred_set: std::collections::HashSet<&str> = pred_tokens.iter().copied().collect();
    let ref_set: std::collections::HashSet<&str> = ref_tokens.iter().copied().collect();

    let intersection = pred_set.intersection(&ref_set).count() as f64;
    let precision = intersection / pred_set.len() as f64;
    let recall = intersection / ref_set.len() as f64;

    if precision + recall == 0.0 {
        return 0.0;
    }
    2.0 * precision * recall / (precision + recall)
}

/// Ground truth for the complex_document test image.
const COMPLEX_DOC_GT: &str = "Sales Report 2024 This report contains quarterly sales data for our products. Q1 Sales: Product Units Revenue Widget A 150 ,500 Widget B 200 ,000 Widget C 100 ,000 Q2 Sales: Product Units Revenue Widget A 180 ,000 Widget B 220 ,200 Widget C 130 ,400 Summary: Total Q1 Revenue: ,500 Total Q2 Revenue: ,600 Prepared by: John Doe Date: 2024-03-15 Department: Finance";

/// Test mobile tier OCR on a document image, measuring TF1.
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_mobile_tier_ocr_quality() {
    let image_path = test_documents_dir().join("images/complex_document.png");
    assert!(image_path.exists(), "Test image not found: {:?}", image_path);

    let image_bytes = std::fs::read(&image_path).expect("Failed to read image");

    let config = PaddleOcrConfig::new("en")
        .with_cache_dir(test_cache_dir())
        .with_model_tier("mobile");

    let backend = PaddleOcrBackend::with_config(config).expect("Failed to create backend");

    let ocr_config = OcrConfig {
        backend: "paddle-ocr".to_string(),
        language: vec!["en".to_string()],
        paddle_ocr_settings: Some(PaddleOcrConfig::new("en").with_model_tier("mobile")),
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let result = backend.process_image(&image_bytes, &ocr_config).await;
    let elapsed_ms = start.elapsed().as_millis();

    assert!(result.is_ok(), "Mobile tier OCR failed: {:?}", result.err());

    let extraction = result.unwrap();
    let tf1 = compute_tf1(&extraction.content, COMPLEX_DOC_GT);

    println!("Mobile tier TF1: {:.1}% ({} ms)", tf1 * 100.0, elapsed_ms);
    println!(
        "Extracted text: {}",
        &extraction.content[..extraction.content.len().min(200)]
    );

    assert!(
        tf1 > 0.5,
        "Mobile tier TF1 too low: {:.1}% (expected >50%)",
        tf1 * 100.0
    );
}

/// Test server tier OCR on the same document for comparison.
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_server_tier_ocr_quality() {
    let image_path = test_documents_dir().join("images/complex_document.png");
    assert!(image_path.exists(), "Test image not found: {:?}", image_path);

    let image_bytes = std::fs::read(&image_path).expect("Failed to read image");

    let config = PaddleOcrConfig::new("en")
        .with_cache_dir(test_cache_dir())
        .with_model_tier("server");

    let backend = PaddleOcrBackend::with_config(config).expect("Failed to create backend");

    let ocr_config = OcrConfig {
        backend: "paddle-ocr".to_string(),
        language: vec!["en".to_string()],
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let result = backend.process_image(&image_bytes, &ocr_config).await;
    let elapsed_ms = start.elapsed().as_millis();

    assert!(result.is_ok(), "Server tier OCR failed: {:?}", result.err());

    let extraction = result.unwrap();
    let tf1 = compute_tf1(&extraction.content, COMPLEX_DOC_GT);

    println!("Server tier TF1: {:.1}% ({} ms)", tf1 * 100.0, elapsed_ms);

    assert!(
        tf1 > 0.6,
        "Server tier TF1 too low: {:.1}% (expected >60%)",
        tf1 * 100.0
    );
}

/// Test mobile tier with auto_rotate on rotated images.
/// Verifies that doc_ori detection + rotation correction produces consistent TF1.
#[tokio::test]
#[ignore = "requires ONNX Runtime and downloaded models"]
async fn test_mobile_tier_auto_rotate() {
    let base_dir = test_documents_dir().join("images");

    let test_cases = vec![
        ("complex_document.png", "original (0°)"),
        ("complex_document_rotated_90.png", "rotated 90°"),
        ("complex_document_rotated_180.png", "rotated 180°"),
        ("complex_document_rotated_270.png", "rotated 270°"),
    ];

    let config = PaddleOcrConfig::new("en")
        .with_cache_dir(test_cache_dir())
        .with_model_tier("mobile");

    let backend = PaddleOcrBackend::with_config(config).expect("Failed to create backend");

    let mut tf1_scores = Vec::new();

    for (filename, label) in &test_cases {
        let image_path = base_dir.join(filename);
        assert!(image_path.exists(), "Test image not found: {:?}", image_path);

        let image_bytes = std::fs::read(&image_path).expect("Failed to read image");

        let ocr_config = OcrConfig {
            backend: "paddle-ocr".to_string(),
            language: vec!["en".to_string()],
            auto_rotate: true,
            paddle_ocr_settings: Some(PaddleOcrConfig::new("en").with_model_tier("mobile")),
            ..Default::default()
        };

        let start = std::time::Instant::now();
        let result = backend.process_image(&image_bytes, &ocr_config).await;
        let elapsed_ms = start.elapsed().as_millis();

        assert!(result.is_ok(), "OCR failed on {}: {:?}", label, result.err());

        let extraction = result.unwrap();
        let tf1 = compute_tf1(&extraction.content, COMPLEX_DOC_GT);
        tf1_scores.push(tf1);

        println!("{}: TF1={:.1}% ({} ms)", label, tf1 * 100.0, elapsed_ms);
    }

    let min_tf1 = tf1_scores.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_tf1 = tf1_scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    println!(
        "TF1 range: {:.1}% - {:.1}% (spread: {:.1}%)",
        min_tf1 * 100.0,
        max_tf1 * 100.0,
        (max_tf1 - min_tf1) * 100.0
    );

    assert!(
        min_tf1 > 0.4,
        "Worst orientation TF1 too low: {:.1}% (expected >40% with auto_rotate)",
        min_tf1 * 100.0
    );

    assert!(
        max_tf1 - min_tf1 < 0.3,
        "TF1 spread too large: {:.1}% (expected <30% with auto_rotate)",
        (max_tf1 - min_tf1) * 100.0
    );
}

/// Test that mobile tier model download caches correctly.
///
/// Drives the download through the mobile-tier OCR path (`PaddleOcrBackend`
/// with `model_tier = "mobile"`) and verifies the resulting cache layout on
/// disk, since tier-specific model resolution (`ensure_v2_det_model`,
/// `resolve_rec_model`) is a crate-internal `ModelManager` detail.
#[tokio::test]
#[ignore = "requires network access"]
async fn test_mobile_tier_model_cache() {
    let cache_dir = test_cache_dir();
    let image_path = test_documents_dir().join("images/test_hello_world.png");
    assert!(image_path.exists(), "Test image not found: {:?}", image_path);
    let image_bytes = std::fs::read(&image_path).expect("Failed to read image");

    let config = PaddleOcrConfig::new("en")
        .with_cache_dir(cache_dir.clone())
        .with_model_tier("mobile");
    let backend = PaddleOcrBackend::with_config(config).expect("Failed to create backend");

    let ocr_config = OcrConfig {
        backend: "paddle-ocr".to_string(),
        language: vec!["en".to_string()],
        paddle_ocr_settings: Some(PaddleOcrConfig::new("en").with_model_tier("mobile")),
        ..Default::default()
    };

    let result: xberg::Result<ExtractedDocument> = backend.process_image(&image_bytes, &ocr_config).await;
    assert!(result.is_ok(), "Mobile tier OCR failed: {:?}", result.err());

    let det_file = cache_dir.join("v2").join("det").join("mobile").join("model.onnx");
    assert!(det_file.exists(), "Mobile det model not cached at {:?}", det_file);

    let det_size = std::fs::metadata(&det_file).unwrap().len();
    assert!(
        det_size < 10_000_000,
        "Mobile det model too large: {} bytes (expected <10MB)",
        det_size
    );
    println!(
        "Mobile det model size: {} bytes ({:.1} MB)",
        det_size,
        det_size as f64 / 1_048_576.0
    );
}

/// Test that server and mobile tiers produce different cached model files.
#[tokio::test]
#[ignore = "requires network access"]
async fn test_tier_model_differentiation() {
    let cache_dir = test_cache_dir();
    let image_path = test_documents_dir().join("images/test_hello_world.png");
    assert!(image_path.exists(), "Test image not found: {:?}", image_path);
    let image_bytes = std::fs::read(&image_path).expect("Failed to read image");

    for tier in ["server", "mobile"] {
        let config = PaddleOcrConfig::new("en")
            .with_cache_dir(cache_dir.clone())
            .with_model_tier(tier);
        let backend = PaddleOcrBackend::with_config(config).expect("Failed to create backend");

        let ocr_config = OcrConfig {
            backend: "paddle-ocr".to_string(),
            language: vec!["en".to_string()],
            paddle_ocr_settings: Some(PaddleOcrConfig::new("en").with_model_tier(tier)),
            ..Default::default()
        };

        let result: xberg::Result<ExtractedDocument> = backend.process_image(&image_bytes, &ocr_config).await;
        assert!(result.is_ok(), "{tier} tier OCR failed: {:?}", result.err());
    }

    let server_det = cache_dir.join("v2").join("det").join("server").join("model.onnx");
    let mobile_det = cache_dir.join("v2").join("det").join("mobile").join("model.onnx");
    assert!(server_det.exists(), "Server det model not cached");
    assert!(mobile_det.exists(), "Mobile det model not cached");
    assert_ne!(server_det, mobile_det, "Server and mobile det paths should differ");

    println!("Server det: {:?}", server_det);
    println!("Mobile det: {:?}", mobile_det);
}
