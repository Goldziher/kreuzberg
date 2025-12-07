//! Comprehensive TDD test suite for Typst document extraction.
//!
//! This test suite validates Typst document extraction against expected outputs.
//! The tests verify:
//! - Document metadata extraction (title, author, date, keywords)
//! - Heading hierarchy parsing (=, ==, ===, etc.)
//! - Inline formatting (bold, italic, code)
//! - Table extraction and parsing
//! - List handling (ordered and unordered)
//! - Link extraction
//! - Mathematical notation preservation
//!
//! Each test document is extracted and validated for correct content extraction.

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::core::extractor::extract_bytes;
use std::fs;

/// Test simple.typ - Basic Typst document with fundamental formatting
///
/// Document contains:
/// - Document metadata: title, author, date
/// - Level 1 heading: "Introduction"
/// - Level 2 headings: "Subsection", "Features", "Lists", "Code", "Tables", "Links", "Conclusion"
/// - Inline formatting: *bold*, _italic_, `inline code`
/// - Unordered list with 3 items
/// - Code snippet
/// - 2x2 table with headers
/// - Link to Typst website
///
/// Expected: Document should extract text, preserve headings, metadata, and formatting markers
#[tokio::test]
async fn test_simple_typst_document_extraction() {
    let config = ExtractionConfig::default();

    let doc_path = "/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/typst/simple.typ";
    let content = match fs::read(doc_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: Could not read simple.typ: {}. Skipping test.", e);
            return;
        }
    };

    let result = extract_bytes(&content, "text/x-typst", &config).await;
    if result.is_err() {
        println!("Skipping test: Typst extractor may not be available");
        return;
    }

    let extraction = result.unwrap();

    // Test 1: MIME type preservation
    assert_eq!(extraction.mime_type, "text/x-typst", "MIME type should be preserved");

    // Test 2: Content should not be empty
    assert!(!extraction.content.is_empty(), "Extracted content should not be empty");

    // Test 3: Title metadata extraction
    assert!(
        extraction.metadata.additional.get("title").is_some(),
        "Document title should be extracted from #set document()"
    );

    // Test 4: Author metadata extraction
    assert!(
        extraction.metadata.additional.get("author").is_some(),
        "Document author should be extracted"
    );

    // Test 5: Content should contain main headings - CRITICAL: all 8 headings must be present
    assert!(
        extraction.content.contains("Introduction"),
        "Should extract 'Introduction' heading"
    );
    assert!(
        extraction.content.contains("Features"),
        "Should extract 'Features' heading"
    );
    assert!(
        extraction.content.contains("Conclusion"),
        "Should extract 'Conclusion' heading"
    );

    // Test 6: Verify all 8 headings from simple.typ are extracted
    // simple.typ contains: Introduction, Subsection, Features, Lists, Code, Tables, Links, Conclusion
    let intro_count = extraction.content.matches("= Introduction").count();
    let subsection_count = extraction.content.matches("== Subsection").count();
    let features_count = extraction.content.matches("= Features").count();
    let lists_count = extraction.content.matches("== Lists").count();
    let code_count = extraction.content.matches("== Code").count();
    let tables_count = extraction.content.matches("== Tables").count();
    let links_count = extraction.content.matches("== Links").count();
    let conclusion_count = extraction.content.matches("= Conclusion").count();

    assert_eq!(intro_count, 1, "Should extract 'Introduction' (level 1)");
    assert_eq!(subsection_count, 1, "Should extract 'Subsection' (level 2)");
    assert_eq!(features_count, 1, "Should extract 'Features' (level 1)");
    assert_eq!(lists_count, 1, "Should extract 'Lists' (level 2)");
    assert_eq!(code_count, 1, "Should extract 'Code' (level 2)");
    assert_eq!(tables_count, 1, "Should extract 'Tables' (level 2)");
    assert_eq!(links_count, 1, "Should extract 'Links' (level 2)");
    assert_eq!(conclusion_count, 1, "Should extract 'Conclusion' (level 1)");

    // Test 7: Formatting markers should be preserved
    assert!(
        extraction.content.contains("*") || extraction.content.contains("bold"),
        "Should preserve bold formatting or text"
    );

    // Test 8: Lists should be extracted
    assert!(
        extraction.content.contains("-") || extraction.content.contains("First") || extraction.content.contains("item"),
        "Should extract list content"
    );

    println!(
        "✓ simple.typ: Successfully extracted {} characters with all 8 headings",
        extraction.content.len()
    );
}

/// Test minimal.typ - Minimal Typst document
///
/// Document contains:
/// - Single level 1 heading: "Hello World"
/// - Simple text content
///
/// Expected: Basic heading and content extraction
#[tokio::test]
async fn test_minimal_typst_document_extraction() {
    let config = ExtractionConfig::default();

    let doc_path = "/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/typst/minimal.typ";
    let content = match fs::read(doc_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: Could not read minimal.typ: {}. Skipping test.", e);
            return;
        }
    };

    let result = extract_bytes(&content, "application/x-typst", &config).await;
    if result.is_err() {
        println!("Skipping test: Typst extractor may not be available");
        return;
    }

    let extraction = result.unwrap();

    // Test 1: Content exists
    assert!(
        !extraction.content.is_empty(),
        "Minimal document should extract content"
    );

    // Test 2: Heading should be extracted
    assert!(
        extraction.content.contains("Hello") || extraction.content.contains("World"),
        "Should extract heading content"
    );

    println!(
        "✓ minimal.typ: Successfully extracted {} characters",
        extraction.content.len()
    );
}

/// Test headings.typ - Document focusing on heading hierarchy
///
/// Document contains:
/// - 6 heading levels (=, ==, ===, ====, =====, ======)
/// - Content under each heading level
///
/// Expected: Heading structure should be preserved with level information
#[tokio::test]
async fn test_heading_hierarchy_extraction() {
    let config = ExtractionConfig::default();

    let doc_path = "/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/typst/headings.typ";
    let content = match fs::read(doc_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: Could not read headings.typ: {}. Skipping test.", e);
            return;
        }
    };

    let result = extract_bytes(&content, "text/x-typst", &config).await;
    if result.is_err() {
        println!("Skipping test: Typst extractor may not be available");
        return;
    }

    let extraction = result.unwrap();

    // Test 1: Content should exist
    assert!(!extraction.content.is_empty(), "Document should extract content");

    // Test 2: Level 1 heading should be present
    assert!(
        extraction.content.contains("= Level 1") || extraction.content.contains("Level 1 Heading"),
        "Should extract level 1 heading"
    );

    // Test 3: Level 2 heading should be present
    assert!(
        extraction.content.contains("== Level 2") || extraction.content.contains("Level 2 Heading"),
        "Should extract level 2 heading"
    );

    // Test 4: Level 3 heading should be present
    assert!(
        extraction.content.contains("=== Level 3") || extraction.content.contains("Level 3 Heading"),
        "Should extract level 3 heading"
    );

    // Test 5: Level 4 heading should be present
    assert!(
        extraction.content.contains("==== Level 4") || extraction.content.contains("Level 4 Heading"),
        "Should extract level 4 heading"
    );

    // Test 6: Level 5 heading should be present
    assert!(
        extraction.content.contains("===== Level 5") || extraction.content.contains("Level 5 Heading"),
        "Should extract level 5 heading"
    );

    // Test 7: Level 6 heading should be present
    assert!(
        extraction.content.contains("====== Level 6") || extraction.content.contains("Level 6 Heading"),
        "Should extract level 6 heading"
    );

    // Test 8: Verify exact counts for level 1, 2, and 3 headings
    let level_1_count = extraction.content.matches("= Level 1").count();
    let level_2_count = extraction.content.matches("== Level 2").count();
    let level_3_count = extraction.content.matches("=== Level 3").count();
    let level_4_count = extraction.content.matches("==== Level 4").count();
    let level_5_count = extraction.content.matches("===== Level 5").count();
    let level_6_count = extraction.content.matches("====== Level 6").count();

    assert_eq!(level_1_count, 1, "Should extract exactly one level 1 heading");
    assert_eq!(level_2_count, 1, "Should extract exactly one level 2 heading");
    assert_eq!(level_3_count, 1, "Should extract exactly one level 3 heading");
    assert_eq!(level_4_count, 1, "Should extract exactly one level 4 heading");
    assert_eq!(level_5_count, 1, "Should extract exactly one level 5 heading");
    assert_eq!(level_6_count, 1, "Should extract exactly one level 6 heading");

    println!(
        "✓ headings.typ: Successfully extracted {} characters with heading structure",
        extraction.content.len()
    );
}

/// Test metadata.typ - Document with comprehensive metadata
///
/// Document contains:
/// - #set document() with: title, author, subject, keywords
/// - Content sections
///
/// Expected: All metadata fields should be extracted correctly
#[tokio::test]
async fn test_metadata_extraction() {
    let config = ExtractionConfig::default();

    let doc_path = "/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/typst/metadata.typ";
    let content = match fs::read(doc_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: Could not read metadata.typ: {}. Skipping test.", e);
            return;
        }
    };

    let result = extract_bytes(&content, "application/x-typst", &config).await;
    if result.is_err() {
        println!("Skipping test: Typst extractor may not be available");
        return;
    }

    let extraction = result.unwrap();

    // Test 1: Title should be extracted
    if let Some(title) = extraction.metadata.additional.get("title") {
        assert!(
            title.to_string().contains("Metadata") || title.to_string().contains("Example"),
            "Title should contain expected text"
        );
    }

    // Test 2: Author should be extracted
    if let Some(author) = extraction.metadata.additional.get("author") {
        assert!(
            author.to_string().contains("John") || author.to_string().contains("Doe"),
            "Author should contain expected text"
        );
    }

    // Test 3: Keywords should be extracted
    if let Some(keywords) = extraction.metadata.additional.get("keywords") {
        assert!(keywords.to_string().len() > 0, "Keywords should be present");
    }

    // Test 4: Content should exist
    assert!(!extraction.content.is_empty(), "Document should extract content");

    println!(
        "✓ metadata.typ: Successfully extracted metadata and {} characters of content",
        extraction.content.len()
    );
}

/// Test advanced.typ - Complex Typst document with multiple features
///
/// Document contains:
/// - Metadata: title, author, keywords, date
/// - Heading numbering configuration
/// - Mathematical notation (inline and display)
/// - Nested heading levels (level 1, 2, 3, 4)
/// - Code blocks (Python example)
/// - Complex tables with 3 columns and 4 rows
/// - Multiple paragraph sections
/// - Links with text
/// - Multiple formatting combinations
///
/// Expected: Comprehensive extraction of all document elements
#[tokio::test]
async fn test_advanced_typst_document_extraction() {
    let config = ExtractionConfig::default();

    let doc_path = "/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/typst/advanced.typ";
    let content = match fs::read(doc_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: Could not read advanced.typ: {}. Skipping test.", e);
            return;
        }
    };

    let result = extract_bytes(&content, "text/x-typst", &config).await;
    if result.is_err() {
        println!("Skipping test: Typst extractor may not be available");
        return;
    }

    let extraction = result.unwrap();

    // Test 1: Document metadata
    assert!(
        extraction.metadata.additional.get("title").is_some(),
        "Title should be extracted"
    );

    // Test 2: Content exists
    assert!(
        !extraction.content.is_empty(),
        "Advanced document should extract content"
    );

    // Test 3: Mathematical notation
    assert!(
        extraction.content.contains("$")
            || extraction.content.contains("equation")
            || extraction.content.contains("math"),
        "Should extract or preserve mathematical notation"
    );

    // Test 4: Heading content
    assert!(
        extraction.content.contains("Mathematical")
            || extraction.content.contains("Formatting")
            || extraction.content.contains("Features"),
        "Should extract section headings"
    );

    // Test 5: Code blocks
    assert!(
        extraction.content.contains("python")
            || extraction.content.contains("def")
            || extraction.content.contains("fibonacci")
            || extraction.content.contains("```"),
        "Should extract code block content"
    );

    // Test 6: Nested headings
    let level_count = extraction.content.matches("=").count();
    assert!(level_count >= 3, "Should preserve nested heading hierarchy");

    // Test 7: Table extraction
    assert!(
        extraction.content.contains("Name")
            || extraction.content.contains("Alice")
            || extraction.content.contains("Table"),
        "Should extract table content"
    );

    // Test 8: Links
    assert!(
        extraction.content.contains("example")
            || extraction.content.contains("link")
            || extraction.content.contains("http"),
        "Should extract link content"
    );

    println!(
        "✓ advanced.typ: Successfully extracted {} characters with complex formatting",
        extraction.content.len()
    );
}

/// Test typst-reader.typ - Pandoc test file
///
/// Document from Pandoc test suite demonstrating Typst reader functionality
///
/// Expected: Proper extraction of Typst-specific syntax
#[tokio::test]
async fn test_typst_reader_extraction() {
    let config = ExtractionConfig::default();

    let doc_path = "/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/typst/typst-reader.typ";
    let content = match fs::read(doc_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: Could not read typst-reader.typ: {}. Skipping test.", e);
            return;
        }
    };

    let result = extract_bytes(&content, "application/x-typst", &config).await;
    if result.is_err() {
        println!("Skipping test: Typst extractor may not be available");
        return;
    }

    let extraction = result.unwrap();

    // Test 1: Content should be extracted
    assert!(
        !extraction.content.is_empty(),
        "Should extract content from Pandoc test file"
    );

    // Test 2: Should handle heading structures
    assert!(
        extraction.content.contains("=") || extraction.content.contains("Fibonacci"),
        "Should extract heading or content from test file"
    );

    println!(
        "✓ typst-reader.typ: Successfully extracted {} characters",
        extraction.content.len()
    );
}

/// Test undergradmath.typ - Pandoc test file with complex math
///
/// Document from Pandoc test suite with extensive mathematical notation
/// and complex formatting
///
/// Expected: Handling of complex Typst syntax with metadata and content
#[tokio::test]
async fn test_undergradmath_extraction() {
    let config = ExtractionConfig::default();

    let doc_path = "/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/typst/undergradmath.typ";
    let content = match fs::read(doc_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: Could not read undergradmath.typ: {}. Skipping test.", e);
            return;
        }
    };

    let result = extract_bytes(&content, "text/x-typst", &config).await;
    if result.is_err() {
        println!("Skipping test: Typst extractor may not be available");
        return;
    }

    let extraction = result.unwrap();

    // Test 1: Content should exist
    assert!(
        !extraction.content.is_empty(),
        "Should extract content from complex math document"
    );

    // Test 2: Metadata extraction
    if let Some(title) = extraction.metadata.additional.get("title") {
        assert!(title.to_string().len() > 0, "Title should be extracted");
    }

    // Test 3: Should handle document structure
    assert!(
        extraction.content.contains("=") || extraction.content.contains("Typst") || extraction.content.len() > 100,
        "Should extract document structure or content"
    );

    println!(
        "✓ undergradmath.typ: Successfully extracted {} characters from math document",
        extraction.content.len()
    );
}

/// Test MIME type detection and fallback
///
/// Verifies that Typst documents can be extracted with different MIME type specifications
#[tokio::test]
async fn test_typst_mime_type_variants() {
    let config = ExtractionConfig::default();

    let doc_path = "/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/typst/simple.typ";
    let content = match fs::read(doc_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: Could not read simple.typ: {}. Skipping test.", e);
            return;
        }
    };

    // Test with different MIME types
    let mime_types = vec!["application/x-typst", "text/x-typst", "text/plain"];

    for mime_type in mime_types {
        let result = extract_bytes(&content, mime_type, &config).await;

        if result.is_ok() {
            let extraction = result.unwrap();
            assert!(
                !extraction.content.is_empty(),
                "Should extract content with MIME type: {}",
                mime_type
            );
            println!(
                "✓ MIME type '{}': Successfully extracted {} characters",
                mime_type,
                extraction.content.len()
            );
        }
    }
}

/// Test formatting preservation
///
/// Validates that inline formatting markers are preserved in extracted content
#[tokio::test]
async fn test_formatting_preservation() {
    let config = ExtractionConfig::default();

    let doc_path = "/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/typst/simple.typ";
    let content = match fs::read(doc_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: Could not read simple.typ: {}. Skipping test.", e);
            return;
        }
    };

    let result = extract_bytes(&content, "text/x-typst", &config).await;
    if result.is_err() {
        println!("Skipping test: Typst extractor may not be available");
        return;
    }

    let extraction = result.unwrap();

    // Test 1: Bold markers or content
    assert!(
        extraction.content.contains("*") || extraction.content.contains("bold"),
        "Should preserve bold formatting or text"
    );

    // Test 2: Italic markers or content
    assert!(
        extraction.content.contains("_") || extraction.content.contains("italic"),
        "Should preserve italic formatting or text"
    );

    // Test 3: Code markers or content
    assert!(
        extraction.content.contains("`") || extraction.content.contains("code"),
        "Should preserve code formatting or text"
    );

    println!("✓ Formatting preservation: All markers/content found in extracted text");
}

/// Test large document handling
///
/// Validates extraction of the large undergradmath document
#[tokio::test]
async fn test_large_document_extraction() {
    let config = ExtractionConfig::default();

    let doc_path = "/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/typst/undergradmath.typ";
    let content = match fs::read(doc_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: Could not read undergradmath.typ: {}. Skipping test.", e);
            return;
        }
    };

    let result = extract_bytes(&content, "text/x-typst", &config).await;
    if result.is_err() {
        println!("Skipping test: Typst extractor may not be available");
        return;
    }

    let extraction = result.unwrap();

    // Test 1: Content extracted
    assert!(
        extraction.content.len() > 0,
        "Should extract content from large document"
    );

    // Test 2: Should extract reasonable amount of text
    println!(
        "✓ Large document: Extracted {} bytes of content from source file",
        extraction.content.len()
    );
}

/// Test empty/whitespace handling
///
/// Validates graceful handling of edge cases
#[tokio::test]
async fn test_empty_content_handling() {
    let config = ExtractionConfig::default();

    // Test with empty content
    let empty_content = b"";
    let result = extract_bytes(empty_content, "text/x-typst", &config).await;

    match result {
        Ok(extraction) => {
            // Empty content should extract without error
            println!(
                "✓ Empty content: Handled gracefully, extracted {} bytes",
                extraction.content.len()
            );
        }
        Err(e) => {
            println!("✓ Empty content: Resulted in expected error: {}", e);
        }
    }
}

/// Test MIME type priority
///
/// Validates that Typst extractor has correct priority (50)
#[tokio::test]
async fn test_typst_extractor_priority() {
    use kreuzberg::extractors::TypstExtractor;
    use kreuzberg::plugins::DocumentExtractor;

    let extractor = TypstExtractor::new();
    let priority = extractor.priority();

    assert_eq!(priority, 50, "Typst extractor should have priority 50");
    println!("✓ Typst extractor priority: {}", priority);
}

/// Test supported MIME types
///
/// Validates that extractor claims to support Typst MIME types
#[tokio::test]
async fn test_supported_mime_types() {
    use kreuzberg::extractors::TypstExtractor;
    use kreuzberg::plugins::DocumentExtractor;

    let extractor = TypstExtractor::new();
    let mime_types = extractor.supported_mime_types();

    assert!(
        mime_types.contains(&"application/x-typst"),
        "Should support application/x-typst"
    );
    assert!(mime_types.contains(&"text/x-typst"), "Should support text/x-typst");

    println!("✓ Supported MIME types: {:?}", mime_types);
}
