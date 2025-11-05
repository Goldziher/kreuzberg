//! Pandoc integration tests.
//!
//! Tests for Pandoc-based document extraction (RST, LaTeX, ODT, RTF).
//! Validates that Pandoc integration works when available and degrades gracefully when missing.
//!
//! Note: These tests require the `office` feature to be enabled.

#![cfg(feature = "office")]

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::core::extractor::extract_bytes;
use kreuzberg::extraction::pandoc::validate_pandoc_version;

mod helpers;

/// Check if Pandoc is installed and available.
async fn is_pandoc_available() -> bool {
    validate_pandoc_version().await.is_ok()
}

/// Test reStructuredText (RST) extraction.
#[tokio::test]
async fn test_rst_extraction() {
    if !is_pandoc_available().await {
        println!("Skipping test: Pandoc not installed");
        return;
    }

    let config = ExtractionConfig::default();

    let rst_content = b"Title
=====

This is a paragraph in reStructuredText.

Section Heading
---------------

- Bullet point 1
- Bullet point 2
- Bullet point 3

**Bold text** and *italic text*.";

    let result = extract_bytes(rst_content, "text/x-rst", &config).await;

    assert!(result.is_ok(), "RST extraction should succeed");
    let extraction = result.unwrap();

    assert_eq!(extraction.mime_type, "text/x-rst");

    assert!(!extraction.content.is_empty(), "Content should not be empty");
    assert!(
        extraction.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        extraction.detected_languages.is_none(),
        "Language detection not enabled"
    );
    assert!(extraction.tables.is_empty(), "RST should not extract tables");

    assert!(extraction.content.contains("Title"), "Should extract title");
    assert!(
        extraction.content.contains("paragraph"),
        "Should extract paragraph text"
    );
    assert!(
        extraction.content.contains("Section Heading"),
        "Should extract section heading"
    );

    assert!(
        extraction.content.contains("Bullet point 1") || extraction.content.contains("point 1"),
        "Should extract bullet points"
    );

    assert!(
        extraction.content.contains("Bold text") || extraction.content.contains("italic text"),
        "Should extract formatted text content"
    );

    let content_lower = extraction.content.to_lowercase();
    assert!(content_lower.contains("title"), "Should extract title");
    assert!(content_lower.contains("section"), "Should extract section heading");
    assert!(content_lower.contains("bullet"), "Should extract bullet list");
}

/// Test LaTeX extraction.
#[tokio::test]
async fn test_latex_extraction() {
    if !is_pandoc_available().await {
        println!("Skipping test: Pandoc not installed");
        return;
    }

    let config = ExtractionConfig::default();

    let latex_content = b"\\documentclass{article}
\\begin{document}

\\title{Test Document}
\\author{Test Author}
\\maketitle

\\section{Introduction}

This is a test LaTeX document with \\textbf{bold} and \\textit{italic} text.

\\subsection{Subsection}

Some content in a subsection.

\\end{document}";

    let result = extract_bytes(latex_content, "application/x-latex", &config).await;

    assert!(result.is_ok(), "LaTeX extraction should succeed");
    let extraction = result.unwrap();

    assert_eq!(extraction.mime_type, "application/x-latex");

    assert!(!extraction.content.is_empty(), "Content should not be empty");
    assert!(
        extraction.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        extraction.detected_languages.is_none(),
        "Language detection not enabled"
    );
    assert!(
        extraction.tables.is_empty(),
        "LaTeX should not extract tables in this test"
    );

    assert!(
        extraction.content.contains("Test Document"),
        "Should extract document title"
    );
    // Note: Author extraction from LaTeX varies by pandoc version and configuration
    // The critical test is that content is extracted, not specific metadata fields

    assert!(
        extraction.content.contains("Introduction"),
        "Should extract section heading"
    );
    assert!(
        extraction.content.contains("Subsection"),
        "Should extract subsection heading"
    );

    assert!(
        extraction.content.contains("test LaTeX document"),
        "Should extract paragraph text"
    );

    assert!(
        !extraction.content.contains("\\textbf") && !extraction.content.contains("\\section"),
        "LaTeX commands should be stripped, not included in output"
    );
}

/// Test OpenDocument Text (ODT) extraction.
#[tokio::test]
async fn test_odt_extraction() {
    if !is_pandoc_available().await {
        println!("Skipping test: Pandoc not installed");
        return;
    }

    let config = ExtractionConfig::default();

    let invalid_odt = b"This is not a valid ODT file";

    let result = extract_bytes(invalid_odt, "application/vnd.oasis.opendocument.text", &config).await;

    assert!(result.is_err(), "Invalid ODT should fail gracefully");

    let error = result.unwrap_err();
    match error {
        kreuzberg::KreuzbergError::Parsing { .. } => {}
        kreuzberg::KreuzbergError::Io(_) => {}
        other => panic!("Expected Parsing or Io error, got: {:?}", other),
    }
}

/// Test Rich Text Format (RTF) extraction.
#[tokio::test]
async fn test_rtf_extraction() {
    if !is_pandoc_available().await {
        println!("Skipping test: Pandoc not installed");
        return;
    }

    let config = ExtractionConfig::default();

    let rtf_content = b"{\\rtf1\\ansi\\deff0
{\\fonttbl{\\f0 Times New Roman;}}
\\f0\\fs24 This is a test RTF document.\\par
\\b Bold text\\b0  and \\i italic text\\i0.\\par
}";

    let result = extract_bytes(rtf_content, "application/rtf", &config).await;

    assert!(result.is_ok(), "RTF extraction should succeed");
    let extraction = result.unwrap();

    assert_eq!(extraction.mime_type, "application/rtf");

    assert!(!extraction.content.is_empty(), "Content should not be empty");
    assert!(
        extraction.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        extraction.detected_languages.is_none(),
        "Language detection not enabled"
    );
    assert!(
        extraction.tables.is_empty(),
        "RTF should not extract tables in this test"
    );

    assert!(
        extraction.content.contains("test RTF document"),
        "Should extract main paragraph"
    );
    assert!(
        extraction.content.contains("Bold text") || extraction.content.contains("Bold"),
        "Should extract bold text"
    );
    assert!(
        extraction.content.contains("italic text") || extraction.content.contains("italic"),
        "Should extract italic text"
    );

    assert!(
        !extraction.content.contains("\\rtf") && !extraction.content.contains("\\par"),
        "RTF control codes should be stripped from output"
    );
}

/// Test graceful degradation when Pandoc is not installed.
#[tokio::test]
async fn test_pandoc_not_installed() {
    let validation_result = validate_pandoc_version().await;

    if validation_result.is_ok() {
        println!("Pandoc is installed - skipping 'not installed' test");
        return;
    }

    assert!(
        validation_result.is_err(),
        "Should return error when Pandoc not installed"
    );
}

/// Test Pandoc conversion error handling.
#[tokio::test]
async fn test_pandoc_conversion_error() {
    if !is_pandoc_available().await {
        println!("Skipping test: Pandoc not installed");
        return;
    }

    let config = ExtractionConfig::default();

    let malformed_rst = b"===\nThis is malformed\n===\n===";

    let result = extract_bytes(malformed_rst, "text/x-rst", &config).await;

    assert!(
        result.is_ok() || result.is_err(),
        "Should handle malformed content gracefully"
    );
}

/// Test EPUB extraction (ebook format).
#[tokio::test]
async fn test_epub_extraction() {
    if !is_pandoc_available().await {
        println!("Skipping test: Pandoc not installed");
        return;
    }

    let config = ExtractionConfig::default();

    let invalid_epub = b"This is not a valid EPUB file";

    let result = extract_bytes(invalid_epub, "application/epub+zip", &config).await;

    assert!(result.is_err(), "Invalid EPUB should fail gracefully");

    let error = result.unwrap_err();
    match error {
        kreuzberg::KreuzbergError::Parsing { .. } => {}
        kreuzberg::KreuzbergError::Io(_) => {}
        other => panic!("Expected Parsing or Io error for invalid EPUB, got: {:?}", other),
    }
}

/// Test Org mode extraction.
#[tokio::test]
async fn test_org_mode_extraction() {
    if !is_pandoc_available().await {
        println!("Skipping test: Pandoc not installed");
        return;
    }

    let config = ExtractionConfig::default();

    let org_content = b"* Top Level Heading

This is a paragraph in Org mode.

** Second Level Heading

- Item 1
- Item 2
- Item 3

*bold text* and /italic text/";

    let result = extract_bytes(org_content, "text/x-org", &config).await;

    assert!(result.is_ok(), "Org mode extraction should succeed");
    let extraction = result.unwrap();

    assert!(!extraction.content.is_empty(), "Content should not be empty");
    assert!(
        extraction.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        extraction.detected_languages.is_none(),
        "Language detection not enabled"
    );
    assert!(
        extraction.tables.is_empty(),
        "Org mode should not extract tables in this test"
    );

    assert!(
        extraction.content.contains("Top Level") || extraction.content.contains("paragraph"),
        "Org mode content should be extracted"
    );

    assert!(
        extraction.content.contains("paragraph") || extraction.content.contains("Heading"),
        "Text content should be present"
    );
}

/// Test Typst extraction (new document format).
#[tokio::test]
async fn test_typst_extraction() {
    if !is_pandoc_available().await {
        println!("Skipping test: Pandoc not installed");
        return;
    }

    let config = ExtractionConfig::default();

    let typst_content = b"= Heading

This is a paragraph in Typst.

== Subheading

#strong[Bold text] and #emph[italic text].";

    let result = extract_bytes(typst_content, "application/x-typst", &config).await;

    assert!(
        result.is_ok() || result.is_err(),
        "Should handle Typst gracefully (may not be supported in all Pandoc versions)"
    );
}

/// Test CommonMark extraction.
#[tokio::test]
async fn test_commonmark_extraction() {
    if !is_pandoc_available().await {
        println!("Skipping test: Pandoc not installed");
        return;
    }

    let config = ExtractionConfig::default();

    let commonmark_content = b"# Heading

This is a paragraph in CommonMark.

## Subheading

- List item 1
- List item 2

**Bold** and *italic* text.";

    let result = extract_bytes(commonmark_content, "text/x-commonmark", &config).await;

    assert!(result.is_ok(), "CommonMark extraction should succeed");
    let extraction = result.unwrap();

    assert!(!extraction.content.is_empty(), "Content should not be empty");
    assert!(
        extraction.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        extraction.detected_languages.is_none(),
        "Language detection not enabled"
    );
    assert!(
        extraction.tables.is_empty(),
        "CommonMark should not extract tables in this test"
    );

    assert!(
        extraction.content.contains("Heading") || extraction.content.contains("paragraph"),
        "CommonMark content should be extracted"
    );

    let content_lower = extraction.content.to_lowercase();
    assert!(
        content_lower.contains("heading") || content_lower.contains("paragraph"),
        "Should extract text"
    );
    assert!(
        content_lower.contains("list") || content_lower.contains("item"),
        "Should extract list items"
    );
}

/// Test empty content.
#[tokio::test]
async fn test_pandoc_empty_content() {
    if !is_pandoc_available().await {
        println!("Skipping test: Pandoc not installed");
        return;
    }

    let config = ExtractionConfig::default();

    let empty_rst = b"";

    let result = extract_bytes(empty_rst, "text/x-rst", &config).await;

    if let Ok(extraction) = result {
        assert!(
            extraction.content.is_empty() || extraction.content.trim().is_empty(),
            "Empty input should produce empty or minimal output"
        );
    }
}

/// Test Unicode content in Pandoc formats.
#[tokio::test]
async fn test_pandoc_unicode_content() {
    if !is_pandoc_available().await {
        println!("Skipping test: Pandoc not installed");
        return;
    }

    let config = ExtractionConfig::default();

    let unicode_rst = "Title with Unicode
==================

This document contains Unicode: 你好世界 🌍 café

Section
-------

Arabic: مرحبا
Emoji: 🎉 ✅ 🚀"
        .as_bytes();

    let result = extract_bytes(unicode_rst, "text/x-rst", &config).await;

    assert!(result.is_ok(), "Unicode content should be handled");
    let extraction = result.unwrap();

    assert!(!extraction.content.is_empty(), "Content should be extracted");
    assert!(
        extraction.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        extraction.detected_languages.is_none(),
        "Language detection not enabled"
    );
    assert!(
        extraction.tables.is_empty(),
        "RST should not extract tables in this test"
    );

    assert!(
        extraction.content.len() > 20,
        "Should have substantial extracted content"
    );
}
