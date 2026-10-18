//! Word numbers the separator lines in `footnotes.xml`/`endnotes.xml` -1 and 0 and
//! its first real note 1. The note parser skipped id 1 as a separator, so the first
//! footnote and the first endnote of every Word document disappeared, marker and
//! text. Separators are now identified by their `w:type`.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test/bench binaries print by design; org logging policy exempts tests
#![cfg(feature = "office")]

mod helpers;
use helpers::extract_bytes_document_blocking;

use xberg::core::config::{ExtractionConfig, OutputFormat};

const DOCX_MIME: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

#[test]
fn first_footnote_and_endnote_of_a_word_document_are_kept() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/office/word_footnote_ids.docx");
    let bytes = std::fs::read(&path).expect("Word footnote fixture must be present");
    let doc = extract_bytes_document_blocking(&bytes, DOCX_MIME, &ExtractionConfig::default())
        .expect("extraction must succeed");

    assert!(
        doc.content.contains("Body sentence continues and ends.[^1][^2][^3]"),
        "a note marker is missing or duplicated in {:?}",
        doc.content
    );
    for (marker, note) in [
        ("[^1]", "FIRST FOOTNOTE"),
        ("[^2]", "SECOND FOOTNOTE"),
        ("[^3]", "FIRST ENDNOTE"),
    ] {
        let definition = format!("{marker}: {note}");
        assert!(
            doc.content.contains(&definition),
            "{definition:?} missing from {:?}",
            doc.content
        );
        assert_eq!(
            doc.content.matches(marker).count(),
            2,
            "{marker:?} in {:?}",
            doc.content
        );
    }
}

#[test]
fn footnote_and_endnote_with_the_same_id_are_both_kept() {
    // Word numbers footnotes and endnotes independently, so this fixture has a
    // footnote 1 and an endnote 1. Both were keyed `fn1`, and the Markdown notes
    // listed the endnote in place of the first footnote.
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/office/word_footnote_ids.docx");
    let bytes = std::fs::read(&path).expect("Word footnote fixture must be present");
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    let doc = extract_bytes_document_blocking(&bytes, DOCX_MIME, &config).expect("extraction must succeed");

    for note in ["FIRST FOOTNOTE", "SECOND FOOTNOTE", "FIRST ENDNOTE"] {
        assert!(doc.content.contains(note), "{note:?} missing from {:?}", doc.content);
    }
}
