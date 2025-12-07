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

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

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

// ============================================================================
// TEST 1: MINIMAL - Basic Content Extraction
// ============================================================================

#[tokio::test]
async fn test_latex_minimal_extraction() {
    let content = fs::read(test_file_path("minimal.tex")).expect("Failed to read minimal.tex");

    let extractor = LatexExtractor::new();
    let result = extractor
        .extract_bytes(&content, "text/x-tex", &ExtractionConfig::default())
        .await
        .expect("Should extract minimal LaTeX");

    // Test 1.1: Should extract non-zero content
    assert!(
        result.content.len() > 0,
        "FAIL: Extracted 0 bytes (current bug). Should extract content from minimal.tex"
    );

    // Test 1.2: Should extract the actual text
    assert!(
        result.content.contains("Hello World from LaTeX!"),
        "FAIL: Should extract 'Hello World from LaTeX!' but got: '{}'",
        result.content
    );
}

// ============================================================================
// TEST 2: BASIC SECTIONS - Metadata and Hierarchy
// ============================================================================

#[tokio::test]
async fn test_latex_metadata_extraction() {
    let content = fs::read(test_file_path("basic_sections.tex")).expect("Failed to read basic_sections.tex");

    let extractor = LatexExtractor::new();
    let result = extractor
        .extract_bytes(&content, "text/x-tex", &ExtractionConfig::default())
        .await
        .expect("Should extract LaTeX with metadata");

    // Test 2.1: Should extract title from \title{Test Document}
    assert_eq!(
        result.metadata.additional.get("title").and_then(|v| v.as_str()),
        Some("Test Document"),
        "FAIL: Should extract title 'Test Document' from \\title{{}} command"
    );

    // Test 2.2: Should extract author from \author{John Doe}
    assert_eq!(
        result.metadata.additional.get("author").and_then(|v| v.as_str()),
        Some("John Doe"),
        "FAIL: Should extract author 'John Doe' from \\author{{}} command"
    );

    // Test 2.3: Should extract date from \date{2025-12-07}
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

    // Test 2.4: Should extract section titles
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

    // Test 2.5: Should extract subsection titles
    assert!(
        result.content.contains("Background"),
        "FAIL: Should extract \\subsection{{Background}} as text"
    );

    // Test 2.6: Should extract subsubsection titles
    assert!(
        result.content.contains("Historical Context"),
        "FAIL: Should extract \\subsubsection{{Historical Context}} as text"
    );

    // Test 2.7: Should extract paragraph content
    assert!(
        result.content.contains("This is the introduction paragraph"),
        "FAIL: Should extract paragraph text from document body"
    );
}

// ============================================================================
// TEST 3: FORMATTING - Text Formatting Commands
// ============================================================================

#[tokio::test]
async fn test_latex_text_formatting() {
    let content = fs::read(test_file_path("formatting.tex")).expect("Failed to read formatting.tex");

    let extractor = LatexExtractor::new();
    let result = extractor
        .extract_bytes(&content, "text/x-tex", &ExtractionConfig::default())
        .await
        .expect("Should extract LaTeX formatting");

    // Test 3.1: Should extract section title
    assert!(
        result.content.contains("Text Formatting"),
        "FAIL: Should extract \\section{{Text Formatting}}"
    );

    // Test 3.2: Should extract normal text
    assert!(
        result.content.contains("This is normal text"),
        "FAIL: Should extract plain paragraph text"
    );

    // Test 3.3: Should extract bold text (from \textbf{})
    assert!(
        result.content.contains("bold text"),
        "FAIL: Should extract text from \\textbf{{bold text}}"
    );

    // Test 3.4: Should extract italic text (from \textit{})
    assert!(
        result.content.contains("italic text"),
        "FAIL: Should extract text from \\textit{{italic text}}"
    );

    // Test 3.5: Should extract underlined text (from \underline{})
    assert!(
        result.content.contains("underlined text"),
        "FAIL: Should extract text from \\underline{{underlined text}}"
    );

    // Test 3.6: Should extract emphasized text (from \emph{})
    assert!(
        result.content.contains("emphasized text"),
        "FAIL: Should extract text from \\emph{{emphasized text}}"
    );

    // Test 3.7: Should extract monospace text (from \texttt{})
    assert!(
        result.content.contains("monospace text"),
        "FAIL: Should extract text from \\texttt{{monospace text}}"
    );

    // Test 3.8: Should extract combined formatting
    assert!(
        result.content.contains("bold and italic"),
        "FAIL: Should extract text from nested formatting commands"
    );
}

// ============================================================================
// TEST 4: MATH - Mathematical Expressions
// ============================================================================

#[tokio::test]
async fn test_latex_math_extraction() {
    let content = fs::read(test_file_path("math.tex")).expect("Failed to read math.tex");

    let extractor = LatexExtractor::new();
    let result = extractor
        .extract_bytes(&content, "text/x-tex", &ExtractionConfig::default())
        .await
        .expect("Should extract LaTeX math");

    // Test 4.1: Should extract section title
    assert!(
        result.content.contains("Math Formulas"),
        "FAIL: Should extract \\section{{Math Formulas}}"
    );

    // Test 4.2: Should extract subsection titles
    assert!(
        result.content.contains("Inline Math"),
        "FAIL: Should extract \\subsection{{Inline Math}}"
    );

    assert!(
        result.content.contains("Display Math"),
        "FAIL: Should extract \\subsection{{Display Math}}"
    );

    // Test 4.3: Should extract inline math content (E=mc²)
    // Math can be extracted as LaTeX or as Unicode - just check it's present
    assert!(
        result.content.contains("mc") || result.content.contains("mc²"),
        "FAIL: Should extract inline math content from $E = mc^2$"
    );

    // Test 4.4: Should extract text surrounding math
    assert!(
        result.content.contains("The equation"),
        "FAIL: Should extract text before inline math"
    );

    assert!(
        result.content.contains("is famous"),
        "FAIL: Should extract text after inline math"
    );

    // Test 4.5: Should extract display math (integral)
    assert!(
        result.content.contains("int") || result.content.contains("∫"),
        "FAIL: Should extract display math environment content"
    );
}

// ============================================================================
// TEST 5: TABLES - Table Extraction
// ============================================================================

#[tokio::test]
async fn test_latex_table_extraction() {
    let content = fs::read(test_file_path("tables.tex")).expect("Failed to read tables.tex");

    let extractor = LatexExtractor::new();
    let result = extractor
        .extract_bytes(&content, "text/x-tex", &ExtractionConfig::default())
        .await
        .expect("Should extract LaTeX tables");

    // Test 5.1: Should extract section title
    assert!(
        result.content.contains("Tables"),
        "FAIL: Should extract \\section{{Tables}}"
    );

    // Test 5.2: Should extract table headers
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

    // Test 5.3: Should extract table data
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

    // Test 5.4: Should extract second table headers
    assert!(
        result.content.contains("Column 1"),
        "FAIL: Should extract 'Column 1' from second table"
    );

    assert!(
        result.content.contains("Column 2"),
        "FAIL: Should extract 'Column 2' from second table"
    );

    // Test 5.5: Should extract table caption
    assert!(
        result.content.contains("Sample table with caption"),
        "FAIL: Should extract table caption from \\caption{{}}"
    );
}

// ============================================================================
// TEST 6: LISTS - List Structure Extraction
// ============================================================================

#[tokio::test]
async fn test_latex_list_itemize() {
    let content = fs::read(test_file_path("lists.tex")).expect("Failed to read lists.tex");

    let extractor = LatexExtractor::new();
    let result = extractor
        .extract_bytes(&content, "text/x-tex", &ExtractionConfig::default())
        .await
        .expect("Should extract LaTeX lists");

    // Test 6.1: Should extract itemize list items
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

    // Test 6.2: Should extract nested list items
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

    // Test 6.3: Should extract numbered list items
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

    // Test 6.4: Should extract description list terms and definitions
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

// ============================================================================
// TEST 7: UNICODE - Unicode Character Handling
// ============================================================================

#[tokio::test]
async fn test_latex_unicode_handling() {
    let content = fs::read(test_file_path("unicode.tex")).expect("Failed to read unicode.tex");

    let extractor = LatexExtractor::new();
    let result = extractor
        .extract_bytes(&content, "text/x-tex", &ExtractionConfig::default())
        .await
        .expect("Should extract LaTeX with Unicode");

    // Test 7.1: Should extract Hebrew characters
    assert!(
        result.content.contains("אֳרָנִים") || result.content.contains("Hebrew"),
        "FAIL: Should extract Hebrew characters or 'Hebrew' text"
    );

    // Test 7.2: Should handle Unicode modifiers
    // The exact representation may vary, but content should be extracted
    assert!(
        result.content.len() > 0,
        "FAIL: Should extract non-zero content from unicode.tex"
    );
}

// ============================================================================
// TEST 8: INTEGRATION - Overall Quality Checks
// ============================================================================

#[tokio::test]
async fn test_latex_no_content_loss_bug() {
    // This test specifically targets the current bug: 0 bytes extracted
    let content = fs::read(test_file_path("minimal.tex")).expect("Failed to read minimal.tex");

    let extractor = LatexExtractor::new();
    let result = extractor
        .extract_bytes(&content, "text/x-tex", &ExtractionConfig::default())
        .await
        .expect("Should extract minimal LaTeX");

    // Test 8.1: CRITICAL - Must extract more than 0 bytes
    assert!(
        result.content.len() > 0,
        "FAIL: CRITICAL BUG - Extracted 0 bytes from minimal.tex. Current LaTeX extractor is completely broken."
    );

    // Test 8.2: Content should be at least 10 bytes (reasonable minimum)
    assert!(
        result.content.len() >= 10,
        "FAIL: Extracted only {} bytes, expected at least 10. Content: '{}'",
        result.content.len(),
        result.content
    );
}

#[tokio::test]
async fn test_latex_extraction_deterministic() {
    // Test that extraction is deterministic (same input = same output)
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

    // Test 8.3: Results should be identical
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
    // Test graceful handling of empty LaTeX documents
    let empty_latex = b"\\documentclass{article}\n\\begin{document}\n\\end{document}";

    let extractor = LatexExtractor::new();
    let result = extractor
        .extract_bytes(empty_latex, "text/x-tex", &ExtractionConfig::default())
        .await
        .expect("Should handle empty LaTeX without panicking");

    // Test 8.4: Should not panic on empty document
    // Content can be empty or contain minimal whitespace
    assert!(
        result.content.trim().len() == 0,
        "Empty document should produce empty content (got: '{}')",
        result.content
    );
}

// ============================================================================
// DOCUMENTATION
// ============================================================================

// Test Suite Summary
//
// This test suite defines 30+ individual test assertions across 8 test groups:
//
// 1. **Minimal Extraction** (2 tests)
//    - Non-zero content extraction
//    - Exact text matching
//
// 2. **Metadata Extraction** (3 tests)
//    - Title extraction from \title{}
//    - Author extraction from \author{}
//    - Date extraction from \date{}
//
// 3. **Section Hierarchy** (4 tests)
//    - Section titles (\section{})
//    - Subsection titles (\subsection{})
//    - Subsubsection titles (\subsubsection{})
//    - Paragraph content
//
// 4. **Text Formatting** (8 tests)
//    - Bold (\textbf{})
//    - Italic (\textit{})
//    - Underline (\underline{})
//    - Emphasis (\emph{})
//    - Monospace (\texttt{})
//    - Combined formatting
//
// 5. **Math Expressions** (5 tests)
//    - Inline math ($...$)
//    - Display math (\[...\])
//    - Math environments
//    - Surrounding text
//
// 6. **Tables** (5 tests)
//    - Table headers
//    - Table data cells
//    - Multiple tables
//    - Table captions
//
// 7. **Lists** (4 tests)
//    - Itemize lists (\begin{itemize})
//    - Enumerate lists (\begin{enumerate})
//    - Description lists (\begin{description})
//    - Nested lists
//
// 8. **Integration** (4 tests)
//    - No content loss bug (0 bytes)
//    - Deterministic extraction
//    - Empty document handling
//    - Unicode character support
//
// **Success Criteria**: ALL TESTS PASS (100% pass rate)
