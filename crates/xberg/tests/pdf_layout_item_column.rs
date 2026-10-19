//! Layout table suppression must not delete item numbers the table does not hold (#2171).
#![cfg(all(feature = "pdf", feature = "layout-detection"))]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::ExtractionConfig;

const PAGE: &[u8] = include_bytes!("../../../fixtures/repros/layout-item-column-loss/fixture.pdf");

#[test]
fn layout_table_suppression_keeps_item_numbers_the_table_does_not_hold() {
    let config: ExtractionConfig = serde_json::from_value(serde_json::json!({
        "use_cache": false, "disable_ocr": true, "enable_quality_processing": false,
        "output_format": "markdown", "use_layout_for_markdown": true,
        "pages": {"extract_pages": true, "insert_page_markers": false},
        "layout": {"table_model": "tatr", "table_overlap_preference": "layout"},
        "concurrency": {"max_threads": 1}
    }))
    .unwrap();
    let result = extract_bytes_document_blocking(PAGE, "application/pdf", &config).unwrap();
    let missing: Vec<String> = (1..=25)
        .map(|id| format!("{id:04}"))
        .filter(|id| !result.content.contains(id.as_str()))
        .collect();
    assert!(missing.is_empty(), "item numbers deleted from the output: {missing:?}");
}
