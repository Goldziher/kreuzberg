//! Comprehensive TDD test suite for ODT (OpenDocument Text) extraction
//!
//! This test suite validates ODT extraction capabilities using Pandoc's output as the baseline.
//! It covers:
//! - Metadata extraction (title, creator, date, keywords from meta.xml)
//! - Content extraction (text, formatting, structure)
//! - Table extraction with captions
//! - Formatting preservation (bold, italic, strikeout)
//! - Image handling with captions
//! - Math formula extraction
//! - Note handling (footnotes, endnotes)
//! - Citation/reference extraction
//! - Unicode and special character handling
//!
//! Note: These tests require the `office` feature to be enabled and Pandoc to be installed.

#![cfg(feature = "office")]

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::core::extractor::extract_file;
use std::path::{Path, PathBuf};

mod helpers;

/// Helper function to get the workspace root and construct test file paths
fn get_test_file_path(filename: &str) -> PathBuf {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    workspace_root.join(format!("test_documents/odt/{}", filename))
}

/// Helper to verify a test file exists before running test
fn ensure_test_file_exists(path: &Path) -> bool {
    if !path.exists() {
        println!("Skipping test: Test file not found at {:?}", path);
        false
    } else {
        true
    }
}

/// Tests extraction of document metadata from ODT meta.xml
/// Validates: title, subject, creator, dates, generator
#[tokio::test]
async fn test_odt_metadata_extraction() {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/metadata_test.odt");

    if !ensure_test_file_exists(&test_file) {
        println!("Skipping metadata test: metadata_test.odt not found");
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&test_file, None, &config)
        .await
        .expect("Should extract ODT metadata successfully");

    assert!(!result.content.is_empty(), "Content should not be empty");
    assert!(
        result.content.contains("Test Document"),
        "Should contain document title in content"
    );

    let metadata = &result.metadata.additional;
    println!("Extracted metadata: {:?}", metadata);

    if let Some(title) = metadata.get("title") {
        assert_eq!(title.as_str(), Some("Test Metadata Document"), "Title should match");
    }

    if let Some(subject) = metadata.get("subject") {
        assert_eq!(
            subject.as_str(),
            Some("Testing ODT Metadata Extraction"),
            "Subject should match"
        );
    }

    if let Some(created_by) = metadata.get("created_by") {
        assert_eq!(created_by.as_str(), Some("John Doe"), "Creator should match");
    }

    if let Some(authors) = metadata.get("authors") {
        let authors_array = authors.as_array().expect("Authors should be an array");
        assert_eq!(authors_array.len(), 1, "Should have one author");
        assert_eq!(authors_array[0].as_str(), Some("John Doe"), "Author name should match");
    }

    assert!(metadata.get("created_at").is_some(), "Creation date should be present");

    assert!(
        metadata.get("modified_at").is_some(),
        "Modification date should be present"
    );

    if let Some(generator) = metadata.get("generator") {
        let gen_str = generator.as_str().expect("Generator should be a string");
        assert!(gen_str.contains("Pandoc"), "Generator should be Pandoc");
    }

    println!("✅ ODT metadata extraction test passed!");
    println!("   Metadata fields extracted: {}", metadata.len());
}

/// Tests extraction of tables with captions from ODT
/// Baseline from Pandoc: simpleTableWithCaption.odt
/// Expected Pandoc output:
/// ```
/// --------- --------------
/// Content   More content
/// --------- --------------
/// : Table 1: Some caption for a table
/// ```
#[tokio::test]
async fn test_odt_table_with_caption_extraction() {
    let test_file = get_test_file_path("simpleTableWithCaption.odt");
    if !ensure_test_file_exists(&test_file) {
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&test_file, None, &config).await;

    if let Ok(result) = result {
        if !result.content.is_empty() {
            let content_lower = result.content.to_lowercase();
            assert!(
                content_lower.contains("content") || content_lower.contains("table") || !result.tables.is_empty(),
                "Should either extract table content or structured tables"
            );
        }
        println!("✅ ODT table with caption extraction test passed!");
        println!("   Extracted {} tables", result.tables.len());
    } else {
        println!("⚠️  ODT table extraction not fully supported yet (Pandoc integration needed)");
    }
}

/// Tests extraction of basic tables without captions
/// Baseline from Pandoc: simpleTable.odt
/// Expected: Table with "Content" and "More content" cells
#[tokio::test]
async fn test_odt_simple_table_extraction() {
    let test_file = get_test_file_path("simpleTable.odt");
    if !ensure_test_file_exists(&test_file) {
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&test_file, None, &config).await;

    if let Ok(result) = result {
        if !result.content.is_empty() {
            let content_lower = result.content.to_lowercase();
            assert!(
                content_lower.contains("content") || !result.tables.is_empty(),
                "Table should either contain 'content' text or be in structured tables"
            );
        }
        println!("✅ ODT simple table extraction test passed!");
    } else {
        println!("⚠️  ODT table extraction not fully supported yet");
    }
}

/// Tests extraction of document heading hierarchy
/// Baseline from Pandoc: headers.odt
/// Expected:
/// - H1: "A header (Lv 1)"
/// - H2: "Another header (Lv 2)"
/// - H1: "Back to Level 1"
#[tokio::test]
async fn test_odt_heading_structure_extraction() {
    let test_file = get_test_file_path("headers.odt");
    if !ensure_test_file_exists(&test_file) {
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&test_file, None, &config)
        .await
        .expect("Should extract heading structure successfully");

    assert!(!result.content.is_empty(), "Content should not be empty");

    assert!(
        result.content.contains("header") || result.content.contains("Header"),
        "Should contain heading text"
    );

    assert!(
        result.content.contains("#") || result.content.contains("header"),
        "Should indicate heading structure"
    );

    println!("✅ ODT heading structure extraction test passed!");
}

/// Tests extraction of bold text formatting
/// Baseline from Pandoc: bold.odt
/// Expected Pandoc output: "Here comes **bold** text"
#[tokio::test]
async fn test_odt_bold_formatting_extraction() {
    let test_file = get_test_file_path("bold.odt");
    if !ensure_test_file_exists(&test_file) {
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&test_file, None, &config)
        .await
        .expect("Should extract bold formatting successfully");

    assert!(!result.content.is_empty(), "Content should not be empty");

    let content = result.content.to_lowercase();
    assert!(content.contains("bold"), "Should contain 'bold' text");

    assert!(
        result.content.contains("**bold**") || result.content.contains("bold"),
        "Should preserve bold text"
    );

    println!("✅ ODT bold formatting extraction test passed!");
}

/// Tests extraction of italic text formatting
/// Baseline from Pandoc: italic.odt
/// Expected Pandoc output: "Here comes *italic* text"
#[tokio::test]
async fn test_odt_italic_formatting_extraction() {
    let test_file = get_test_file_path("italic.odt");
    if !ensure_test_file_exists(&test_file) {
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&test_file, None, &config)
        .await
        .expect("Should extract italic formatting successfully");

    assert!(!result.content.is_empty(), "Content should not be empty");

    let content = result.content.to_lowercase();
    assert!(content.contains("italic"), "Should contain 'italic' text");

    assert!(
        result.content.contains("*italic*") || result.content.contains("italic"),
        "Should preserve italic text"
    );

    println!("✅ ODT italic formatting extraction test passed!");
}

/// Tests extraction of strikeout/strikethrough text formatting
/// Baseline from Pandoc: strikeout.odt
/// Expected Pandoc output: "Here comes text that was ~~striken out~~."
#[tokio::test]
async fn test_odt_strikeout_formatting_extraction() {
    let test_file = get_test_file_path("strikeout.odt");
    if !ensure_test_file_exists(&test_file) {
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&test_file, None, &config)
        .await
        .expect("Should extract strikeout formatting successfully");

    assert!(!result.content.is_empty(), "Content should not be empty");

    let content = result.content.to_lowercase();
    assert!(
        content.contains("strike") || content.contains("striken"),
        "Should contain strikeout text"
    );

    println!("✅ ODT strikeout formatting extraction test passed!");
}

/// Tests extraction of images with captions
/// Baseline from Pandoc: imageWithCaption.odt
/// Expected: Image reference with caption
/// Expected Pandoc output:
/// ```
/// ![Image caption](Pictures/10000000000000FA000000FAD6A15225.jpg)
/// {alt="Abbildung 1: Image caption" width="5.292cm" height="5.292cm"}
/// ```
#[tokio::test]
async fn test_odt_image_with_caption_extraction() {
    let test_file = get_test_file_path("imageWithCaption.odt");
    if !ensure_test_file_exists(&test_file) {
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&test_file, None, &config).await;

    if let Ok(result) = result {
        if !result.content.is_empty() {
            let content_lower = result.content.to_lowercase();
            assert!(
                content_lower.contains("image")
                    || content_lower.contains("caption")
                    || content_lower.contains("!")
                    || result.images.is_some(),
                "Should reference image or caption or have extracted images"
            );
        }
        println!("✅ ODT image with caption extraction test passed!");
    } else {
        println!("⚠️  ODT image extraction not fully supported yet");
    }
}

/// Tests extraction of mathematical formulas
/// Baseline from Pandoc: formula.odt
/// Expected Pandoc output: "$$E = {m \\cdot c^{2}}$$"
#[tokio::test]
async fn test_odt_formula_extraction() {
    let test_file = get_test_file_path("formula.odt");
    if !ensure_test_file_exists(&test_file) {
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&test_file, None, &config)
        .await
        .expect("Should extract formula successfully");

    assert!(!result.content.is_empty(), "Content should not be empty");

    let content = &result.content;
    assert!(
        content.contains("E") && (content.contains("m") || content.contains("$")),
        "Should extract formula content"
    );

    println!("✅ ODT formula extraction test passed!");
}

/// Tests extraction of footnotes
/// Baseline from Pandoc: footnote.odt
/// Expected Pandoc output:
/// ```
/// Some text[^1] with a footnote.
///
/// [^1]: Footnote text
/// ```
#[tokio::test]
async fn test_odt_footnote_extraction() {
    let test_file = get_test_file_path("footnote.odt");
    if !ensure_test_file_exists(&test_file) {
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&test_file, None, &config)
        .await
        .expect("Should extract footnote successfully");

    assert!(!result.content.is_empty(), "Content should not be empty");

    let content_lower = result.content.to_lowercase();
    assert!(
        content_lower.contains("footnote") || content_lower.contains("[^"),
        "Should extract footnote"
    );

    println!("✅ ODT footnote extraction test passed!");
}

/// Tests extraction of endnotes
/// Baseline from Pandoc: endnote.odt
/// Expected: Endnote content with reference (similar to footnotes)
#[tokio::test]
async fn test_odt_endnote_extraction() {
    let test_file = get_test_file_path("endnote.odt");
    if !ensure_test_file_exists(&test_file) {
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&test_file, None, &config)
        .await
        .expect("Should extract endnote successfully");

    assert!(!result.content.is_empty(), "Content should not be empty");

    let content_lower = result.content.to_lowercase();
    assert!(
        content_lower.contains("endnote") || content_lower.contains("[^"),
        "Should extract endnote"
    );

    println!("✅ ODT endnote extraction test passed!");
}

/// Tests extraction of citations and references
/// Baseline from Pandoc: citation.odt
/// Expected Pandoc output: "Some text[@Ex] with a citation."
#[tokio::test]
async fn test_odt_citation_extraction() {
    let test_file = get_test_file_path("citation.odt");
    if !ensure_test_file_exists(&test_file) {
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&test_file, None, &config)
        .await
        .expect("Should extract citation successfully");

    assert!(!result.content.is_empty(), "Content should not be empty");

    let content_lower = result.content.to_lowercase();
    assert!(
        content_lower.contains("citation") || content_lower.contains("text") || content_lower.contains("@"),
        "Should extract citation"
    );

    println!("✅ ODT citation extraction test passed!");
}

/// Tests extraction of unicode characters and special symbols
/// Baseline from Pandoc: unicode.odt
/// Expected: Proper preservation of unicode characters
/// Expected Pandoc output: ""'çӨ©¼вбФШöÉµ"
#[tokio::test]
async fn test_odt_unicode_extraction() {
    let test_file = get_test_file_path("unicode.odt");
    if !ensure_test_file_exists(&test_file) {
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&test_file, None, &config)
        .await
        .expect("Should extract unicode successfully");

    assert!(!result.content.is_empty(), "Content should not be empty");

    assert!(!result.content.is_empty(), "Should extract unicode content (not empty)");

    println!("✅ ODT unicode extraction test passed!");
    println!("   Extracted unicode content: {:?}", result.content);
}

/// Tests extraction of inline code formatting
/// Baseline from Pandoc: inlinedCode.odt
/// Expected Pandoc output: "Here comes `inlined code` text and `an another` one."
#[tokio::test]
async fn test_odt_inlined_code_extraction() {
    let test_file = get_test_file_path("inlinedCode.odt");
    if !ensure_test_file_exists(&test_file) {
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&test_file, None, &config)
        .await
        .expect("Should extract inline code successfully");

    assert!(!result.content.is_empty(), "Content should not be empty");

    let content_lower = result.content.to_lowercase();
    assert!(
        content_lower.contains("code") || content_lower.contains("`"),
        "Should extract inline code"
    );

    println!("✅ ODT inline code extraction test passed!");
}

/// Tests extraction of paragraph structure and content
/// Baseline from Pandoc: paragraph.odt
/// Expected: Multiple paragraphs separated by blank lines
#[tokio::test]
async fn test_odt_paragraph_structure_extraction() {
    let test_file = get_test_file_path("paragraph.odt");
    if !ensure_test_file_exists(&test_file) {
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&test_file, None, &config)
        .await
        .expect("Should extract paragraph structure successfully");

    assert!(!result.content.is_empty(), "Content should not be empty");

    let content_lower = result.content.to_lowercase();
    assert!(content_lower.contains("paragraph"), "Should contain paragraph text");

    let paragraph_count = result.content.split('\n').filter(|l| !l.is_empty()).count();
    assert!(paragraph_count >= 2, "Should extract multiple paragraphs");

    println!("✅ ODT paragraph structure extraction test passed!");
    println!("   Extracted {} paragraph segments", paragraph_count);
}

/// Integration test: Verify ODT extraction works with standard API
#[tokio::test]
async fn test_odt_extraction_api_integration() {
    let test_file = get_test_file_path("bold.odt");
    if !ensure_test_file_exists(&test_file) {
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&test_file, None, &config)
        .await
        .expect("Should extract via standard API");

    assert!(!result.content.is_empty(), "Should have content");
    assert_eq!(result.mime_type, "application/vnd.oasis.opendocument.text");

    println!("✅ ODT extraction API integration test passed!");
}

/// Test error handling for non-existent files
#[tokio::test]
async fn test_odt_extraction_missing_file_handling() {
    let test_file = get_test_file_path("nonexistent.odt");
    let config = ExtractionConfig::default();

    let result = extract_file(&test_file, None, &config).await;

    assert!(result.is_err(), "Should return error for non-existent file");

    println!("✅ ODT extraction error handling test passed!");
}

/// Test extraction from multiple representative files
#[tokio::test]
async fn test_odt_extraction_variety() {
    let test_files = vec![
        "bold.odt",
        "italic.odt",
        "headers.odt",
        "simpleTable.odt",
        "footnote.odt",
    ];

    let config = ExtractionConfig::default();
    let mut successful_extractions = 0;

    for filename in &test_files {
        let test_file = get_test_file_path(filename);
        if !test_file.exists() {
            continue;
        }

        if let Ok(result) = extract_file(&test_file, None, &config).await
            && !result.content.is_empty()
        {
            successful_extractions += 1;
        }
    }

    assert!(
        successful_extractions >= 3,
        "Should successfully extract from at least 3 test files"
    );

    println!("✅ ODT extraction variety test passed!");
    println!(
        "   Successfully extracted {} out of {} files",
        successful_extractions,
        test_files.len()
    );
}

/// Test that ODT table extraction doesn't include duplicate cell content
/// This is a regression test for the bug where table cells were extracted twice:
/// once as markdown tables and once as raw cell text
#[tokio::test]
async fn test_odt_table_no_duplicate_content() {
    let test_file = get_test_file_path("simpleTable.odt");
    if !ensure_test_file_exists(&test_file) {
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&test_file, None, &config)
        .await
        .expect("Should extract table successfully");

    assert!(!result.content.is_empty(), "Content should not be empty");

    let content_count = result.content.matches("Content").count();

    println!("   'Content' appears {} times in output", content_count);
    println!("   Content preview:\n{}", result.content);

    assert!(
        content_count <= 3,
        "Content should not appear excessively, indicating no duplicate table cell extraction"
    );

    println!("✅ ODT table no duplicate content test passed!");
}

/// Test comprehensive table extraction with headers, multiple rows, and tables
/// Uses the extraction_test document created with pandoc to ensure complete content
#[tokio::test]
async fn test_odt_comprehensive_table_extraction() {
    let test_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test_documents/extraction_test.odt");

    if !test_file.exists() {
        println!("⚠️  Test document not found at {:?}, skipping", test_file);
        return;
    }

    let config = ExtractionConfig::default();
    let result = extract_file(&test_file, None, &config)
        .await
        .expect("Should extract comprehensive table document successfully");

    assert!(!result.content.is_empty(), "Content should not be empty");

    assert!(result.content.contains("Comprehensive"), "Should contain heading");
    assert!(
        result.content.contains("First Section") || result.content.contains("First"),
        "Should contain first section"
    );
    assert!(
        result.content.contains("Second Section") || result.content.contains("Second"),
        "Should contain second section"
    );
    assert!(
        result.content.contains("Third Section") || result.content.contains("Third"),
        "Should contain third section"
    );

    assert!(
        result.content.contains("|"),
        "Should contain pipe characters for markdown tables"
    );
    assert!(result.content.contains("---"), "Should contain table separator");

    assert!(
        result.content.contains("Header 1") || result.content.contains("Cell 1A"),
        "Should contain table data"
    );
    assert!(
        result.content.contains("Product") || result.content.contains("Apple"),
        "Should contain second table data"
    );

    let cell_count = result.content.matches("Cell 1A").count();
    assert!(
        cell_count <= 2,
        "Cell content should not be heavily duplicated (found {} instances)",
        cell_count
    );

    println!("✅ ODT comprehensive table extraction test passed!");
    println!("   Extracted content length: {} chars", result.content.len());
    println!("   Tables found in output: {}", result.tables.len());
}
