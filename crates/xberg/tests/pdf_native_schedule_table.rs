//! A complete item schedule with ~2 line-heights between rows must stay one native table (#2168).
#![cfg(feature = "pdf")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::{ExtractedDocument, ExtractionConfig};

const SCHEDULE_PAGE: &[u8] = include_bytes!("../../../fixtures/repros/caltrans-native-table/fixtures/caltrans-p3.pdf");
const CONTROL_PAGE: &[u8] = include_bytes!("../../../fixtures/repros/caltrans-native-table/fixtures/caltrans-p8.pdf");

fn extract(bytes: &[u8]) -> ExtractedDocument {
    let config: ExtractionConfig = serde_json::from_value(serde_json::json!({
        "use_cache": false, "disable_ocr": true, "enable_quality_processing": false,
        "output_format": "plain", "include_document_structure": true,
        "pages": {"extract_pages": true, "insert_page_markers": false},
        "pdf_options": {"extract_tables": true},
        "concurrency": {"max_threads": 1}
    }))
    .unwrap();
    extract_bytes_document_blocking(bytes, "application/pdf", &config).unwrap()
}

/// The first four cells of each row of page 3: ID, item code, bid and amount stay on one row.
const EXPECTED_ROWS: [(&str, &str, &str, &str); 25] = [
    ("0001", "070030", "5,500.00", "5,500.00"),
    ("0002", "080060", "15,000.00", "15,000.00"),
    ("0003", "090100", "5,000.00", "500,000.00"),
    ("0004", "090205", "7,500.00", "30,000.00"),
    ("0005", "090210", "250.00", "20,000.00"),
    ("0006", "000001", "0.00", "0.00"),
    ("0007", "120090", "32,000.00", "32,000.00"),
    ("0008", "120100", "219,766.00", "219,766.00"),
    ("0009", "120103", "1,000.00", "70,000.00"),
    ("0010", "120159", "1.60", "7,664.00"),
    ("0011", "120165", "68.00", "2,584.00"),
    ("0012", "120300", "5.50", "1,155.00"),
    ("0013", "120320", "20.00", "120,800.00"),
    ("0014", "128651", "10,500.00", "42,000.00"),
    ("0015", "129108", "8,500.00", "76,500.00"),
    ("0016", "129152", "7,000.00", "14,000.00"),
    ("0017", "130100", "50,000.00", "50,000.00"),
    ("0018", "130301", "5,000.00", "5,000.00"),
    ("0019", "130312", "500.00", "17,000.00"),
    ("0020", "130323", "500.00", "7,500.00"),
    ("0021", "130331", "3,000.00", "9,000.00"),
    ("0022", "130560", "0.20", "3,400.00"),
    ("0023", "130570", "2.00", "2,000.00"),
    ("0024", "130640", "4.50", "33,750.00"),
    ("0025", "130650", "7.00", "14,000.00"),
];

#[test]
fn item_schedule_page_keeps_every_row_in_one_native_table() {
    let result = extract(SCHEDULE_PAGE);

    assert_eq!(result.tables.len(), 1, "the schedule must be exactly one native table");
    let table = &result.tables[0];
    assert_eq!(table.cells.len(), EXPECTED_ROWS.len(), "one table row per work item");
    for (row, (id, code, bid, amount)) in table.cells.iter().zip(EXPECTED_ROWS) {
        assert_eq!(row[0], id);
        assert_eq!(row[1], code, "item {id}");
        assert_eq!(&row[row.len() - 2], bid, "item {id}");
        assert_eq!(&row[row.len() - 1], amount, "item {id}");
    }
}

#[test]
fn ruled_summary_page_still_yields_its_table() {
    let result = extract(CONTROL_PAGE);

    assert_eq!(result.tables.len(), 1);
    assert_eq!(result.tables[0].cells.len(), 34);
    assert_eq!(result.tables[0].cells[0].len(), 9);
}
