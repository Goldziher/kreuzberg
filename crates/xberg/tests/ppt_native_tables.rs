//! Tables drawn natively on legacy `.ppt` slides, extracted as tables.
//!
//! The deck is `tests/fixtures/ppt/native_tables.ppt`: two slides, each a title over a native
//! table (not an embedded object). Slide 1 has a 3x3 table; slide 2 has a header cell merged
//! across two columns. It was built from a flat ODP with LibreOffice 7.3's PowerPoint 97
//! export, which draws a table the way PowerPoint does: a group shape carrying
//! `tableProperties`, with one text shape per cell. Its `SummaryInformation` stream, which held nothing but LibreOffice's
//! 440 KB preview thumbnail, was left out. ~keep

#![cfg(feature = "office")]

use xberg::core::config::{ExtractInput, ExtractionConfig, OutputFormat};

fn input() -> ExtractInput {
    let deck = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/ppt/native_tables.ppt"
    ))
    .expect("the fixture is in the repository");
    ExtractInput::from_bytes(
        deck,
        "application/vnd.ms-powerpoint",
        Some("native_tables.ppt".to_string()),
    )
}

fn rows(rows: &[&[&str]]) -> Vec<Vec<String>> {
    rows.iter().map(|r| r.iter().map(|c| c.to_string()).collect()).collect()
}

#[tokio::test]
async fn should_extract_each_slide_table_as_a_grid_on_its_slide() {
    let result = xberg::extract(input(), &ExtractionConfig::default())
        .await
        .expect("extract");
    let document = result.results.first().expect("one document");

    let tables: Vec<_> = document
        .tables
        .iter()
        .map(|t| (t.page_number, t.cells.clone()))
        .collect();
    assert_eq!(
        tables,
        vec![
            (
                1,
                rows(&[
                    &["Region", "Q1", "Q2"],
                    &["North", "27,039", "253"],
                    &["South", "1,204", "98"]
                ])
            ),
            (2, rows(&[&["Site", "Quarter totals", ""], &["Lab A", "12", "15"]])),
        ],
        "a merged cell is written once, at its top-left position"
    );
}

#[tokio::test]
async fn should_render_the_table_after_the_slide_title_instead_of_loose_lines() {
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    let result = xberg::extract(input(), &config).await.expect("extract");
    let content = &result.results.first().expect("one document").content;

    let title = content.find("## Cases by region").expect("slide 1 keeps its title");
    let table = content
        .find("| North | 27,039 | 253 |")
        .expect("slide 1's table is rendered as a table");
    assert!(title < table, "{content}");
    assert!(
        !content.lines().any(|line| line.trim() == "27,039"),
        "no cell is left as a loose line: {content}"
    );
}
