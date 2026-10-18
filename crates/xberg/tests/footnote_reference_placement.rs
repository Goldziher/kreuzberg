//! A footnote reference belongs to the block that cites it. The Markdown, Djot and LaTeX
//! extractors pushed the `FootnoteRef` the moment they read it, before the block holding it,
//! and renderers attach a reference to the block it follows, so the marker showed up on the
//! previous paragraph, list item or heading. The Djot renderer wrote the marker where the
//! next block starts, which broke that block: `[^1]# Heading`, `[^2]- item two`.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test/bench binaries print by design; org logging policy exempts tests

mod helpers;
use helpers::extract_bytes_document_blocking;

use xberg::core::config::{ExtractionConfig, OutputFormat};

fn extract(input: &str, mime: &str, output_format: OutputFormat) -> String {
    let config = ExtractionConfig {
        output_format,
        ..Default::default()
    };
    extract_bytes_document_blocking(input.as_bytes(), mime, &config)
        .expect("extraction must succeed")
        .content
}

fn assert_contains_all(content: &str, expected: &[&str]) {
    for line in expected {
        assert!(content.contains(line), "{line:?} missing from {content:?}");
    }
}

const MARKDOWN: &str = "# Heading[^h]

First paragraph.

- item one
- item two[^l]

> quoted[^q] text

Last paragraph.

[^h]: Heading note.
[^l]: List note.
[^q]: Quote note.
";

#[test]
fn markdown_references_stay_on_the_block_that_cites_them() {
    let content = extract(MARKDOWN, "text/markdown", OutputFormat::Markdown);
    assert_contains_all(
        &content,
        &[
            "# Heading[^1]\n",
            "\nFirst paragraph.\n",
            "- item one\n",
            "- item two[^2]\n",
            "> quoted text[^3]\n",
        ],
    );
}

#[test]
fn djot_output_keeps_each_reference_on_the_block_that_cites_it() {
    let content = extract(MARKDOWN, "text/markdown", OutputFormat::Djot);
    assert_contains_all(
        &content,
        &[
            "# Heading[^1]\n",
            "- item one\n",
            "- item two[^2]\n",
            "> quoted text[^3]\n",
        ],
    );
}

#[test]
fn djot_references_stay_on_the_paragraph_that_cites_them() {
    let djot = "First paragraph.\n\nBody sentence[^a] continues.\n\n[^a]: The note.\n";
    let content = extract(djot, "text/x-djot", OutputFormat::Markdown);
    assert_contains_all(&content, &["First paragraph.\n", "Body sentence continues.[^1]\n"]);
}

#[cfg(feature = "office")]
#[test]
fn latex_references_stay_on_the_paragraph_that_cites_them() {
    let latex = "\\documentclass{article}\n\\begin{document}\nFirst paragraph.\n\nBody sentence\\footnote{The note.} continues.\n\\end{document}\n";
    let content = extract(latex, "application/x-latex", OutputFormat::Markdown);
    assert_contains_all(&content, &["First paragraph.\n", "Body sentence continues.[^1]\n"]);
}
