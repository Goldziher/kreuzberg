//! Regression test for #2182 — whitespace-only inline formatting panicked HTML extraction.
//!
//! `html-to-markdown-rs` 3.17 (bundled from 1.3.5 through 1.3.10) panicked with
//! `begin > end` when a paragraph or heading held only whitespace inside formatting, because
//! `StructureCollector::trim_capture` inverted the slice bounds for whitespace-only text.
//! Microsoft Word emits exactly that for a blank line when saving to HTML
//! (`<p class="MsoNormal"><strong><span>&nbsp;</span></strong></p>`), so most Word-exported
//! documents failed to extract. Fixed upstream in html-to-markdown 3.18.0; these tests keep a
//! future crate bump from bringing the panic back.

#![cfg(feature = "html")]

mod helpers;
use helpers::extract_bytes_document_blocking;

use xberg::core::config::{ExtractionConfig, OutputFormat};

fn extract_markdown(html: &[u8]) -> String {
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        use_cache: false,
        ..Default::default()
    };
    extract_bytes_document_blocking(html, "text/html", &config)
        .expect("whitespace-only formatting must not fail the extraction")
        .content
}

/// The minimal input: a non-breaking space alone inside `<b>`, after some content.
#[test]
fn should_extract_a_paragraph_holding_only_a_formatted_nbsp() {
    let content = extract_markdown(b"<p>x</p><p><b>&nbsp;</b></p>");
    assert_eq!(content.trim(), "x");
}

/// A heading holding only a space panicked the same way.
#[test]
fn should_extract_a_whitespace_only_heading() {
    let content = extract_markdown(b"<h1> </h1>");
    assert!(content.trim().is_empty(), "nothing to keep, got: {content:?}");
}

/// The blank line Word writes between paragraphs, as a raw UTF-8 non-breaking space.
#[test]
fn should_keep_the_text_around_a_word_blank_line() {
    let html = "<p class=\"MsoNormal\">First paragraph.</p>\
                <p class=\"MsoNormal\"><strong><span style=\"font-size:12pt\">\u{a0}</span></strong></p>\
                <p class=\"MsoNormal\"><em><!-- c -->&nbsp;<br></em></p>\
                <p class=\"MsoNormal\">Second paragraph.</p>";
    let content = extract_markdown(html.as_bytes());
    assert!(content.contains("First paragraph."), "got: {content:?}");
    assert!(content.contains("Second paragraph."), "got: {content:?}");
}
