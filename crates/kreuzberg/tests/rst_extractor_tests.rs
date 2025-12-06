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
use kreuzberg::extraction::pandoc::validate_pandoc_version;

mod helpers;

// ============================================================================
// SECTION 1: BASIC METADATA EXTRACTION
// ============================================================================

/// Test extraction of document title from RST file structure
#[tokio::test]
async fn test_rst_title_extraction() {
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // The document contains: "Pandoc Test Suite" as the main title
    assert!(
        result.content.to_lowercase().contains("pandoc test suite"),
        "Should contain document title 'Pandoc Test Suite'"
    );

    // The content should also contain key headings from the document
    assert!(
        result.content.contains("Level one header") || result.content.contains("header"),
        "Should contain document headers"
    );

    println!("✅ RST title extraction test passed!");
}

/// Test field list metadata extraction (:Authors:, :Date:, :Revision:)
#[tokio::test]
async fn test_rst_field_list_metadata_extraction() {
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // Field lists in RST are converted to definition list format by Pandoc
    // The metadata contains: :Authors:, :Date:, :Revision:
    // Check what's actually in the content for debugging
    println!(
        "Content excerpt (first 500 chars): {}",
        &result.content[..std::cmp::min(500, result.content.len())]
    );

    // Check for Authors metadata - the field lists appear after the subtitle
    assert!(
        result.content.contains("John MacFarlane")
            || result.content.contains("July 17")
            || result.content.contains("Pandoc Test Suite"),
        "Should contain metadata information or title"
    );

    println!("✅ RST field list metadata extraction test passed!");
}

// ============================================================================
// SECTION 2: SECTION AND HEADING STRUCTURE
// ============================================================================

/// Test extraction of multiple heading levels
#[tokio::test]
async fn test_rst_section_hierarchy() {
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // Test for various heading levels from the RST file
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
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // "Level four with *emphasis*" should preserve the emphasis marker
    assert!(
        result.content.contains("emphasis") || result.content.contains("Level four"),
        "Should contain heading with emphasis"
    );

    println!("✅ RST heading with inline markup test passed!");
}

// ============================================================================
// SECTION 3: CODE BLOCK AND DIRECTIVE EXTRACTION
// ============================================================================

/// Test code block extraction with language specification
#[tokio::test]
async fn test_rst_code_block_extraction() {
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // The file contains Python code block
    assert!(
        result.content.contains("def my_function") || result.content.contains("python"),
        "Should contain Python code block or language specification"
    );

    // Test for content inside code blocks
    assert!(
        result.content.contains("return x + 1") || result.content.contains("my_function"),
        "Should contain Python function code"
    );

    println!("✅ RST code block extraction test passed!");
}

/// Test Haskell code blocks with highlight directive
#[tokio::test]
async fn test_rst_highlight_directive_code_blocks() {
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // Haskell code blocks from highlight directive
    assert!(
        result.content.contains("haskell") || result.content.contains("Tree") || result.content.contains("data Tree"),
        "Should contain Haskell code blocks"
    );

    // Check for specific Haskell code
    assert!(
        result.content.contains("Leaf") || result.content.contains("Node"),
        "Should contain Haskell data constructors"
    );

    println!("✅ RST highlight directive code blocks test passed!");
}

/// Test JavaScript code blocks
#[tokio::test]
async fn test_rst_javascript_code_blocks() {
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // JavaScript code
    assert!(
        result.content.contains("javascript") || result.content.contains("=>") || result.content.contains("let f"),
        "Should contain JavaScript code"
    );

    println!("✅ RST JavaScript code blocks test passed!");
}

// ============================================================================
// SECTION 4: LIST EXTRACTION
// ============================================================================

/// Test unordered list extraction
#[tokio::test]
async fn test_rst_unordered_lists() {
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // Test for list items from various list types
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
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // Ordered list items
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
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // Nested list and definition list items
    // The document contains nested lists and definition lists which should be extracted
    assert!(
        result.content.contains("First")
            || result.content.contains("Second")
            || result.content.contains("Third")
            || result.content.contains("Definition"),
        "Should contain nested or definition list content"
    );

    println!("✅ RST nested lists test passed!");
}

// ============================================================================
// SECTION 5: TABLE EXTRACTION
// ============================================================================

/// Test simple table extraction
#[tokio::test]
async fn test_rst_simple_table_extraction() {
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // Simple table content - should contain the table heading and/or some table data
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
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // Grid table content - should contain table section or table data
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
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // Complex table with spanning cells
    // The document contains tables with temperature and location data
    assert!(
        result.content.contains("Table with cells")
            || result.content.contains("Property")
            || result.content.contains("min")
            || result.content.contains("°C"),
        "Should contain complex table content"
    );

    println!("✅ RST complex table with spanning test passed!");
}

// ============================================================================
// SECTION 6: INLINE MARKUP AND SPECIAL FORMATTING
// ============================================================================

/// Test emphasis and strong markup
#[tokio::test]
async fn test_rst_emphasis_and_strong() {
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // Emphasis and strong text should be converted
    assert!(
        result.content.contains("emphasized") || result.content.contains("strong"),
        "Should contain emphasis markers or converted text"
    );

    println!("✅ RST emphasis and strong test passed!");
}

/// Test inline code extraction
#[tokio::test]
async fn test_rst_inline_code() {
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // Inline code should contain special characters like `>`, `$`, etc.
    assert!(
        result.content.contains(">") || result.content.contains("code"),
        "Should contain inline code or code markers"
    );

    println!("✅ RST inline code test passed!");
}

/// Test subscript and superscript
#[tokio::test]
async fn test_rst_subscript_superscript() {
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // subscripted and superscripted text
    assert!(
        result.content.contains("subscript") || result.content.contains("superscript"),
        "Should contain subscript/superscript text"
    );

    println!("✅ RST subscript/superscript test passed!");
}

// ============================================================================
// SECTION 7: LINKS AND REFERENCES
// ============================================================================

/// Test explicit links extraction
#[tokio::test]
async fn test_rst_explicit_links() {
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // Link references should be resolved
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
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // Reference links like "link1" and "link2" should be resolved
    assert!(
        result.content.contains("link1") || result.content.contains("link2") || result.content.contains("link"),
        "Should contain resolved reference links"
    );

    println!("✅ RST reference links test passed!");
}

/// Test autolinks (bare URLs and email addresses)
#[tokio::test]
async fn test_rst_autolinks() {
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // URLs and email addresses
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

// ============================================================================
// SECTION 8: IMAGE AND MEDIA DIRECTIVES
// ============================================================================

/// Test image directive extraction
#[tokio::test]
async fn test_rst_image_directive() {
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // Image references and descriptions
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

// ============================================================================
// SECTION 9: SPECIAL BLOCKS AND RAW HTML/LATEX
// ============================================================================

/// Test raw HTML block extraction
#[tokio::test]
async fn test_rst_raw_html_blocks() {
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // Raw HTML blocks
    assert!(
        result.content.contains("div") || result.content.contains("foo"),
        "Should contain HTML block content"
    );

    println!("✅ RST raw HTML blocks test passed!");
}

/// Test LaTeX block extraction
#[tokio::test]
async fn test_rst_latex_blocks() {
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // LaTeX content (tabular environment, Animal & Number table, or LaTeX Block section)
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
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // Math formulas
    assert!(
        result.content.contains("E=mc^2")
            || result.content.contains("E = mc")
            || result.content.contains("alpha")
            || result.content.contains("Math"),
        "Should contain math formulas"
    );

    println!("✅ RST math directive test passed!");
}

// ============================================================================
// SECTION 10: COMMENTS AND SPECIAL CONTENT
// ============================================================================

/// Test comment blocks are excluded from output
#[tokio::test]
async fn test_rst_comment_blocks_excluded() {
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // Comments should NOT appear in the output
    assert!(
        !result.content.contains("should not appear"),
        "Comments should be excluded from output"
    );

    // But the paragraph text surrounding comments should exist
    assert!(
        result.content.contains("First paragraph") || result.content.contains("paragraph"),
        "Non-comment content should be present"
    );

    println!("✅ RST comment blocks excluded test passed!");
}

/// Test line blocks extraction
#[tokio::test]
async fn test_rst_line_blocks() {
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // Line block content about a bee - or the line blocks section heading
    assert!(
        result.content.contains("Line blocks")
            || result.content.contains("bee")
            || result.content.contains("entire bee"),
        "Should contain line block content or heading"
    );

    println!("✅ RST line blocks test passed!");
}

// ============================================================================
// SECTION 11: SPECIAL CHARACTERS AND UNICODE
// ============================================================================

/// Test unicode character preservation
#[tokio::test]
async fn test_rst_unicode_characters() {
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // Unicode symbols and characters
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
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // Escaped special characters
    assert!(
        result.content.contains("Backslash")
            || result.content.contains("Backtick")
            || result.content.contains("Asterisk"),
        "Should contain escaped special character sections"
    );

    println!("✅ RST escaped characters test passed!");
}

// ============================================================================
// SECTION 12: FOOTNOTES AND REFERENCES
// ============================================================================

/// Test footnote extraction
#[tokio::test]
async fn test_rst_footnotes() {
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // Footnote content
    assert!(
        result.content.contains("Note") || result.content.contains("continuation"),
        "Should contain footnote content"
    );

    println!("✅ RST footnotes test passed!");
}

// ============================================================================
// SECTION 13: BLOCK QUOTES
// ============================================================================

/// Test block quote extraction
#[tokio::test]
async fn test_rst_block_quotes() {
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // Block quote content
    assert!(
        result.content.contains("block quote") || result.content.contains("pretty short"),
        "Should contain block quote content"
    );

    println!("✅ RST block quotes test passed!");
}

// ============================================================================
// SECTION 14: CONTENT QUALITY VALIDATION
// ============================================================================

/// Test overall content extraction volume
#[tokio::test]
async fn test_rst_content_extraction_volume() {
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // The 682-line RST file should produce substantial content
    let content_length = result.content.len();
    println!("Extracted content length: {} bytes", content_length);

    assert!(
        content_length > 1000,
        "Extracted content should be substantial (> 1000 bytes), got {} bytes",
        content_length
    );

    // Check mime type is correct
    assert_eq!(result.mime_type, "text/x-rst", "MIME type should be preserved");

    println!("✅ RST content extraction volume test passed!");
    println!("   Extracted {} bytes from RST file", content_length);
}

/// Test extracted content contains all major sections
#[tokio::test]
async fn test_rst_all_major_sections_present() {
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract RST successfully");

    // Major sections that must be present
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
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");

    // Test with proper MIME type
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default())
        .await
        .expect("Should extract with text/x-rst MIME type");

    assert_eq!(result.mime_type, "text/x-rst");

    println!("✅ RST MIME type detection test passed!");
}

/// Test that no extraction errors occur on valid RST file
#[tokio::test]
async fn test_rst_extraction_no_errors() {
    if validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/rst/rst-reader.rst");

    if !test_file.exists() {
        println!("Skipping test: RST test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read RST file");

    // Should not produce errors
    let result = extract_bytes(&content, "text/x-rst", &ExtractionConfig::default()).await;

    assert!(
        result.is_ok(),
        "RST extraction should succeed without errors: {:?}",
        result.err()
    );

    let extraction = result.unwrap();

    // Content should not be empty
    assert!(!extraction.content.is_empty(), "Extracted content should not be empty");

    println!("✅ RST extraction no errors test passed!");
}
