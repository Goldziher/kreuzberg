//! GH#2109 — a column of margin notes on the RIGHT of the main text was read line by line
//! across both columns, with the main text set as single justified spans (one 375pt span per
//! line, wider than the page-width furniture cutoff).
//!
//! The fixture is the reporter's page with the letters inside each line shuffled (text
//! operators, fonts and positions unchanged), so its layout is real and its text unreadable.
//! The main text is 10pt from x=82 to x=457; the notes are 8pt from x=470; the gutter is 13pt.

#![cfg(feature = "pdf")]

mod helpers;
use helpers::extract_bytes_document_blocking;
use xberg::{ExtractionConfig, OutputFormat, PdfConfig};

const FIRST_MAIN_LINE: &str = "uzsed, sez eir";
const SECOND_MAIN_LINE: &str = "ei rae iAecmign";
const LAST_MAIN_LINE: &str = "ondnrderhmt.";
const MARGIN_LABEL: &str = "pcftszmseGesoarähs";
const FIRST_MARGIN_NOTE: &str = "cdneiuhirnz egB";
const LAST_MARGIN_NOTE: &str = "ewdern.";

fn extract(output_format: OutputFormat) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pdf/regressions/margin_note/gh2109-right-margin-note.pdf");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read fixture {path:?}: {e}"));
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        output_format,
        pdf_options: Some(PdfConfig {
            extract_tables: false,
            ..Default::default()
        }),
        ..Default::default()
    };
    extract_bytes_document_blocking(&bytes, "application/pdf", &config)
        .unwrap_or_else(|e| panic!("extract fixture {path:?}: {e}"))
        .content
}

fn assert_main_column_is_read_before_the_margin_column(content: &str) {
    let position = |needle: &str| {
        content
            .find(needle)
            .unwrap_or_else(|| panic!("{needle:?} missing from {content:?}"))
    };
    let first_main_line = position(FIRST_MAIN_LINE);
    let second_main_line = position(SECOND_MAIN_LINE);
    let last_main_line = position(LAST_MAIN_LINE);
    let label = position(MARGIN_LABEL);
    let first_note = position(FIRST_MARGIN_NOTE);
    let last_note = position(LAST_MARGIN_NOTE);

    assert!(
        first_main_line < second_main_line && second_main_line < last_main_line,
        "the main column must be read in order: {content:?}"
    );
    assert!(
        last_main_line < label && label < first_note && first_note < last_note,
        "the margin label and its notes must be read as one block after the whole main column: {content:?}"
    );
}

#[test]
fn right_margin_notes_are_read_after_the_main_column_in_plain_text() {
    assert_main_column_is_read_before_the_margin_column(&extract(OutputFormat::Plain));
}

#[test]
fn right_margin_notes_are_read_after_the_main_column_in_markdown() {
    assert_main_column_is_read_before_the_margin_column(&extract(OutputFormat::Markdown));
}
