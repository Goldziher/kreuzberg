//! Wrapping in several cells of one drawn band must not split the record (#2169).
#![cfg(feature = "pdf")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::ExtractionConfig;

const PAGE: &[u8] = include_bytes!("../../../fixtures/repros/wrapped-subcontractor-rows/sfport-p7.pdf");

fn rows() -> Vec<Vec<String>> {
    let config: ExtractionConfig = serde_json::from_value(serde_json::json!({
        "use_cache": false, "disable_ocr": true, "enable_quality_processing": false,
        "output_format": "plain", "include_document_structure": true,
        "pages": {"extract_pages": true, "insert_page_markers": false},
        "pdf_options": {"extract_tables": true},
        "concurrency": {"max_threads": 1}
    }))
    .unwrap();
    let result = extract_bytes_document_blocking(PAGE, "application/pdf", &config).unwrap();
    assert_eq!(result.tables.len(), 1, "the disclosure list is one table");
    result.tables[0].cells.clone()
}

#[test]
fn wrapped_subcontractor_record_stays_one_row() {
    let rows = rows();

    let bayview: Vec<_> = rows
        .iter()
        .filter(|row| row.iter().any(|c| c.contains("BAYVIEW")))
        .collect();
    assert_eq!(bayview.len(), 1);
    let record = bayview[0].join(" | ");
    for part in [
        "1235 THOMAS AVE.",
        "SAN FRANCISCO, CA 94124",
        "MICRO",
        "FABRICATIONS",
        "$17,500",
    ] {
        assert!(record.contains(part), "{part:?} must stay with its record: {record}");
    }
    assert_eq!(
        rows.iter().filter(|row| row.iter().any(|c| c.contains("$"))).count(),
        10
    );
    assert_eq!(rows.len(), 11, "one header row plus ten records");
}
