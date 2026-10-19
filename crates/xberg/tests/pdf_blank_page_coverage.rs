//! Text coverage counts a known empty page whether or not page content is requested (#2170).
#![cfg(all(feature = "pdf", feature = "heuristics"))]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::ExtractionConfig;

const EMPTY_MIDDLE: &[u8] = include_bytes!("../../../fixtures/repros/blank-page-coverage/empty-middle.pdf");

fn coverage(pages: Option<bool>) -> (f32, usize) {
    let mut config = serde_json::json!({
        "use_cache": false, "disable_ocr": true, "include_document_structure": false, "output_format": "plain"
    });
    if let Some(extract_pages) = pages {
        config["pages"] = serde_json::json!({"extract_pages": extract_pages, "insert_page_markers": false});
    }
    let config: ExtractionConfig = serde_json::from_value(config).unwrap();
    let result = extract_bytes_document_blocking(EMPTY_MIDDLE, "application/pdf", &config).unwrap();
    (result.extraction_confidence.unwrap().text_coverage, result.counts.pages)
}

#[test]
fn empty_middle_page_lowers_coverage_in_every_page_output_mode() {
    for mode in [None, Some(false), Some(true)] {
        let (text_coverage, pages) = coverage(mode);
        assert_eq!(pages, 3, "mode {mode:?}");
        assert!(
            (text_coverage - 2.0 / 3.0).abs() < 1e-6,
            "mode {mode:?}: two of three pages have text, got {text_coverage}"
        );
    }
}
