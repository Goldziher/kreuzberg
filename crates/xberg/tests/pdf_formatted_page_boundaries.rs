//! `metadata.pages.boundaries` must index the final returned `content` (#2166).
#![cfg(feature = "pdf")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::{ExtractedDocument, ExtractionConfig};

const SYNTHETIC: &[u8] = include_bytes!("../../../fixtures/repros/rendered-page-offsets/synthetic-prices.pdf");

fn config(format: &str, structure: bool) -> ExtractionConfig {
    serde_json::from_value(serde_json::json!({
        "use_cache": false, "disable_ocr": true, "enable_quality_processing": false,
        "output_format": format, "include_document_structure": structure,
        "pages": {"extract_pages": true, "insert_page_markers": false},
        "concurrency": {"max_threads": 1}
    }))
    .unwrap()
}

fn page_slices(result: &ExtractedDocument) -> Vec<(u32, String)> {
    let boundaries = result
        .metadata
        .pages
        .as_ref()
        .and_then(|pages| pages.boundaries.as_ref())
        .expect("page boundaries must be present");
    boundaries
        .iter()
        .map(|b| (b.page_number, result.content[b.byte_start..b.byte_end].to_string()))
        .collect()
}

#[test]
fn page_ranges_select_their_own_footer_in_every_output_format() {
    for format in ["plain", "markdown", "html", "djot"] {
        for structure in [false, true] {
            let result =
                extract_bytes_document_blocking(SYNTHETIC, "application/pdf", &config(format, structure)).unwrap();
            let located = result.metadata.pages.as_ref().is_some_and(|p| p.boundaries.is_some());
            if format == "plain" && structure {
                // The structured plain rendering is not page-verifiable; ranges are either exact or omitted.
                if !located {
                    assert!(result.processing_warnings.iter().any(|w| w.source == "page_boundaries"));
                    continue;
                }
            }
            let slices = page_slices(&result);
            assert_eq!(slices.len(), 2, "format={format} structure={structure}");
            for (page_number, text) in &slices {
                let own = format!("Page {page_number};");
                let other = format!("Page {};", 3 - page_number);
                assert!(
                    text.contains(&own) && !text.contains(&other),
                    "format={format} structure={structure}: range for page {page_number} is wrong: {text:?}"
                );
            }
        }
    }
}
