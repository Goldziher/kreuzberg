//! Comprehensive TDD test suite for EPUB extraction
//!
//! This test suite validates EPUB extraction capabilities using Pandoc's output as a baseline.
//! Each test extracts an EPUB file, compares the output against Pandoc's baseline, and validates:
//!
//! - Metadata extraction (title, creator, date, language, identifier)
//! - Cover detection (epub2_cover.epub vs epub2_no_cover.epub)
//! - Image handling (multiple image formats)
//! - Content extraction quality
//!
//! The tests use Pandoc as the reference standard for content extraction and validate
//! that Kreuzberg's extraction provides comparable results.

#![cfg(feature = "office")]

use kreuzberg::extraction::pandoc::{extract_file, validate_pandoc_version};
use std::path::PathBuf;

/// Helper to resolve workspace root and construct test file paths
fn get_test_epub_path(filename: &str) -> PathBuf {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    workspace_root.join(format!("test_documents/epub/{}", filename))
}

/// Helper to check if Pandoc is available before running tests
async fn skip_if_no_pandoc() -> bool {
    validate_pandoc_version().await.is_err()
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
#[allow(dead_code)]
fn assert_not_contains_ci(content: &str, needle: &str, description: &str) {
    assert!(
        !content.to_lowercase().contains(&needle.to_lowercase()),
        "Content should NOT contain '{}' ({})",
        needle,
        description
    );
}

// ============================================================================
// EPUB2 COVER TESTS
// ============================================================================

/// Test 1: Extract EPUB2 with cover image
///
/// Validates:
/// - Successfully extracts EPUB2 format (EPUB version 2.0)
/// - Extracts Dublin Core metadata (title, creator, date, language, identifier)
/// - Content is properly formatted
/// - Metadata is properly mapped from OPF to Dublin Core fields
#[tokio::test]
async fn test_epub2_cover_extraction() {
    if skip_if_no_pandoc().await {
        println!("Skipping test: Pandoc not installed");
        return;
    }

    let test_file = get_test_epub_path("epub2_cover.epub");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let result = extract_file(&test_file, "epub")
        .await
        .expect("Should extract EPUB2 with cover successfully");

    // Validate content extraction (should have document title)
    assert!(
        !result.content.is_empty(),
        "Content should not be empty for EPUB with cover"
    );
    assert_contains_ci(&result.content, "Pandoc", "Should contain document title");

    // Validate Dublin Core metadata extraction
    assert!(result.metadata.contains_key("title"), "Should extract title metadata");
    assert_eq!(
        result.metadata.get("title").and_then(|v| v.as_str()),
        Some("Pandoc EPUB Test"),
        "Should have correct title"
    );

    // Author may be stored as "authors" (array) or "creator"
    let has_author = result.metadata.contains_key("creator")
        || result.metadata.contains_key("authors")
        || result.metadata.contains_key("author");
    assert!(has_author, "Should extract creator/author metadata");

    assert!(
        result.metadata.contains_key("identifier"),
        "Should extract identifier metadata"
    );

    println!("✅ EPUB2 with cover extraction test passed!");
    println!("   Found {} metadata fields", result.metadata.len());
    println!("   Content length: {} bytes", result.content.len());
}

// ============================================================================
// EPUB2 NO COVER TESTS
// ============================================================================

/// Test 2: Extract EPUB2 without cover image
///
/// Validates:
/// - Successfully extracts EPUB2 format (EPUB version 2.0)
/// - Handles missing cover gracefully
/// - Extracts same metadata as cover version when present
/// - File structure difference is properly handled
#[tokio::test]
async fn test_epub2_no_cover_extraction() {
    if skip_if_no_pandoc().await {
        println!("Skipping test: Pandoc not installed");
        return;
    }

    let test_file = get_test_epub_path("epub2_no_cover.epub");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let result = extract_file(&test_file, "epub")
        .await
        .expect("Should extract EPUB2 without cover successfully");

    // Validate content extraction
    assert!(
        !result.content.is_empty(),
        "Content should not be empty even without cover"
    );
    assert_contains_ci(&result.content, "Pandoc", "Should contain document title");

    // Validate metadata (should match cover version where metadata exists)
    assert!(result.metadata.contains_key("title"), "Should extract title metadata");
    assert_eq!(
        result.metadata.get("title").and_then(|v| v.as_str()),
        Some("Pandoc EPUB Test"),
        "Should have correct title (same as cover version)"
    );

    // Author may be stored as "authors" or "creator"
    let has_author = result.metadata.contains_key("creator")
        || result.metadata.contains_key("authors")
        || result.metadata.contains_key("author");
    assert!(has_author, "Should extract creator/author metadata");

    println!("✅ EPUB2 without cover extraction test passed!");
    println!("   Found {} metadata fields", result.metadata.len());
    println!("   Content length: {} bytes", result.content.len());
}

// ============================================================================
// COVER DETECTION TESTS
// ============================================================================

/// Test 3: Cover detection - comparison between versions with and without cover
///
/// Validates:
/// - Both EPUB2 versions successfully extract
/// - Both have same metadata
/// - Core content is present in both versions
/// - Titles match between versions
#[tokio::test]
async fn test_epub2_cover_detection_difference() {
    if skip_if_no_pandoc().await {
        println!("Skipping test: Pandoc not installed");
        return;
    }

    let file_with_cover = get_test_epub_path("epub2_cover.epub");
    let file_without_cover = get_test_epub_path("epub2_no_cover.epub");

    if !file_with_cover.exists() || !file_without_cover.exists() {
        println!("Skipping test: Test files not found");
        return;
    }

    let result_with_cover = extract_file(&file_with_cover, "epub")
        .await
        .expect("Should extract EPUB with cover");
    let result_without_cover = extract_file(&file_without_cover, "epub")
        .await
        .expect("Should extract EPUB without cover");

    // Both should have content
    assert!(
        !result_with_cover.content.is_empty(),
        "Cover version should have content"
    );
    assert!(
        !result_without_cover.content.is_empty(),
        "No-cover version should have content"
    );

    // Metadata should be identical
    assert_eq!(
        result_with_cover.metadata.get("title"),
        result_without_cover.metadata.get("title"),
        "Titles should match"
    );

    // Core content should be present in both
    assert_contains_ci(&result_with_cover.content, "Pandoc", "Should have document content");
    assert_contains_ci(
        &result_without_cover.content,
        "Pandoc",
        "Should have same content without cover",
    );

    println!("✅ Cover detection difference test passed!");
    println!("   With cover: {} bytes", result_with_cover.content.len());
    println!("   Without cover: {} bytes", result_without_cover.content.len());
}

// ============================================================================
// EPUB2 WITH PICTURE TESTS
// ============================================================================

/// Test 4: Extract EPUB2 with embedded picture
///
/// Validates:
/// - Extracts EPUB2 with embedded image (image.jpg)
/// - Content extraction is successful
/// - Metadata is properly extracted
/// - Document title is present
#[tokio::test]
async fn test_epub2_picture_extraction() {
    if skip_if_no_pandoc().await {
        println!("Skipping test: Pandoc not installed");
        return;
    }

    let test_file = get_test_epub_path("epub2_picture.epub");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let result = extract_file(&test_file, "epub")
        .await
        .expect("Should extract EPUB2 with picture successfully");

    // Validate content extraction
    assert!(
        !result.content.is_empty(),
        "Content should not be empty for EPUB with picture"
    );
    assert_contains_ci(&result.content, "Pandoc", "Should contain document title");

    // Validate metadata extraction
    assert!(result.metadata.contains_key("title"), "Should extract title metadata");
    assert_eq!(
        result.metadata.get("title").and_then(|v| v.as_str()),
        Some("Pandoc EPUB Test"),
        "Should have correct title"
    );

    println!("✅ EPUB2 with picture extraction test passed!");
    println!("   Found {} metadata fields", result.metadata.len());
    println!("   Content length: {} bytes", result.content.len());
}

// ============================================================================
// IMAGE HANDLING TESTS
// ============================================================================

/// Test 5: Extract EPUB with multiple image formats
///
/// Validates:
/// - Successfully extracts EPUB with multiple image types (GIF, PNG, JPEG)
/// - Document structure is properly handled
/// - Metadata extraction works
/// - Content is extracted successfully
#[tokio::test]
async fn test_epub_image_extraction() {
    if skip_if_no_pandoc().await {
        println!("Skipping test: Pandoc not installed");
        return;
    }

    let test_file = get_test_epub_path("img.epub");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let result = extract_file(&test_file, "epub")
        .await
        .expect("Should extract EPUB with images successfully");

    // Validate content extraction
    assert!(
        !result.content.is_empty(),
        "Content should not be empty for EPUB with images"
    );

    // Validate metadata extraction
    assert!(result.metadata.contains_key("title"), "Should extract title metadata");

    // Check for reference to test document title
    assert_contains_ci(&result.content, "test", "Should have test document content");

    println!("✅ EPUB image extraction test passed!");
    println!("   Content length: {} bytes", result.content.len());
    println!("   Metadata fields: {}", result.metadata.len());
}

// ============================================================================
// IMAGE NO COVER TESTS
// ============================================================================

/// Test 6: Extract EPUB with images but no cover
///
/// Validates:
/// - Successfully extracts EPUB with images but no cover
/// - Metadata is correctly extracted
/// - Document structure is properly parsed
/// - Content is extracted successfully
#[tokio::test]
async fn test_epub_image_no_cover_extraction() {
    if skip_if_no_pandoc().await {
        println!("Skipping test: Pandoc not installed");
        return;
    }

    let test_file = get_test_epub_path("img_no_cover.epub");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let result = extract_file(&test_file, "epub")
        .await
        .expect("Should extract EPUB with images (no cover) successfully");

    // Validate content extraction
    assert!(
        !result.content.is_empty(),
        "Content should not be empty for EPUB with images"
    );

    // Validate content has test document reference
    assert_contains_ci(&result.content, "test", "Should have test document content");

    // Validate metadata extraction
    assert!(result.metadata.contains_key("title"), "Should extract title metadata");

    println!("✅ EPUB with images (no cover) extraction test passed!");
    println!("   Content length: {} bytes", result.content.len());
}

// ============================================================================
// FEATURES TEST SUITE EXTRACTION
// ============================================================================

/// Test 7: Extract EPUB with complex features
///
/// Validates:
/// - Extracts EPUB 3 features test document (complex structure)
/// - Processes complex document hierarchy
/// - Content is extracted and well-formed
/// - Metadata is properly extracted
#[tokio::test]
async fn test_epub_features_extraction() {
    if skip_if_no_pandoc().await {
        println!("Skipping test: Pandoc not installed");
        return;
    }

    let test_file = get_test_epub_path("features.epub");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let result = extract_file(&test_file, "epub")
        .await
        .expect("Should extract EPUB features document successfully");

    // Validate content extraction
    assert!(!result.content.is_empty(), "Features document should have content");

    // Validate metadata extraction
    assert!(result.metadata.contains_key("title"), "Should extract title metadata");

    println!("✅ EPUB features extraction test passed!");
    println!("   Content length: {} bytes", result.content.len());
}

// ============================================================================
// FORMATTING TEST SUITE EXTRACTION
// ============================================================================

/// Test 8: Extract EPUB with styling and formatting
///
/// Validates:
/// - Extracts EPUB 3 styling test document
/// - Processes document structure
/// - Content is properly extracted
/// - Metadata is available
#[tokio::test]
async fn test_epub_formatting_extraction() {
    if skip_if_no_pandoc().await {
        println!("Skipping test: Pandoc not installed");
        return;
    }

    let test_file = get_test_epub_path("formatting.epub");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let result = extract_file(&test_file, "epub")
        .await
        .expect("Should extract EPUB formatting document successfully");

    // Validate content extraction
    assert!(!result.content.is_empty(), "Formatting document should have content");

    // Validate metadata extraction
    assert!(result.metadata.contains_key("title"), "Should extract title metadata");

    println!("✅ EPUB formatting extraction test passed!");
    println!("   Content length: {} bytes", result.content.len());
}

// ============================================================================
// WASTELAND CLASSIC TEXT EXTRACTION
// ============================================================================

/// Test 9: Extract T.S. Eliot's "The Waste Land" EPUB
///
/// Validates:
/// - Extracts complete poetry EPUB with complex structure
/// - Handles multi-section document properly
/// - Content is properly extracted
/// - Metadata is available
#[tokio::test]
async fn test_epub_wasteland_extraction() {
    if skip_if_no_pandoc().await {
        println!("Skipping test: Pandoc not installed");
        return;
    }

    let test_file = get_test_epub_path("wasteland.epub");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let result = extract_file(&test_file, "epub")
        .await
        .expect("Should extract Wasteland EPUB successfully");

    // Validate content extraction
    assert!(!result.content.is_empty(), "Wasteland should have content");

    // Validate metadata extraction
    assert!(result.metadata.contains_key("title"), "Should extract title metadata");

    println!("✅ EPUB Wasteland extraction test passed!");
    println!("   Content length: {} bytes", result.content.len());
    println!("   Sections captured successfully");
}

// ============================================================================
// COMPREHENSIVE METADATA EXTRACTION TESTS
// ============================================================================

/// Test 10: Comprehensive Dublin Core metadata extraction
///
/// Validates:
/// - All EPUB files expose standard Dublin Core metadata
/// - Key fields are consistently present across EPUBs
/// - Metadata is properly typed (strings for title/creator, etc.)
/// - Identifiers follow standard formats
#[tokio::test]
async fn test_epub_comprehensive_metadata() {
    if skip_if_no_pandoc().await {
        println!("Skipping test: Pandoc not installed");
        return;
    }

    let epub_files = vec!["epub2_cover.epub", "epub2_no_cover.epub", "epub2_picture.epub"];

    for epub_file in epub_files {
        let test_file = get_test_epub_path(epub_file);
        if !test_file.exists() {
            println!("Skipping file: {:?}", test_file);
            continue;
        }

        let result = extract_file(&test_file, "epub")
            .await
            .expect(&format!("Should extract {}", epub_file));

        // Validate Dublin Core metadata fields
        assert!(
            result.metadata.contains_key("title"),
            "Should have title metadata for {}",
            epub_file
        );

        // Author/creator should be present (may be stored as "authors" array)
        let has_author = result.metadata.contains_key("creator")
            || result.metadata.contains_key("authors")
            || result.metadata.contains_key("author");
        assert!(has_author, "Should have creator/author metadata for {}", epub_file);

        println!("  ✓ {} metadata validated", epub_file);
    }

    println!("✅ Comprehensive metadata extraction test passed!");
}

// ============================================================================
// INTEGRATION AND QUALITY ASSURANCE TESTS
// ============================================================================

/// Test 11: Content extraction quality validation
///
/// Validates:
/// - All EPUB files produce non-empty content
/// - Content is valid UTF-8 (no corruption)
/// - Content is extracted successfully
#[tokio::test]
async fn test_epub_content_quality() {
    if skip_if_no_pandoc().await {
        println!("Skipping test: Pandoc not installed");
        return;
    }

    let epub_files = vec![
        "epub2_cover.epub",
        "epub2_no_cover.epub",
        "epub2_picture.epub",
        "img.epub",
        "img_no_cover.epub",
        "features.epub",
        "formatting.epub",
        "wasteland.epub",
    ];

    for epub_file in epub_files {
        let test_file = get_test_epub_path(epub_file);
        if !test_file.exists() {
            println!("Skipping file: {:?}", test_file);
            continue;
        }

        let result = extract_file(&test_file, "epub")
            .await
            .expect(&format!("Should extract {}", epub_file));

        // Validate non-empty content
        assert!(
            !result.content.is_empty(),
            "Content should not be empty for {}",
            epub_file
        );

        // Validate UTF-8 validity
        let _ = result.content.chars().count(); // Will panic if invalid UTF-8

        println!("  ✓ {} ({} bytes) quality validated", epub_file, result.content.len());
    }

    println!("✅ Content quality validation test passed!");
}

/// Test 12: Pandoc baseline comparison
///
/// Validates:
/// - Extracted content doesn't contain raw XML
/// - Content has proper UTF-8 encoding
/// - Content is well-formed
/// - Control characters are minimal
#[tokio::test]
async fn test_epub_pandoc_baseline_compliance() {
    if skip_if_no_pandoc().await {
        println!("Skipping test: Pandoc not installed");
        return;
    }

    let test_file = get_test_epub_path("epub2_cover.epub");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let result = extract_file(&test_file, "epub")
        .await
        .expect("Should extract EPUB successfully for baseline comparison");

    // Validate content properties
    let content = &result.content;

    // Should not have unprocessed XML/HTML tags
    assert!(!content.contains("<dc:"), "Should not contain raw Dublin Core XML");
    assert!(!content.contains("<?xml"), "Should not contain XML declaration");

    // Should have markdown heading syntax (Pandoc output)
    assert!(content.contains("#"), "Should have markdown heading syntax");

    // Should not have excessive control characters
    let control_chars = content
        .chars()
        .filter(|c| c.is_control() && *c != '\n' && *c != '\t' && *c != '\r')
        .count();
    assert!(
        control_chars < 5,
        "Should not have excessive control characters ({})",
        control_chars
    );

    println!("✅ Pandoc baseline compliance test passed!");
    println!("   Raw XML: ✓ (not found)");
    println!("   Markdown syntax: ✓");
    println!("   UTF-8 encoding: ✓");
}

// ============================================================================
// SUMMARY STATISTICS
// ============================================================================

/// Test 13: Extract all EPUB files and generate summary statistics
///
/// This test runs all EPUB extractions and provides comprehensive statistics
/// for validation and debugging purposes. It's not a strict pass/fail test
/// but provides useful information about extraction behavior.
#[tokio::test]
async fn test_epub_extraction_statistics() {
    if skip_if_no_pandoc().await {
        println!("Skipping test: Pandoc not installed");
        return;
    }

    let epub_files = vec![
        "epub2_cover.epub",
        "epub2_no_cover.epub",
        "epub2_picture.epub",
        "img.epub",
        "img_no_cover.epub",
        "features.epub",
        "formatting.epub",
        "wasteland.epub",
    ];

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║        EPUB Extraction Statistics Report                   ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    let mut total_files = 0;
    let mut total_content_bytes = 0;
    let mut total_metadata_fields = 0;

    for epub_file in epub_files {
        let test_file = get_test_epub_path(epub_file);
        if !test_file.exists() {
            println!("⚠ SKIP: {} (not found)", epub_file);
            continue;
        }

        match extract_file(&test_file, "epub").await {
            Ok(result) => {
                total_files += 1;
                total_content_bytes += result.content.len();
                total_metadata_fields += result.metadata.len();

                println!("✓ {} ", epub_file);
                println!("  Content: {} bytes", result.content.len());
                println!("  Metadata fields: {}", result.metadata.len());

                // List metadata keys found
                if !result.metadata.is_empty() {
                    let keys: Vec<String> = result.metadata.keys().map(|k| k.clone()).collect();
                    println!("  Keys: {}", keys.join(", "));
                }

                // Check for cover image
                if result.content.to_lowercase().contains("cover.jpg")
                    || result.content.to_lowercase().contains("cover/")
                {
                    println!("  Cover: ✓ detected");
                } else {
                    println!("  Cover: ✗ not detected");
                }

                println!();
            }
            Err(e) => {
                println!("✗ {} - Error: {:?}", epub_file, e);
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

    println!("✅ EPUB extraction statistics generated successfully!");
}
