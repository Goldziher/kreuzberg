//! GH#2110 — on a page with a bordered two-column table, the opening paragraph and the table's
//! right column were read after a section heading far lower on the page, and the short last
//! line of that section's paragraph was split off and read before it.
//!
//! The fixture is the reporter's page with the letters inside each line shuffled (text
//! operators, fonts and positions unchanged), so its layout is real and its text unreadable.

#![cfg(feature = "pdf")]

mod helpers;
use helpers::extract_bytes_document_blocking;
use xberg::{ExtractionConfig, OutputFormat};

const OPENING_PARAGRAPH: &str = "ue ernbuVgkc re lrnewidGeoü";
const SECTION_HEADING: &str = "aeDueise tgileDgsinttlinn";
const SECTION_PARAGRAPH: &str = "issBnrce niDGläDeigimneii";
const SECTION_PARAGRAPH_LAST_LINE: &str = "2017):";

fn extract(output_format: OutputFormat) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pdf/regressions/paragraph_order/gh2110-paragraph-after-heading.pdf");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read fixture {path:?}: {e}"));
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        output_format,
        ..Default::default()
    };
    extract_bytes_document_blocking(&bytes, "application/pdf", &config)
        .unwrap_or_else(|e| panic!("extract fixture {path:?}: {e}"))
        .content
}

fn assert_page_is_read_top_to_bottom(content: &str) {
    let position = |needle: &str| {
        content
            .find(needle)
            .unwrap_or_else(|| panic!("{needle:?} missing from {content:?}"))
    };
    let opening = position(OPENING_PARAGRAPH);
    let heading = position(SECTION_HEADING);
    let paragraph = position(SECTION_PARAGRAPH);
    let last_line = content
        .rfind(SECTION_PARAGRAPH_LAST_LINE)
        .unwrap_or_else(|| panic!("{SECTION_PARAGRAPH_LAST_LINE:?} missing from {content:?}"));

    assert!(
        opening < heading && heading < paragraph && paragraph < last_line,
        "the opening paragraph must precede the 5.2 heading, whose paragraph must end with its last line: {content:?}"
    );
}

#[test]
fn page_is_read_top_to_bottom_in_markdown() {
    assert_page_is_read_top_to_bottom(&extract(OutputFormat::Markdown));
}

#[test]
fn page_is_read_top_to_bottom_in_plain_text() {
    assert_page_is_read_top_to_bottom(&extract(OutputFormat::Plain));
}
