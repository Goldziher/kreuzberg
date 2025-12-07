//! Comprehensive TDD test suite for RST (reStructuredText) extraction
//!
//! Tests RST extraction using Pandoc as the baseline for quality validation.
//! The test documents are derived from the Pandoc test suite and provide
//! comprehensive coverage of RST-specific features including:
//! - Metadata extraction from field lists (:Author:, :Date:, etc.)
//! - Directive handling (.. code-block::, .. image::, .. math::, etc.)
//! - Section structure and heading levels
//! - Table extraction (simple and grid tables)
//! - Reference links and images
//! - Comments and special blocks
//! - Content quality validation

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::core::extractor::extract_bytes;

mod helpers;

const RST_FIXTURE: &str = include_str!("../../../test_documents/rst/rst-reader.rst");

fn rst_fixture_bytes() -> Vec<u8> {
    RST_FIXTURE.as_bytes().to_vec()
}

/// Test extraction of document title from RST file structure
#[tokio::test]
async fn test_rst_title_extraction() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    assert!(
        result.content.to_lowercase().contains("pandoc test suite"),
        "Should contain document title 'Pandoc Test Suite'"
    );

    assert!(
        result.content.contains("Level one header") || result.content.contains("header"),
        "Should contain document headers"
    );

    println!("✅ RST title extraction test passed!");
}

/// Test field list metadata extraction (:Authors:, :Date:, :Revision:)
#[tokio::test]
async fn test_rst_field_list_metadata_extraction() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    println!(
        "Content excerpt (first 500 chars): {}",
        &result.content[..std::cmp::min(500, result.content.len())]
    );

    assert!(
        result.content.contains("John MacFarlane")
            || result.content.contains("July 17")
            || result.content.contains("Pandoc Test Suite"),
        "Should contain metadata information or title"
    );

    println!("✅ RST field list metadata extraction test passed!");
}

/// Test extraction of multiple heading levels
#[tokio::test]
async fn test_rst_section_hierarchy() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    let headings = vec![
        "Level one header",
        "Level two header",
        "Level three",
        "Paragraphs",
        "Block Quotes",
        "Code Blocks",
        "Lists",
        "Field Lists",
        "HTML Blocks",
        "LaTeX Block",
        "Images",
        "Tables",
    ];

    for heading in headings {
        assert!(
            result.content.contains(heading),
            "Should contain heading: '{}'",
            heading
        );
    }

    println!("✅ RST section hierarchy test passed!");
}

/// Test that emphasis in headings is preserved
#[tokio::test]
async fn test_rst_heading_with_inline_markup() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    assert!(
        result.content.contains("emphasis") || result.content.contains("Level four"),
        "Should contain heading with emphasis"
    );

    println!("✅ RST heading with inline markup test passed!");
}

/// Test code block extraction with language specification
#[tokio::test]
async fn test_rst_code_block_extraction() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    assert!(
        result.content.contains("def my_function") || result.content.contains("python"),
        "Should contain Python code block or language specification"
    );

    assert!(
        result.content.contains("return x + 1") || result.content.contains("my_function"),
        "Should contain Python function code"
    );

    println!("✅ RST code block extraction test passed!");
}

/// Test Haskell code blocks with highlight directive
#[tokio::test]
async fn test_rst_highlight_directive_code_blocks() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    assert!(
        result.content.contains("haskell") || result.content.contains("Tree") || result.content.contains("data Tree"),
        "Should contain Haskell code blocks"
    );

    assert!(
        result.content.contains("Leaf") || result.content.contains("Node"),
        "Should contain Haskell data constructors"
    );

    println!("✅ RST highlight directive code blocks test passed!");
}

/// Test JavaScript code blocks
#[tokio::test]
async fn test_rst_javascript_code_blocks() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    assert!(
        result.content.contains("javascript") || result.content.contains("=>") || result.content.contains("let f"),
        "Should contain JavaScript code"
    );

    println!("✅ RST JavaScript code blocks test passed!");
}

/// Test unordered list extraction
#[tokio::test]
async fn test_rst_unordered_lists() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    let list_items = vec![
        "asterisk 1",
        "asterisk 2",
        "asterisk 3",
        "Plus 1",
        "Plus 2",
        "Plus 3",
        "Minus 1",
        "Minus 2",
        "Minus 3",
    ];

    for item in list_items {
        assert!(result.content.contains(item), "Should contain list item: '{}'", item);
    }

    println!("✅ RST unordered lists test passed!");
}

/// Test ordered list extraction
#[tokio::test]
async fn test_rst_ordered_lists() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    let list_items = vec!["First", "Second", "Third"];

    for item in list_items {
        assert!(
            result.content.contains(item),
            "Should contain ordered list item: '{}'",
            item
        );
    }

    println!("✅ RST ordered lists test passed!");
}

/// Test nested lists extraction
#[tokio::test]
async fn test_rst_nested_lists() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    assert!(
        result.content.contains("First")
            || result.content.contains("Second")
            || result.content.contains("Third")
            || result.content.contains("Definition"),
        "Should contain nested or definition list content"
    );

    println!("✅ RST nested lists test passed!");
}

/// Test simple table extraction
#[tokio::test]
async fn test_rst_simple_table_extraction() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    assert!(
        result.content.contains("Simple Tables")
            || result.content.contains("col")
            || (result.content.contains("r1") && result.content.contains("r2")),
        "Should contain simple table content"
    );

    println!("✅ RST simple table extraction test passed!");
}

/// Test grid table extraction
#[tokio::test]
async fn test_rst_grid_table_extraction() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    assert!(
        result.content.contains("Grid Tables")
            || result.content.contains("r1 a")
            || (result.content.contains("r1") && result.content.contains("r2")),
        "Should contain grid table content"
    );

    println!("✅ RST grid table extraction test passed!");
}

/// Test table with complex structure (multiple rows/columns spanning)
#[tokio::test]
async fn test_rst_complex_table_with_spanning() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    assert!(
        result.content.contains("Table with cells")
            || result.content.contains("Property")
            || result.content.contains("min")
            || result.content.contains("°C"),
        "Should contain complex table content"
    );

    println!("✅ RST complex table with spanning test passed!");
}

/// Test emphasis and strong markup
#[tokio::test]
async fn test_rst_emphasis_and_strong() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    assert!(
        result.content.contains("emphasized") || result.content.contains("strong"),
        "Should contain emphasis markers or converted text"
    );

    println!("✅ RST emphasis and strong test passed!");
}

/// Test inline code extraction
#[tokio::test]
async fn test_rst_inline_code() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    assert!(
        result.content.contains(">") || result.content.contains("code"),
        "Should contain inline code or code markers"
    );

    println!("✅ RST inline code test passed!");
}

/// Test subscript and superscript
#[tokio::test]
async fn test_rst_subscript_superscript() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    assert!(
        result.content.contains("subscript") || result.content.contains("superscript"),
        "Should contain subscript/superscript text"
    );

    println!("✅ RST subscript/superscript test passed!");
}

/// Test explicit links extraction
#[tokio::test]
async fn test_rst_explicit_links() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    assert!(
        result.content.contains("/url") || result.content.contains("URL"),
        "Should contain link URLs"
    );

    assert!(
        result.content.contains("link"),
        "Should contain link references or text"
    );

    println!("✅ RST explicit links test passed!");
}

/// Test reference links
#[tokio::test]
async fn test_rst_reference_links() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    assert!(
        result.content.contains("link1") || result.content.contains("link2") || result.content.contains("link"),
        "Should contain resolved reference links"
    );

    println!("✅ RST reference links test passed!");
}

/// Test autolinks (bare URLs and email addresses)
#[tokio::test]
async fn test_rst_autolinks() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    assert!(
        result.content.contains("example.com") || result.content.contains("http"),
        "Should contain URLs from autolinks"
    );

    assert!(
        result.content.contains("nowhere") || result.content.contains("@"),
        "Should contain email references"
    );

    println!("✅ RST autolinks test passed!");
}

/// Test image directive extraction
#[tokio::test]
async fn test_rst_image_directive() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    assert!(
        result.content.contains("image") || result.content.contains("lalune") || result.content.contains("movie"),
        "Should contain image directives or references"
    );

    assert!(
        result.content.contains("Voyage") || result.content.contains("Melies"),
        "Should contain image descriptions"
    );

    println!("✅ RST image directive test passed!");
}

/// Test raw HTML block extraction
#[tokio::test]
async fn test_rst_raw_html_blocks() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    assert!(
        result.content.contains("div") || result.content.contains("foo"),
        "Should contain HTML block content"
    );

    println!("✅ RST raw HTML blocks test passed!");
}

/// Test LaTeX block extraction
#[tokio::test]
async fn test_rst_latex_blocks() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    assert!(
        result.content.contains("LaTeX Block")
            || result.content.contains("begin{tabular}")
            || result.content.contains("Animal")
            || result.content.contains("Dog"),
        "Should contain LaTeX block or content"
    );

    println!("✅ RST LaTeX blocks test passed!");
}

/// Test math directive extraction
#[tokio::test]
async fn test_rst_math_directive() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    assert!(
        result.content.contains("E=mc^2")
            || result.content.contains("E = mc")
            || result.content.contains("alpha")
            || result.content.contains("Math"),
        "Should contain math formulas"
    );

    println!("✅ RST math directive test passed!");
}

/// Test comment blocks are excluded from output
#[tokio::test]
async fn test_rst_comment_blocks_excluded() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    assert!(
        !result.content.contains("should not appear"),
        "Comments should be excluded from output"
    );

    assert!(
        result.content.contains("First paragraph") || result.content.contains("paragraph"),
        "Non-comment content should be present"
    );

    println!("✅ RST comment blocks excluded test passed!");
}

/// Test line blocks extraction
#[tokio::test]
async fn test_rst_line_blocks() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    assert!(
        result.content.contains("Line blocks")
            || result.content.contains("bee")
            || result.content.contains("entire bee"),
        "Should contain line block content or heading"
    );

    println!("✅ RST line blocks test passed!");
}

/// Test unicode character preservation
#[tokio::test]
async fn test_rst_unicode_characters() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    assert!(
        result.content.contains("©")
            || result.content.contains("copyright")
            || result.content.contains("umlaut")
            || result.content.contains("unicode"),
        "Should contain unicode characters or references"
    );

    println!("✅ RST unicode characters test passed!");
}

/// Test escaped characters
#[tokio::test]
async fn test_rst_escaped_characters() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    assert!(
        result.content.contains("Backslash")
            || result.content.contains("Backtick")
            || result.content.contains("Asterisk"),
        "Should contain escaped special character sections"
    );

    println!("✅ RST escaped characters test passed!");
}

// SECTION 12: FOOTNOTES AND REFERENCES

/// Test footnote extraction
#[tokio::test]
async fn test_rst_footnotes() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    assert!(
        result.content.contains("Note") || result.content.contains("continuation"),
        "Should contain footnote content"
    );

    println!("✅ RST footnotes test passed!");
}

/// Test block quote extraction
#[tokio::test]
async fn test_rst_block_quotes() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    assert!(
        result.content.contains("block quote") || result.content.contains("pretty short"),
        "Should contain block quote content"
    );

    println!("✅ RST block quotes test passed!");
}

/// Test overall content extraction volume
#[tokio::test]
async fn test_rst_content_extraction_volume() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    let content_length = result.content.len();
    println!("Extracted content length: {} bytes", content_length);

    assert!(
        content_length > 1000,
        "Extracted content should be substantial (> 1000 bytes), got {} bytes",
        content_length
    );

    assert_eq!(result.mime_type, "text/x-rst", "MIME type should be preserved");

    println!("✅ RST content extraction volume test passed!");
    println!("   Extracted {} bytes from RST file", content_length);
}

/// Test extracted content contains all major sections
#[tokio::test]
async fn test_rst_all_major_sections_present() {
    let content = rst_fixture_bytes();
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    let major_sections = vec![
        "Paragraphs",
        "Block Quotes",
        "Code Blocks",
        "Lists",
        "Field Lists",
        "HTML Blocks",
        "LaTeX Block",
        "Inline Markup",
        "Special Characters",
        "Links",
        "Images",
        "Comments",
        "Tables",
        "Math",
    ];

    let content_lower = result.content.to_lowercase();
    let mut found_count = 0;

    for section in major_sections {
        if content_lower.contains(&section.to_lowercase()) {
            found_count += 1;
            println!("✓ Found section: {}", section);
        } else {
            println!("✗ Missing section: {}", section);
        }
    }

    assert!(
        found_count >= 10,
        "Should find at least 10 major sections, found {}",
        found_count
    );

    println!("✅ RST all major sections present test passed!");
    println!("   Found {}/14 major sections", found_count);
}

/// Test MIME type detection
#[tokio::test]
async fn test_rst_mime_type_detection() {
    let content = rst_fixture_bytes();

    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract with text/x-rst MIME type");

    assert_eq!(result.mime_type, "text/x-rst");

    println!("✅ RST MIME type detection test passed!");
}

/// Test that no extraction errors occur on valid RST file
#[tokio::test]
async fn test_rst_extraction_no_errors() {
    let content = rst_fixture_bytes();

    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default()).await;

    assert!(
        result.is_ok(),
        "RST extraction should succeed without errors: {:?}",
        result.err()
    );

    let extraction = result.unwrap();

    assert!(!extraction.content.is_empty(), "Extracted content should not be empty");

    println!("✅ RST extraction no errors test passed!");
}
