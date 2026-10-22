//! A UTF-8 byte order mark in front of a markup document must not change what is extracted. The
//! Markdown, Djot, MDX, Org, reStructuredText, Typst, RTF and DocTags extractors kept it as the
//! first character, so a leading heading was read as a paragraph or the mark leaked into the output,
//! and a Jupyter notebook failed to parse. ~keep

use xberg::core::config::ExtractionConfig;

mod helpers;
use helpers::extract_bytes_document;

const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";

async fn assert_a_bom_changes_nothing(mime_type: &str, body: &str) {
    let config = ExtractionConfig::default();
    let plain = extract_bytes_document(body.as_bytes(), mime_type, &config)
        .await
        .unwrap_or_else(|error| panic!("{mime_type} without a BOM should extract: {error}"));
    let with_bom = extract_bytes_document(&[UTF8_BOM, body.as_bytes()].concat(), mime_type, &config)
        .await
        .unwrap_or_else(|error| panic!("{mime_type} with a BOM should extract: {error}"));

    assert!(
        !with_bom.content.contains('\u{feff}'),
        "{mime_type}: the BOM must not reach the content, got {:?}",
        with_bom.content
    );
    assert_eq!(
        with_bom.content, plain.content,
        "{mime_type}: a BOM must not change the extracted content"
    );
}

#[tokio::test]
async fn should_read_markdown_djot_and_doctags_that_start_with_a_utf8_bom() {
    for mime_type in ["text/markdown", "text/x-gfm", "text/x-commonmark", "text/x-quarto"] {
        assert_a_bom_changes_nothing(mime_type, "# Title\n\nBody text.\n").await;
    }
    assert_a_bom_changes_nothing("text/djot", "# Title\n\nBody text.\n").await;
    assert_a_bom_changes_nothing(
        "text/vnd.docling.doctags",
        "<doctag><title>Title</title><text>Body text.</text></doctag>",
    )
    .await;
}

#[cfg(feature = "mdx")]
#[tokio::test]
async fn should_read_mdx_that_starts_with_a_utf8_bom() {
    assert_a_bom_changes_nothing("text/mdx", "# Title\n\nBody text.\n").await;
}

#[cfg(feature = "office")]
#[tokio::test]
async fn should_read_org_rst_typst_and_rtf_that_start_with_a_utf8_bom() {
    assert_a_bom_changes_nothing("text/org", "* Title\nBody text.\n").await;
    assert_a_bom_changes_nothing("text/x-rst", "Title\n=====\n\nBody text.\n").await;
    assert_a_bom_changes_nothing("text/x-typst", "= Title\n\nBody text.\n").await;
    assert_a_bom_changes_nothing("text/rtf", "{\\rtf1\\ansi Hello world}").await;
}

#[cfg(feature = "notebook")]
#[tokio::test]
async fn should_read_a_notebook_that_starts_with_a_utf8_bom() {
    assert_a_bom_changes_nothing(
        "application/x-ipynb+json",
        r##"{"cells": [{"cell_type": "markdown", "metadata": {}, "source": ["# Title"]}], "metadata": {}, "nbformat": 4, "nbformat_minor": 5}"##,
    )
    .await;
}
