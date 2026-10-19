//! Native table suppression must not discard omitted source identifiers.
#![cfg(all(feature = "pdf", feature = "layout-detection"))]

mod helpers;

use std::collections::BTreeMap;
use xberg::core::config::{ExtractionConfig, OutputFormat, PageConfig, layout::LayoutDetectionConfig};

#[test]
#[ignore = "requires cached RT-DETR and TATR model files (ORT inference)"]
fn layout_preserves_all_caltrans_item_ids_without_changing_the_table() {
    let config = ExtractionConfig {
        use_cache: false,
        disable_ocr: true,
        output_format: OutputFormat::Markdown,
        include_document_structure: true,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        layout: Some(LayoutDetectionConfig::default()),
        use_layout_for_markdown: true,
        ..Default::default()
    };
    let result = helpers::extract_bytes_document_blocking(
        include_bytes!("../../../fixtures/repros/layout-item-column-loss/fixture.pdf"),
        "application/pdf",
        &config,
    )
    .expect("native layout extraction");
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/repros/layout-item-column-loss/expected-table.json"
    ))
    .unwrap();
    assert_eq!(
        result.tables.len(),
        1,
        "the model must actually produce the repro table"
    );
    assert_eq!(
        serde_json::to_value(&result.tables[0].cells).unwrap(),
        expected["cells"]
    );
    for id in 1..=25 {
        let pattern = regex::Regex::new(&format!(r"\b{id:04}\b")).unwrap();
        assert_eq!(
            pattern.find_iter(&result.content).count(),
            1,
            "item {id:04} must occur exactly once"
        );
        let page = &result.pages.as_ref().expect("page extraction enabled")[0];
        assert_eq!(
            pattern.find_iter(&page.content).count(),
            1,
            "page item {id:04} must occur exactly once"
        );
    }
    let monetary = regex::Regex::new(r"\b[\d,]+\.\d{2}\b").unwrap();
    let mut counts = BTreeMap::new();
    for token in monetary.find_iter(&result.content) {
        *counts.entry(token.as_str()).or_insert(0usize) += 1;
    }
    assert_eq!(serde_json::to_value(counts).unwrap(), expected["monetary_tokens"]);
    for represented in ["070030", "000001"] {
        let pattern = regex::Regex::new(&format!(r"\b{represented}\b")).unwrap();
        assert_eq!(
            pattern.find_iter(&result.content).count(),
            1,
            "genuine table text must not be duplicated"
        );
    }
}
