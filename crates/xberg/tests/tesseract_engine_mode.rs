#![cfg(all(feature = "ocr", feature = "bundle-tessdata-eng"))]

mod helpers;

use helpers::extract_bytes_document;
use xberg::core::config::{ExtractionConfig, OcrConfig};
use xberg::types::{ImagePreprocessingConfig, TesseractConfig};

const IMAGE: &[u8] = include_bytes!("../../../fixtures/images/test_hello_world.png");

fn bundled_tessdata() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("temporary model directory");
    std::fs::write(
        directory.path().join("eng.traineddata"),
        xberg_tesseract::bundled_eng_traineddata().expect("bundled English model"),
    )
    .expect("materialize bundled model");
    directory
}

fn configuration(directory: &std::path::Path, oem: i32) -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            backend: "tesseract".into(),
            language: vec!["eng".into()],
            tessdata_path: Some(directory.to_path_buf()),
            tesseract_config: Some(TesseractConfig {
                language: vec!["eng".into()],
                oem,
                psm: Some(7),
                use_cache: false,
                preprocessing: Some(ImagePreprocessingConfig {
                    binarization_method: "none".into(),
                    deskew: false,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

async fn assert_supported(directory: &std::path::Path, oem: i32) {
    let result = extract_bytes_document(IMAGE, "image/png", &configuration(directory, oem))
        .await
        .expect("supported engine must extract text");
    assert_eq!(result.content.split_whitespace().collect::<String>(), "HelloWorld");
}

async fn assert_unavailable(directory: &std::path::Path, oem: i32) {
    let error = extract_bytes_document(IMAGE, "image/png", &configuration(directory, oem))
        .await
        .expect_err("LSTM-only traineddata must reject an engine requiring legacy components");
    assert!(
        error.to_string().contains("initialize"),
        "initialization context: {error}"
    );
}

#[tokio::test]
async fn should_reject_legacy_only_mode_with_lstm_only_traineddata() {
    let directory = bundled_tessdata();
    assert_unavailable(directory.path(), 0).await;
}

#[tokio::test]
async fn should_reject_combined_mode_with_lstm_only_traineddata() {
    let directory = bundled_tessdata();
    assert_unavailable(directory.path(), 2).await;
}

#[tokio::test]
async fn should_recognize_with_explicit_lstm_and_default_engine_modes() {
    let directory = bundled_tessdata();
    for oem in [1, 3] {
        assert_supported(directory.path(), oem).await;
    }
}

#[tokio::test]
async fn should_keep_engine_modes_separate_when_reusing_pooled_handles() {
    let directory = bundled_tessdata();
    assert_supported(directory.path(), 1).await;
    assert_unavailable(directory.path(), 0).await;
    assert_supported(directory.path(), 1).await;
    assert_unavailable(directory.path(), 2).await;
    assert_supported(directory.path(), 3).await;
}
