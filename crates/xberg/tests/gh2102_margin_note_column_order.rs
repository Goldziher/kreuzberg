//! GH#2102 — a narrow column of margin notes beside the main text was read line by line across
//! both columns, and the margin label was welded to the main heading that shares its baseline.
//!
//! The fixture is the reporter's page with the letters inside each line shuffled (text
//! operators, fonts and positions unchanged), so its layout is real and its text unreadable.
//! The notes are 8pt, right-aligned to x=125.3, on their own leading; the main text is 10pt
//! from x=138.3; the gutter is 13pt.

#![cfg(feature = "pdf")]

mod helpers;
use helpers::extract_bytes_document_blocking;
use xberg::{ExtractionConfig, OutputFormat, PdfConfig};

const MARGIN_LABEL: &str = "thoSreladek";
const LAST_MARGIN_NOTE: &str = "düleMr 2017).";
const MAIN_HEADING: &str = "erffnebidurgG";
const FIRST_MAIN_LINE: &str = "eiD mssfthiiirnrckWotafta";

fn extract(output_format: OutputFormat) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pdf/regressions/margin_note/gh2102-margin-note.pdf");
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

fn assert_margin_column_is_read_before_the_main_column(content: &str) {
    let position = |needle: &str| {
        content
            .find(needle)
            .unwrap_or_else(|| panic!("{needle:?} missing from {content:?}"))
    };
    let label = position(MARGIN_LABEL);
    let last_note = position(LAST_MARGIN_NOTE);
    let heading = position(MAIN_HEADING);
    let first_main_line = position(FIRST_MAIN_LINE);

    assert!(
        label < last_note && last_note < heading && heading < first_main_line,
        "the margin label and all of its notes must precede the main heading and its text: {content:?}"
    );
    let label_line = content.lines().find(|line| line.contains(MARGIN_LABEL)).unwrap();
    assert!(
        !label_line.contains(MAIN_HEADING),
        "the margin label must not share a line with the main heading: {label_line:?}"
    );
}

#[test]
fn margin_notes_are_read_before_the_main_column_in_plain_text() {
    assert_margin_column_is_read_before_the_main_column(&extract(OutputFormat::Plain));
}

#[test]
fn margin_notes_are_read_before_the_main_column_in_markdown() {
    assert_margin_column_is_read_before_the_main_column(&extract(OutputFormat::Markdown));
}
