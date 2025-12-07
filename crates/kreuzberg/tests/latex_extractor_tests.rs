//! Comprehensive LaTeX Extractor Tests
//!
//! This test suite defines the expected behavior for LaTeX extraction.
//!
//! Test Coverage:
//! - Basic content extraction (minimal.tex)
//! - Section hierarchy (basic_sections.tex)
//! - Text formatting (formatting.tex)
//! - Mathematical expressions (math.tex)
//! - Tables (tables.tex)
//! - Lists (lists.tex)
//! - Unicode handling (unicode.tex)
//!
//! Success Criteria:
//! - All tests passing (100%)
//! - No content loss (extract meaningful content)

#![cfg(feature = "office")]

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::extractors::latex::LatexExtractor;
use kreuzberg::plugins::DocumentExtractor;
use std::fs;
use std::path::PathBuf;

/// Helper to get absolute path to test documents
fn test_file_path(filename: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test_documents")
        .join("latex")
        .join(filename)
}

#[tokio::test]
async fn test_latex_minimal_extraction() {
    let content = fs::read(test_file_path("minimal.tex")).expect("Failed to read minimal.tex");

    let extractor = LatexExtractor::new();
    let result = extractor
        .extract_bytes(&content, "text/x-tex", &ExtractionConfig::default())
        .await
        .expect("Should extract minimal LaTeX");

    assert!(
        result.content.len() > 0,
        "FAIL: Extracted 0 bytes (current bug). Should extract content from minimal.tex"
    );

    assert!(
        result.content.contains("Hello World from LaTeX!"),
        "FAIL: Should extract 'Hello World from LaTeX!' but got: '{}'",
        result.content
    );
}

#[tokio::test]
async fn test_latex_metadata_extraction() {
    let content = fs::read(test_file_path("basic_sections.tex")).expect("Failed to read basic_sections.tex");

    let extractor = LatexExtractor::new();
    let result = extractor
        .extract_bytes(&content, "text/x-tex", &ExtractionConfig::default())
        .await
        .expect("Should extract LaTeX with metadata");

    assert_eq!(
        result.metadata.additional.get("title").and_then(|v| v.as_str()),
        Some("Test Document"),
        "FAIL: Should extract title 'Test Document' from \\title{{}} command"
    );

    assert_eq!(
        result.metadata.additional.get("author").and_then(|v| v.as_str()),
        Some("John Doe"),
        "FAIL: Should extract author 'John Doe' from \\author{{}} command"
    );

    assert_eq!(
        result.metadata.additional.get("date").and_then(|v| v.as_str()),
        Some("2025-12-07"),
        "FAIL: Should extract date '2025-12-07' from \\date{{}} command"
    );
}

#[tokio::test]
async fn test_latex_section_hierarchy() {
    let content = fs::read(test_file_path("basic_sections.tex")).expect("Failed to read basic_sections.tex");

    let extractor = LatexExtractor::new();
    let result = extractor
        .extract_bytes(&content, "text/x-tex", &ExtractionConfig::default())
        .await
        .expect("Should extract LaTeX sections");

    assert!(
        result.content.contains("Introduction"),
        "FAIL: Should extract \\section{{Introduction}} as text"
    );

    assert!(
        result.content.contains("Methods"),
        "FAIL: Should extract \\section{{Methods}} as text"
    );

    assert!(
        result.content.contains("Results"),
        "FAIL: Should extract \\section{{Results}} as text"
    );

    assert!(
        result.content.contains("Background"),
        "FAIL: Should extract \\subsection{{Background}} as text"
    );

    assert!(
        result.content.contains("Historical Context"),
        "FAIL: Should extract \\subsubsection{{Historical Context}} as text"
    );

    assert!(
        result.content.contains("This is the introduction paragraph"),
        "FAIL: Should extract paragraph text from document body"
    );
}

#[tokio::test]
async fn test_latex_text_formatting() {
    let content = fs::read(test_file_path("formatting.tex")).expect("Failed to read formatting.tex");

    let extractor = LatexExtractor::new();
    let result = extractor
        .extract_bytes(&content, "text/x-tex", &ExtractionConfig::default())
        .await
        .expect("Should extract LaTeX formatting");

    assert!(
        result.content.contains("Text Formatting"),
        "FAIL: Should extract \\section{{Text Formatting}}"
    );

    assert!(
        result.content.contains("This is normal text"),
        "FAIL: Should extract plain paragraph text"
    );

    assert!(
        result.content.contains("bold text"),
        "FAIL: Should extract text from \\textbf{{bold text}}"
    );

    assert!(
        result.content.contains("italic text"),
        "FAIL: Should extract text from \\textit{{italic text}}"
    );

    assert!(
        result.content.contains("underlined text"),
        "FAIL: Should extract text from \\underline{{underlined text}}"
    );

    assert!(
        result.content.contains("emphasized text"),
        "FAIL: Should extract text from \\emph{{emphasized text}}"
    );

    assert!(
        result.content.contains("monospace text"),
        "FAIL: Should extract text from \\texttt{{monospace text}}"
    );

    assert!(
        result.content.contains("bold and italic"),
        "FAIL: Should extract text from nested formatting commands"
    );
}

#[tokio::test]
async fn test_latex_math_extraction() {
    let content = fs::read(test_file_path("math.tex")).expect("Failed to read math.tex");

    let extractor = LatexExtractor::new();
    let result = extractor
        .extract_bytes(&content, "text/x-tex", &ExtractionConfig::default())
        .await
        .expect("Should extract LaTeX math");

    assert!(
        result.content.contains("Math Formulas"),
        "FAIL: Should extract \\section{{Math Formulas}}"
    );

    assert!(
        result.content.contains("Inline Math"),
        "FAIL: Should extract \\subsection{{Inline Math}}"
    );

    assert!(
        result.content.contains("Display Math"),
        "FAIL: Should extract \\subsection{{Display Math}}"
    );

    assert!(
        result.content.contains("mc") || result.content.contains("mc²"),
        "FAIL: Should extract inline math content from $E = mc^2$"
    );

    assert!(
        result.content.contains("The equation"),
        "FAIL: Should extract text before inline math"
    );

    assert!(
        result.content.contains("is famous"),
        "FAIL: Should extract text after inline math"
    );

    assert!(
        result.content.contains("int") || result.content.contains("∫"),
        "FAIL: Should extract display math environment content"
    );
}

#[tokio::test]
async fn test_latex_table_extraction() {
    let content = fs::read(test_file_path("tables.tex")).expect("Failed to read tables.tex");

    let extractor = LatexExtractor::new();
    let result = extractor
        .extract_bytes(&content, "text/x-tex", &ExtractionConfig::default())
        .await
        .expect("Should extract LaTeX tables");

    assert!(
        result.content.contains("Tables"),
        "FAIL: Should extract \\section{{Tables}}"
    );

    assert!(
        result.content.contains("Name"),
        "FAIL: Should extract table header 'Name' from tabular"
    );

    assert!(
        result.content.contains("Age"),
        "FAIL: Should extract table header 'Age' from tabular"
    );

    assert!(
        result.content.contains("Score"),
        "FAIL: Should extract table header 'Score' from tabular"
    );

    assert!(
        result.content.contains("Alice"),
        "FAIL: Should extract table cell 'Alice'"
    );

    assert!(result.content.contains("30"), "FAIL: Should extract table cell '30'");

    assert!(result.content.contains("95"), "FAIL: Should extract table cell '95'");

    assert!(result.content.contains("Bob"), "FAIL: Should extract table cell 'Bob'");

    assert!(
        result.content.contains("Charlie"),
        "FAIL: Should extract table cell 'Charlie'"
    );

    assert!(
        result.content.contains("Column 1"),
        "FAIL: Should extract 'Column 1' from second table"
    );

    assert!(
        result.content.contains("Column 2"),
        "FAIL: Should extract 'Column 2' from second table"
    );

    assert!(
        result.content.contains("Sample table with caption"),
        "FAIL: Should extract table caption from \\caption{{}}"
    );
}

#[tokio::test]
async fn test_latex_list_itemize() {
    let content = fs::read(test_file_path("lists.tex")).expect("Failed to read lists.tex");

    let extractor = LatexExtractor::new();
    let result = extractor
        .extract_bytes(&content, "text/x-tex", &ExtractionConfig::default())
        .await
        .expect("Should extract LaTeX lists");

    assert!(
        result.content.contains("First item"),
        "FAIL: Should extract \\item First item from itemize"
    );

    assert!(
        result.content.contains("Second item"),
        "FAIL: Should extract \\item Second item from itemize"
    );

    assert!(
        result.content.contains("Third item with nested list"),
        "FAIL: Should extract \\item Third item with nested list"
    );

    assert!(
        result.content.contains("Fourth item"),
        "FAIL: Should extract \\item Fourth item from itemize"
    );
}

#[tokio::test]
async fn test_latex_list_nested() {
    let content = fs::read(test_file_path("lists.tex")).expect("Failed to read lists.tex");

    let extractor = LatexExtractor::new();
    let result = extractor
        .extract_bytes(&content, "text/x-tex", &ExtractionConfig::default())
        .await
        .expect("Should extract LaTeX nested lists");

    assert!(
        result.content.contains("Nested item 1"),
        "FAIL: Should extract nested \\item Nested item 1"
    );

    assert!(
        result.content.contains("Nested item 2"),
        "FAIL: Should extract nested \\item Nested item 2"
    );
}

#[tokio::test]
async fn test_latex_list_enumerate() {
    let content = fs::read(test_file_path("lists.tex")).expect("Failed to read lists.tex");

    let extractor = LatexExtractor::new();
    let result = extractor
        .extract_bytes(&content, "text/x-tex", &ExtractionConfig::default())
        .await
        .expect("Should extract LaTeX enumerate");

    assert!(
        result.content.contains("First numbered item"),
        "FAIL: Should extract \\item First numbered item from enumerate"
    );

    assert!(
        result.content.contains("Second numbered item"),
        "FAIL: Should extract \\item Second numbered item from enumerate"
    );

    assert!(
        result.content.contains("Third numbered item"),
        "FAIL: Should extract \\item Third numbered item from enumerate"
    );
}

#[tokio::test]
async fn test_latex_list_description() {
    let content = fs::read(test_file_path("lists.tex")).expect("Failed to read lists.tex");

    let extractor = LatexExtractor::new();
    let result = extractor
        .extract_bytes(&content, "text/x-tex", &ExtractionConfig::default())
        .await
        .expect("Should extract LaTeX description lists");

    assert!(
        result.content.contains("Term 1"),
        "FAIL: Should extract \\item[Term 1] from description list"
    );

    assert!(
        result.content.contains("Definition of term 1"),
        "FAIL: Should extract definition text from description list"
    );

    assert!(
        result.content.contains("Term 2"),
        "FAIL: Should extract \\item[Term 2] from description list"
    );

    assert!(
        result.content.contains("Definition of term 2"),
        "FAIL: Should extract definition text from description list"
    );
}

#[tokio::test]
async fn test_latex_lists_pandoc_parity() {
    let content = fs::read(test_file_path("lists.tex")).expect("Failed to read lists.tex");

    let extractor = LatexExtractor::new();
    let result = extractor
        .extract_bytes(&content, "text/x-tex", &ExtractionConfig::default())
        .await
        .expect("Should extract LaTeX lists");
}

#[tokio::test]
async fn test_latex_unicode_handling() {
    let content = fs::read(test_file_path("unicode.tex")).expect("Failed to read unicode.tex");

    let extractor = LatexExtractor::new();
    let result = extractor
        .extract_bytes(&content, "text/x-tex", &ExtractionConfig::default())
        .await
        .expect("Should extract LaTeX with Unicode");

    assert!(
        result.content.contains("אֳרָנִים") || result.content.contains("Hebrew"),
        "FAIL: Should extract Hebrew characters or 'Hebrew' text"
    );

    assert!(
        result.content.len() > 0,
        "FAIL: Should extract non-zero content from unicode.tex"
    );
}

#[tokio::test]
async fn test_latex_no_content_loss_bug() {
    let content = fs::read(test_file_path("minimal.tex")).expect("Failed to read minimal.tex");

    let extractor = LatexExtractor::new();
    let result = extractor
        .extract_bytes(&content, "text/x-tex", &ExtractionConfig::default())
        .await
        .expect("Should extract minimal LaTeX");

    assert!(
        result.content.len() > 0,
        "FAIL: CRITICAL BUG - Extracted 0 bytes from minimal.tex. Current LaTeX extractor is completely broken."
    );

    assert!(
        result.content.len() >= 10,
        "FAIL: Extracted only {} bytes, expected at least 10. Content: '{}'",
        result.content.len(),
        result.content
    );
}

#[tokio::test]
async fn test_latex_extraction_deterministic() {
    let content = fs::read(test_file_path("minimal.tex")).expect("Failed to read minimal.tex");

    let extractor = LatexExtractor::new();

    let result1 = extractor
        .extract_bytes(&content, "text/x-tex", &ExtractionConfig::default())
        .await
        .expect("Should extract LaTeX (first run)");

    let result2 = extractor
        .extract_bytes(&content, "text/x-tex", &ExtractionConfig::default())
        .await
        .expect("Should extract LaTeX (second run)");

    assert_eq!(
        result1.content, result2.content,
        "FAIL: Extraction is not deterministic. Same input produced different outputs."
    );

    assert_eq!(
        result1.metadata.additional, result2.metadata.additional,
        "FAIL: Metadata extraction is not deterministic."
    );
}

#[tokio::test]
async fn test_latex_empty_document_handling() {
    let empty_latex = b"\\documentclass{article}\n\\begin{document}\n\\end{document}";

    let extractor = LatexExtractor::new();
    let result = extractor
        .extract_bytes(empty_latex, "text/x-tex", &ExtractionConfig::default())
        .await
        .expect("Should handle empty LaTeX without panicking");

    assert!(
        result.content.trim().len() == 0,
        "Empty document should produce empty content (got: '{}')",
        result.content
    );
}
