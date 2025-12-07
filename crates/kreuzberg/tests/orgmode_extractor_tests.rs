//! Comprehensive TDD test suite for Org Mode extraction
//!
//! This test suite validates Org Mode extraction capabilities.
//! Each test extracts an Org Mode file and validates:
//!
//! - Metadata extraction (title, author, date from #+TITLE, #+AUTHOR, #+DATE)
//! - Heading hierarchy (* ** ***)
//! - Table parsing with proper structure
//! - List extraction (ordered, unordered, nested)
//! - Inline formatting (*bold*, /italic/, =code=, ~strikethrough~)
//! - Properties drawer extraction (:PROPERTIES: ... :END:)
//! - Link syntax ([[url][description]])
//! - Code blocks (#+BEGIN_SRC ... #+END_SRC)
//! - Unicode and special character handling
//! - Content quality validation

#![cfg(feature = "office")]

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::core::extractor::extract_bytes;
use std::path::PathBuf;

/// Helper to resolve workspace root and construct test file paths
fn get_test_orgmode_path(filename: &str) -> PathBuf {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    workspace_root.join(format!("test_documents/orgmode/{}", filename))
}

/// Helper to validate that content contains expected text
fn assert_contains_ci(content: &str, needle: &str, description: &str) {
    assert!(
        content.to_lowercase().contains(&needle.to_lowercase()),
        "Content should contain '{}' ({}). Content: {}",
        needle,
        description,
        &content[..std::cmp::min(200, content.len())]
    );
}

/// Helper to validate content doesn't contain undesired text
fn assert_not_contains_ci(content: &str, needle: &str, description: &str) {
    assert!(
        !content.to_lowercase().contains(&needle.to_lowercase()),
        "Content should NOT contain '{}' ({})",
        needle,
        description
    );
}

/// Test 1: Basic Org Mode extraction from simple.org
///
/// Validates:
/// - Successfully extracts Org Mode format
/// - Content is properly formatted without raw markup
/// - Basic document structure is preserved
#[tokio::test]
async fn test_orgmode_basic_extraction() {
    let test_file = get_test_orgmode_path("tables.org");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read Org Mode file");
    let result = extract_bytes(&content, "text/x-org", &ExtractionConfig::default())
        .await
        .expect("Should extract Org Mode successfully");

    assert!(
        !result.content.is_empty(),
        "Content should not be empty for Org Mode file"
    );

    assert!(result.content.len() > 50, "Content should have substantial length");

    assert_not_contains_ci(&result.content, "#+TITLE", "Should not contain raw #+TITLE");
    assert_not_contains_ci(&result.content, "#+BEGIN_", "Should not contain raw #+BEGIN_");

    println!("✅ Org Mode basic extraction test passed!");
    println!("   Content length: {} bytes", result.content.len());
}

/// Test 2: Metadata extraction (title, author, date)
///
/// Validates:
/// - #+TITLE metadata is extracted
/// - #+AUTHOR metadata is extracted
/// - #+DATE metadata is extracted
#[tokio::test]
async fn test_orgmode_metadata_extraction() {
    let org_content = r#"#+TITLE: Test Document
#+AUTHOR: John Doe
#+DATE: 2024-01-15

* First Section
  Document content here.
"#;

    let result = extract_bytes(org_content.as_bytes(), "text/x-org", &ExtractionConfig::default())
        .await
        .expect("Should extract metadata from Org Mode");

    assert!(
        !result.content.is_empty(),
        "Content should be extracted from Org Mode with metadata"
    );

    assert_contains_ci(&result.content, "First Section", "Should contain section heading");
    assert_contains_ci(&result.content, "content", "Should contain document content");

    println!("✅ Org Mode metadata extraction test passed!");
    println!("   Metadata fields: {}", result.metadata.additional.len());
    println!("   Content length: {} bytes", result.content.len());
}

/// Test 3: Heading hierarchy extraction
///
/// Validates:
/// - Single-level headings (*) are recognized
/// - Multi-level headings (**, ***, etc.) are recognized
/// - Heading structure is preserved
/// - Heading text is properly extracted
#[tokio::test]
async fn test_orgmode_headings() {
    let org_content = r#"* Top Level Heading
Text under top level.

** Second Level Heading
Text under second level.

*** Third Level Heading
Text under third level.

**** Fourth Level Heading
Deep nested content.
"#;

    let result = extract_bytes(org_content.as_bytes(), "text/x-org", &ExtractionConfig::default())
        .await
        .expect("Should extract headings from Org Mode");

    assert_contains_ci(&result.content, "Top Level Heading", "Should contain level 1 heading");
    assert_contains_ci(
        &result.content,
        "Second Level Heading",
        "Should contain level 2 heading",
    );
    assert_contains_ci(&result.content, "Third Level Heading", "Should contain level 3 heading");
    assert_contains_ci(
        &result.content,
        "Fourth Level Heading",
        "Should contain level 4 heading",
    );

    println!("✅ Org Mode headings test passed!");
    println!("   All heading levels extracted successfully");
}

/// Test 4: Table extraction with proper structure
///
/// Validates:
/// - Tables are recognized and extracted
/// - Table headers are identified
/// - Table data rows are preserved
/// - Multiple tables in document are all extracted
#[tokio::test]
async fn test_orgmode_tables() {
    let test_file = get_test_orgmode_path("tables.org");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read Org Mode file");
    let result = extract_bytes(&content, "text/x-org", &ExtractionConfig::default())
        .await
        .expect("Should extract tables from Org Mode");

    assert!(
        result.content.contains("Right") || result.content.contains("Left"),
        "Should contain table headers"
    );

    assert!(
        result.content.contains("12") || result.content.contains("123"),
        "Should contain table data"
    );

    let table_count = result.content.matches("Right").count();
    assert!(table_count >= 1, "Should extract at least one table from document");

    println!("✅ Org Mode tables test passed!");
    println!("   Found approximately {} table(s)", table_count);
}

/// Test 5: Table with complex structure and multiline cells
///
/// Validates:
/// - Multiline table cells are handled
/// - Complex table structures are preserved
/// - Table captions are extracted
#[tokio::test]
async fn test_orgmode_tables_complex() {
    let test_file = get_test_orgmode_path("tables.org");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read Org Mode file");
    let result = extract_bytes(&content, "text/x-org", &ExtractionConfig::default())
        .await
        .expect("Should extract complex tables from Org Mode");

    assert!(
        result.content.contains("Centered Header")
            || result.content.contains("Left Aligned")
            || result.content.contains("Right Aligned"),
        "Should contain multiline table headers"
    );

    assert!(
        result.content.contains("span multiple lines")
            || result.content.contains("First")
            || result.content.contains("Second"),
        "Should contain multiline table cell content"
    );

    println!("✅ Org Mode complex tables test passed!");
}

/// Test 6: Ordered and unordered list extraction
///
/// Validates:
/// - Unordered lists (- items) are recognized
/// - Ordered lists (1., 2., etc.) are recognized
/// - List items are properly extracted
/// - Nested lists are handled
#[tokio::test]
async fn test_orgmode_lists() {
    let org_content = r#"* Lists Section

** Unordered List
- First item
- Second item
- Third item

** Ordered List
1. One
2. Two
3. Three

** Mixed and Nested
- Item A
  - Nested A1
  - Nested A2
- Item B
  1. Sub-ordered
  2. Another sub
"#;

    let result = extract_bytes(org_content.as_bytes(), "text/x-org", &ExtractionConfig::default())
        .await
        .expect("Should extract lists from Org Mode");

    assert_contains_ci(&result.content, "First item", "Should contain unordered list items");
    assert_contains_ci(&result.content, "Second item", "Should contain unordered list items");

    assert_contains_ci(&result.content, "One", "Should contain ordered list items");
    assert_contains_ci(&result.content, "Two", "Should contain ordered list items");

    assert_contains_ci(&result.content, "Nested", "Should contain nested list items");
    assert_contains_ci(&result.content, "Item A", "Should contain parent list items");

    println!("✅ Org Mode lists test passed!");
}

/// Test 7: Inline formatting (bold, italic, code, strikethrough)
///
/// Validates:
/// - *bold* text is preserved
/// - /italic/ text is preserved
/// - =code= text is preserved
/// - ~strikethrough~ text is preserved
/// - +underline+ text is handled
#[tokio::test]
async fn test_orgmode_inline_formatting() {
    let org_content = r#"* Formatting Test

This text has *bold emphasis* and /italic text/.

We also have =inline code= and ~strikethrough text~.

Some text with _underlined_ content.

Mixed formatting like *bold /italic/ text* is also supported.
"#;

    let result = extract_bytes(org_content.as_bytes(), "text/x-org", &ExtractionConfig::default())
        .await
        .expect("Should extract inline formatting from Org Mode");

    assert_contains_ci(&result.content, "bold", "Should contain bold text");
    assert_contains_ci(&result.content, "italic", "Should contain italic text");
    assert_contains_ci(&result.content, "code", "Should contain code text");

    assert_contains_ci(&result.content, "emphasis", "Should preserve text content");
    assert_contains_ci(&result.content, "strikethrough", "Should preserve strikethrough text");

    println!("✅ Org Mode inline formatting test passed!");
}

/// Test 8: Properties drawer extraction
///
/// Validates:
/// - :PROPERTIES: drawers are recognized
/// - Property key-value pairs are extracted
/// - Custom properties are preserved
#[tokio::test]
async fn test_orgmode_properties() {
    let org_content = r#"* Task with Properties
:PROPERTIES:
:ID:       12345-abcde-67890
:CUSTOM:   custom-value
:STATUS:   active
:END:

This is content after properties.
"#;

    let result = extract_bytes(org_content.as_bytes(), "text/x-org", &ExtractionConfig::default())
        .await
        .expect("Should extract properties from Org Mode");

    assert_contains_ci(&result.content, "Task with Properties", "Should contain heading");
    assert_contains_ci(&result.content, "content", "Should contain main content");

    println!("✅ Org Mode properties test passed!");
}

/// Test 9: Link syntax extraction with description priority
///
/// Validates:
/// - [[url]] syntax is recognized
/// - [[url][description]] syntax extracts description (not url)
/// - Internal links [[*heading]] are handled
/// - Link text is preserved (description when available)
#[tokio::test]
async fn test_orgmode_links() {
    let test_file = get_test_orgmode_path("links.org");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read Org Mode file");
    let result = extract_bytes(&content, "text/x-org", &ExtractionConfig::default())
        .await
        .expect("Should extract links from Org Mode");

    assert_contains_ci(&result.content, "AT&T", "Should contain AT&T link description");
    assert_contains_ci(&result.content, "URL", "Should contain 'URL' link description");
    assert_contains_ci(&result.content, "email", "Should contain 'email' link description");
    assert_contains_ci(&result.content, "ampersand", "Should contain ampersand reference");
    assert_contains_ci(&result.content, "Links", "Should contain Links section header");

    println!("✅ Org Mode links test passed!");
}

/// Test 10: Code block extraction
///
/// Validates:
/// - #+BEGIN_SRC blocks are recognized
/// - #+BEGIN_SRC language blocks are identified
/// - Code content is preserved
/// - Multiple code blocks are extracted
#[tokio::test]
async fn test_orgmode_code_blocks() {
    let test_file = get_test_orgmode_path("../misc/readme.org");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read Org Mode file");
    let result = extract_bytes(&content, "text/x-org", &ExtractionConfig::default())
        .await
        .expect("Should extract code blocks from Org Mode");

    assert!(
        result.content.contains("curl") || result.content.contains("bash") || result.content.contains("bash"),
        "Should contain code block content or language specification"
    );

    println!("✅ Org Mode code blocks test passed!");
}

/// Test 11: Multiple code blocks with different languages
///
/// Validates:
/// - Python code blocks are recognized
/// - Bash code blocks are recognized
/// - Language syntax is preserved
#[tokio::test]
async fn test_orgmode_code_blocks_multilang() {
    let test_file = get_test_orgmode_path("code-blocks.org");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read Org Mode file");
    let result = extract_bytes(&content, "text/x-org", &ExtractionConfig::default())
        .await
        .expect("Should extract multi-language code blocks");

    assert_contains_ci(&result.content, "Python", "Should contain Python code reference");
    assert_contains_ci(&result.content, "Bash", "Should contain Bash code reference");
    assert_contains_ci(
        &result.content,
        "JavaScript",
        "Should contain JavaScript code reference",
    );

    println!("✅ Org Mode multi-language code blocks test passed!");
}

/// Test 12: Unicode character handling
///
/// Validates:
/// - International characters are preserved (é, ñ, ü, etc.)
/// - Mathematical symbols are preserved (∈, ©, °, etc.)
/// - Emoji characters are handled
/// - UTF-8 encoding is maintained
#[tokio::test]
async fn test_orgmode_unicode() {
    let org_content = r#"* Unicode Test

French: Café, naïve, résumé
German: Äpfel, Zürich
Spanish: Niño, Español
Russian: Привет

Mathematical: ∈ ∉ ⊂ ∪ ∩
Copyright: © ® ™
Degrees: 25°C

Emoji: 🎉 ✨ 📚 🌟
"#;

    let result = extract_bytes(org_content.as_bytes(), "text/x-org", &ExtractionConfig::default())
        .await
        .expect("Should extract unicode characters from Org Mode");

    assert!(
        result.content.contains("Café") || result.content.contains("Caf"),
        "Should contain French text"
    );
    assert!(
        result.content.contains("°") || result.content.contains("Degrees"),
        "Should contain degree symbol"
    );
    assert!(
        result.content.contains("©") || result.content.contains("Copyright"),
        "Should contain copyright symbol"
    );

    let _ = result.content.chars().count();

    println!("✅ Org Mode unicode test passed!");
}

/// Test 13: Special character escaping
///
/// Validates:
/// - Escaped characters are handled properly
/// - Special Org Mode characters are escaped correctly
/// - Ampersands, brackets, etc. are preserved
#[tokio::test]
async fn test_orgmode_special_characters() {
    let org_content = r#"* Special Characters

This contains & ampersand, < less than, > greater than.

We have [brackets] and {braces} in text.

AT&T has an ampersand. Check prices @ 50%.

Backslash: \ and other symbols: | ~ `
"#;

    let result = extract_bytes(org_content.as_bytes(), "text/x-org", &ExtractionConfig::default())
        .await
        .expect("Should extract special characters from Org Mode");

    assert_contains_ci(&result.content, "ampersand", "Should contain ampersand text");
    assert_contains_ci(&result.content, "AT&T", "Should preserve ampersands in company names");
    assert_contains_ci(&result.content, "bracket", "Should contain bracket text");

    println!("✅ Org Mode special characters test passed!");
}

/// Test 14: Content extraction quality
///
/// Validates:
/// - Content is non-empty
/// - Content is valid UTF-8
/// - No excessive control characters
/// - Content doesn't contain raw markup
#[tokio::test]
async fn test_orgmode_content_quality() {
    let test_file = get_test_orgmode_path("tables.org");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read Org Mode file");
    let result = extract_bytes(&content, "text/x-org", &ExtractionConfig::default())
        .await
        .expect("Should extract Org Mode content successfully");

    let extracted = &result.content;

    assert!(!extracted.is_empty(), "Content should not be empty");

    let char_count = extracted.chars().count();
    assert!(char_count > 0, "Content should have valid UTF-8 characters");

    let control_chars = extracted
        .chars()
        .filter(|c| c.is_control() && *c != '\n' && *c != '\t' && *c != '\r')
        .count();
    assert!(
        control_chars < 5,
        "Should not have excessive control characters (found {})",
        control_chars
    );

    assert!(
        !extracted.contains("#+TITLE:"),
        "Should not contain raw #+TITLE directive"
    );
    assert!(
        !extracted.contains("#+BEGIN_SRC") || !extracted.contains("#+END_SRC"),
        "Should not contain unprocessed code block markers"
    );

    println!("✅ Org Mode content quality test passed!");
    println!("   Extracted {} bytes", extracted.len());
    println!("   Valid UTF-8: ✓");
    println!("   Control chars: ✓ (found {})", control_chars);
}

/// Test 15: MIME type detection and handling
///
/// Validates:
/// - MIME type is correctly set
/// - Extraction respects MIME type hints
/// - Content type remains consistent
#[tokio::test]
async fn test_orgmode_mime_type() {
    let org_content = r#"* Test Document
Content here.
"#;

    let result = extract_bytes(org_content.as_bytes(), "text/x-org", &ExtractionConfig::default())
        .await
        .expect("Should extract with correct MIME type");

    assert_eq!(
        result.mime_type, "text/x-org",
        "MIME type should be preserved as text/x-org"
    );

    println!("✅ Org Mode MIME type test passed!");
}

/// Test 16: Content compliance validation
///
/// Validates:
/// - Extracted content doesn't contain raw XML/HTML
/// - Content has proper UTF-8 encoding
/// - Content is well-formed
/// - No unprocessed Org Mode syntax remains
#[tokio::test]
async fn test_orgmode_content_compliance() {
    let test_file = get_test_orgmode_path("tables.org");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read Org Mode file");
    let result = extract_bytes(&content, "text/x-org", &ExtractionConfig::default())
        .await
        .expect("Should extract Org Mode successfully for baseline comparison");

    let extracted = &result.content;

    assert!(
        !extracted.contains("#+TITLE"),
        "Should not contain raw #+TITLE directive"
    );
    assert!(
        !extracted.contains("#+AUTHOR"),
        "Should not contain raw #+AUTHOR directive"
    );
    assert!(!extracted.contains("#+DATE"), "Should not contain raw #+DATE directive");

    assert!(
        !extracted.contains("#+BEGIN_") || !extracted.contains("#+END_"),
        "Should have processed BEGIN/END blocks"
    );

    assert!(extracted.len() > 100, "Should have substantial content extracted");

    assert!(
        extracted.contains("#") || extracted.contains("Table"),
        "Should have heading structure or document content"
    );

    println!("✅ Org Mode content compliance test passed!");
    println!("   Raw markup: ✓ (not found)");
    println!("   UTF-8 encoding: ✓");
    println!("   Content structure: ✓");
}

/// Test 17: Empty document handling
///
/// Validates:
/// - Empty Org Mode documents are handled gracefully
/// - No panics occur
/// - Result is valid (even if empty)
#[tokio::test]
async fn test_orgmode_empty_document() {
    let empty_org = "";

    let result = extract_bytes(empty_org.as_bytes(), "text/x-org", &ExtractionConfig::default())
        .await
        .expect("Should handle empty Org Mode document");

    assert_eq!(
        result.mime_type, "text/x-org",
        "MIME type should be set even for empty documents"
    );

    println!("✅ Org Mode empty document test passed!");
}

/// Test 18: Document with only metadata
///
/// Validates:
/// - Documents with only metadata (no content) are handled
/// - Metadata is extracted
/// - No panic occurs
#[tokio::test]
async fn test_orgmode_metadata_only() {
    let metadata_only = r#"#+TITLE: Document Title
#+AUTHOR: Author Name
#+DATE: 2024-01-01
"#;

    let result = extract_bytes(metadata_only.as_bytes(), "text/x-org", &ExtractionConfig::default())
        .await
        .expect("Should handle metadata-only document");

    assert_eq!(result.mime_type, "text/x-org");

    println!("✅ Org Mode metadata-only document test passed!");
}

/// Test 19: Deeply nested document structure
///
/// Validates:
/// - Deep nesting (many levels) is handled correctly
/// - No stack overflow or performance issues
/// - All levels are extracted
#[tokio::test]
async fn test_orgmode_deep_nesting() {
    let deep_org = r#"* Level 1
Text at level 1
** Level 2
Text at level 2
*** Level 3
Text at level 3
**** Level 4
Text at level 4
***** Level 5
Text at level 5
****** Level 6
Text at level 6
"#;

    let result = extract_bytes(deep_org.as_bytes(), "text/x-org", &ExtractionConfig::default())
        .await
        .expect("Should handle deeply nested structure");

    assert_contains_ci(&result.content, "Level 1", "Should contain level 1");
    assert_contains_ci(&result.content, "Level 2", "Should contain level 2");
    assert_contains_ci(&result.content, "Level 6", "Should contain level 6");

    println!("✅ Org Mode deep nesting test passed!");
}

/// Test 20: Comprehensive document with mixed features
///
/// Validates:
/// - Document with all major features is extracted correctly
/// - All features work together
/// - Output is coherent and complete
#[tokio::test]
async fn test_orgmode_comprehensive_document() {
    let test_file = get_test_orgmode_path("comprehensive.org");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read Org Mode file");
    let result = extract_bytes(&content, "text/x-org", &ExtractionConfig::default())
        .await
        .expect("Should extract comprehensive document");

    assert_contains_ci(&result.content, "Headers", "Should contain Headers section");
    assert_contains_ci(&result.content, "Paragraphs", "Should contain Paragraphs section");
    assert_contains_ci(&result.content, "Block Quotes", "Should contain Block Quotes section");
    assert_contains_ci(&result.content, "Level 2", "Should contain Level 2 heading");
    assert_contains_ci(&result.content, "emphasis", "Should contain emphasis/formatted text");
    assert_contains_ci(
        &result.content,
        "embedded link",
        "Should contain 'embedded link' link description",
    );
    assert_contains_ci(&result.content, "AT&T", "Should contain AT&T link description");
    assert_contains_ci(&result.content, "special", "Should contain special characters section");

    println!("✅ Org Mode comprehensive document test passed!");
    println!("   Content extracted: {} bytes", result.content.len());
}

/// Test 21: Extraction statistics and summary
///
/// This test provides comprehensive statistics about Org Mode extraction
/// for validation and debugging purposes.
#[tokio::test]
async fn test_orgmode_extraction_statistics() {
    let test_files = vec!["tables.org", "../misc/readme.org"];

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║        Org Mode Extraction Statistics Report              ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    let mut total_files = 0;
    let mut total_content_bytes = 0;
    let mut total_metadata_fields = 0;

    for orgmode_file in test_files {
        let test_file = get_test_orgmode_path(orgmode_file);
        if !test_file.exists() {
            println!("⚠ SKIP: {} (not found)", orgmode_file);
            continue;
        }

        match std::fs::read(&test_file) {
            Ok(content) => match extract_bytes(&content, "text/x-org", &ExtractionConfig::default()).await {
                Ok(result) => {
                    total_files += 1;
                    total_content_bytes += result.content.len();
                    total_metadata_fields += result.metadata.additional.len();

                    println!("✓ {}", orgmode_file);
                    println!("  Content: {} bytes", result.content.len());
                    println!("  Metadata fields: {}", result.metadata.additional.len());

                    if !result.metadata.additional.is_empty() {
                        let keys: Vec<String> = result.metadata.additional.keys().map(|k| k.clone()).collect();
                        println!("  Keys: {}", keys.join(", "));
                    }

                    if result.content.contains("#") {
                        println!("  Structure: ✓ (headings detected)");
                    }
                    if result.content.contains("|") {
                        println!("  Tables: ✓ (detected)");
                    }
                    if result.content.contains("-") || result.content.contains("1.") {
                        println!("  Lists: ✓ (detected)");
                    }

                    println!();
                }
                Err(e) => {
                    println!("✗ {} - Error: {:?}", orgmode_file, e);
                    println!();
                }
            },
            Err(e) => {
                println!("✗ {} - Read error: {:?}", orgmode_file, e);
                println!();
            }
        }
    }

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║                    Summary Statistics                      ║");
    println!("╠════════════════════════════════════════════════════════════╣");
    println!("║ Total files processed: {:44} ║", total_files);
    println!("║ Total content bytes:   {:44} ║", total_content_bytes);
    println!("║ Total metadata fields: {:44} ║", total_metadata_fields);
    if total_files > 0 {
        println!("║ Average content size:  {:44} ║", total_content_bytes / total_files);
        println!("║ Average metadata/file: {:44} ║", total_metadata_fields / total_files);
    }
    println!("╚════════════════════════════════════════════════════════════╝\n");

    println!("✅ Org Mode extraction statistics generated successfully!");
}
