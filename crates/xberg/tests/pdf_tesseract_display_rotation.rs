//! Forced Tesseract OCR must receive a page in its display orientation (#2172).
#![cfg(all(feature = "pdf", feature = "ocr"))]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::ExtractionConfig;

const CONCORD: &[u8] =
    include_bytes!("../../../fixtures/repros/forced-ocr-orientation/fixtures/concord-airport-page29.pdf");

fn config(auto_rotate: Option<bool>) -> ExtractionConfig {
    let mut ocr = serde_json::json!({
        "backend": "tesseract", "language": "eng",
        "pipeline": {"stages": [{"backend": "tesseract", "backend_options": {"use_cache": false}}]}
    });
    if let Some(auto_rotate) = auto_rotate {
        ocr["auto_rotate"] = serde_json::json!(auto_rotate);
    }
    serde_json::from_value(serde_json::json!({
        "use_cache": false, "force_ocr": true, "pages": {"extract_pages": true},
        "concurrency": {"max_threads": 1}, "ocr": ocr
    }))
    .unwrap()
}

#[test]
fn display_rotated_page_reaches_tesseract_upright_whatever_auto_rotate_says() {
    for auto_rotate in [None, Some(false), Some(true)] {
        let result = extract_bytes_document_blocking(CONCORD, "application/pdf", &config(auto_rotate)).unwrap();
        assert!(
            result.content.to_uppercase().contains("BID TABULATION"),
            "auto_rotate={auto_rotate:?}: the page heading must be recognised"
        );
        let confidence = result.pages.as_ref().unwrap()[0]
            .ocr_confidence
            .as_ref()
            .and_then(|c| c.score)
            .expect("page OCR confidence");
        assert!(
            confidence > 0.6,
            "auto_rotate={auto_rotate:?}: sideways text scores ~0.35, got {confidence}"
        );
    }
}
