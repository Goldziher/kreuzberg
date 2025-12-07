//! Comprehensive TDD test suite for EPUB extraction
//!
//! This test suite validates EPUB extraction capabilities covering:
//! - Basic content extraction (EPUB2 & EPUB3)
//! - Chapter/section navigation (testing complete document extraction)
//! - Structure preservation (headings, lists, hierarchy)
//! - Image handling (with/without covers)
//! - Cover detection and handling
//! - Complex document extraction (poetry, technical documents)
//! - Pandoc parity (content length baseline comparison)
//! - Metadata extraction (Dublin Core fields)
//! - Quality assurance (deterministic, no content loss)
//!
//! Current Status:
//! - Quality: Testing against Pandoc baselines
//! - Known Issue (historical): 99.84% content loss in early version
//! - Test Strategy: Validate all chapters extracted (not just first)
//!
//! Test Organization (13 integration tests):
//! 1. EPUB2 with cover extraction
//! 2. EPUB2 without cover extraction
//! 3. Cover detection difference
//! 4. EPUB2 with pictures extraction
//! 5. Images with covers extraction
//! 6. Images without covers extraction
//! 7. EPUB3 features extraction
//! 8. EPUB3 formatting extraction
//! 9. Wasteland (complex poetry) extraction
//! 10. Comprehensive metadata extraction
//! 11. Content quality validation
//! 12. Pandoc baseline compliance
//! 13. Extraction statistics report
//!
//! Success Criteria:
//! - All tests passing (100%)
//! - No content loss (non-empty extraction for all files)
//! - Pandoc parity: content length within 90-110% of baseline (some files have stricter checks)
//! - Deterministic extraction (same input = same output)
//! - Metadata properly extracted (Dublin Core fields)

// NOTE: This test file has been disabled as part of removing Pandoc dependencies (Part 1).
// The tests in this file relied on the Pandoc extract_file function which has been removed.
// Part 2 of the refactoring will rewrite these tests to use the native EpubExtractor.
// See crates/kreuzberg/tests/epub_native_extractor_tests.rs for native extractor tests.
//
// To re-enable these tests after rewriting them for native extraction, remove the cfg gate below.
#![cfg(all(feature = "office", feature = "disabled_pandoc_epub_tests"))]

use kreuzberg::extraction::pandoc::{extract_file, validate_pandoc_version};
use std::fs;
use std::path::PathBuf;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Helper to resolve workspace root and construct test file paths
fn get_test_epub_path(filename: &str) -> PathBuf {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    workspace_root.join(format!("test_documents/epub/{}", filename))
}

/// Helper to get path to Pandoc baseline for comparison
fn get_baseline_path(filename: &str) -> PathBuf {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    workspace_root.join(format!("test_documents/epub/{}", filename))
}

/// Helper to compare extracted content length with Pandoc baseline
/// Returns (extracted_len, baseline_len, ratio_percent)
fn compare_with_baseline(extracted: &str, baseline_filename: &str) -> (usize, usize, f64) {
    let baseline_path = get_baseline_path(baseline_filename);
    let baseline = fs::read_to_string(&baseline_path).expect(&format!("Failed to read baseline: {:?}", baseline_path));
    let extracted_len = extracted.trim().len();
    let baseline_len = baseline.trim().len();
    let ratio = if baseline_len > 0 {
        (extracted_len as f64 / baseline_len as f64) * 100.0
    } else {
        0.0
    };
    (extracted_len, baseline_len, ratio)
}

/// Helper to check if Pandoc is available before running tests
async fn skip_if_no_pandoc() -> bool {
    validate_pandoc_version().await.is_err()
}

/// Helper to validate that content contains expected text (case-insensitive)
fn assert_contains_ci(content: &str, needle: &str, description: &str) {
    assert!(
        content.to_lowercase().contains(&needle.to_lowercase()),
        "Content should contain '{}' ({}). Content: {}",
        needle,
        description,
        &content[..std::cmp::min(200, content.len())]
    );
}

/// Helper to validate content doesn't contain undesired text (case-insensitive)
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

// ============================================================================
// ENHANCED TDD TESTS - COMPREHENSIVE COVERAGE
// ============================================================================
// These tests focus on the critical bug: 99.84% content loss due to only
// extracting first chapter. Tests validate all chapters are processed.

/// Test 14: Basic EPUB2 content extraction (no cover version)
///
/// Critical Test: Validates non-zero content extraction from minimal EPUB
/// Addresses: Historical bug where 0 bytes were extracted (99.84% loss)
/// File: epub2_no_cover.epub (minimal EPUB2 document)
/// Expected: Extract meaningful content (at least 100+ bytes)
#[tokio::test]
async fn test_epub_basic_content_extraction_epub2_no_cover() {
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
        .expect("Should extract epub2_no_cover.epub successfully");

    // CRITICAL: Should extract substantial content (not 0 bytes)
    assert!(
        result.content.len() > 50,
        "CRITICAL: Should extract content from EPUB2, got {} bytes. Bug: 99.84% content loss?",
        result.content.len()
    );

    // Should contain the expected document title
    assert_contains_ci(&result.content, "Pandoc", "Should contain document title");

    println!(
        "✓ EPUB2 basic content extraction passed ({} bytes)",
        result.content.len()
    );
}

/// Test 15: All chapters extracted (not just first)
///
/// Critical Test: Validates CRITICAL BUG FIX - all chapters extracted
/// Known Bug: Early implementation only extracted first chapter (99.84% loss)
/// File: features.epub (EPUB3 with multiple chapters/sections)
/// Expected: Extract from ALL chapters (much more content than first chapter alone)
#[tokio::test]
async fn test_epub_all_chapters_extracted() {
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
        .expect("Should extract features.epub successfully");

    // CRITICAL: Should extract substantial content from ALL chapters
    // If only first chapter: ~500-1000 bytes
    // If all chapters: >5000 bytes
    assert!(
        result.content.len() > 1000,
        "CRITICAL BUG: Should extract from ALL chapters, got only {} bytes. \
         Indicates first-chapter-only extraction (99.84% loss bug)?",
        result.content.len()
    );

    // Should have multiple content indicators (multiple chapters)
    let test_count = result.content.matches("test").count() + result.content.matches("Test").count();
    assert!(
        test_count >= 2,
        "Should extract multiple test references from multiple chapters"
    );

    println!(
        "✓ All chapters extraction test passed ({} bytes, {} chapter indicators)",
        result.content.len(),
        test_count
    );
}

/// Test 16: Structure preservation - headings detected
///
/// Validates: Document structure hierarchy is preserved
/// File: formatting.epub (EPUB3 with styled content)
/// Expected: Extract heading indicators (markdown # symbols or heading text)
#[tokio::test]
async fn test_epub_structure_preservation_headings() {
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
        .expect("Should extract formatting.epub successfully");

    // Should have substantial content
    assert!(!result.content.is_empty(), "Content should not be empty");
    assert!(
        result.content.len() > 200,
        "Should extract formatted content with structure"
    );

    // Should preserve heading structure (markdown format or text indicators)
    let has_heading_markers =
        result.content.contains("#") || result.content.contains("---") || result.content.contains("===");

    assert!(
        has_heading_markers || result.content.len() > 1000,
        "Should preserve document structure (headings)"
    );

    println!("✓ Structure preservation test passed");
}

/// Test 17: Image handling - EPUB with images
///
/// Validates: Content extraction even when images present
/// File: img.epub (EPUB with multiple image formats)
/// Expected: Extract text content despite image files (not crash, not lose content)
#[tokio::test]
async fn test_epub_image_handling_with_images() {
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
        .expect("Should extract img.epub successfully");

    // CRITICAL: Should extract content even with images
    assert!(
        result.content.len() > 100,
        "Should extract text content from EPUB with images, got {} bytes",
        result.content.len()
    );

    // Should contain test-related content
    assert!(
        result.content.to_lowercase().contains("test")
            || result.content.to_lowercase().contains("image")
            || result.content.to_lowercase().contains("multimedia"),
        "Should extract content about images or tests"
    );

    println!(
        "✓ Image handling test passed ({} bytes extracted)",
        result.content.len()
    );
}

/// Test 18: Image handling - EPUB with images, no cover
///
/// Validates: Content extraction without cover overhead
/// File: img_no_cover.epub (EPUB with images but no cover page)
/// Expected: Extract text content efficiently without cover processing
#[tokio::test]
async fn test_epub_image_no_cover_handling() {
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
        .expect("Should extract img_no_cover.epub successfully");

    // Should have meaningful content
    assert!(
        result.content.len() > 100,
        "Should extract content from image EPUB without cover"
    );

    // Should contain test-related content
    assert!(
        result.content.to_lowercase().contains("test")
            || result.content.to_lowercase().contains("image")
            || result.content.to_lowercase().contains("required"),
        "Should extract test content about images"
    );

    println!(
        "✓ Image (no cover) handling test passed ({} bytes)",
        result.content.len()
    );
}

/// Test 19: Complex books - Wasteland poetry extraction
///
/// Validates: Handling of long, complex documents with literary structure
/// File: wasteland.epub (T.S. Eliot's "The Waste Land" - 25KB EPUB)
/// Expected: Extract substantial content (multi-section poetry)
#[tokio::test]
async fn test_epub_complex_book_wasteland() {
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
        .expect("Should extract wasteland.epub successfully");

    // Wasteland is a longer book - should extract substantial content
    assert!(
        result.content.len() > 2000,
        "Should extract substantial content from Wasteland, got {} bytes",
        result.content.len()
    );

    // Should contain key phrases from T.S. Eliot's poem
    assert!(
        result.content.contains("April")
            || result.content.contains("burial")
            || result.content.contains("waste")
            || result.content.contains("Land"),
        "Should extract Waste Land poem content"
    );

    println!("✓ Complex book extraction test passed ({} bytes)", result.content.len());
}

/// Test 20: Pandoc parity - features.epub baseline comparison
///
/// Validates: Content extraction matches Pandoc's output (90-110% tolerance)
/// File: features.epub with baseline
/// This is critical for ensuring EPUB extraction quality
#[tokio::test]
async fn test_epub_pandoc_parity_features() {
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
        .expect("Should extract features.epub successfully");

    let (extracted_len, baseline_len, ratio) = compare_with_baseline(&result.content, "features_pandoc_baseline.txt");

    assert!(
        ratio >= 90.0 && ratio <= 110.0,
        "FAIL: Content length {}% of Pandoc baseline. Expected 90-110%. \
         (Extracted: {} bytes, Baseline: {} bytes)",
        ratio as i32,
        extracted_len,
        baseline_len
    );

    println!("✓ Pandoc parity test (features.epub) passed - ratio: {:.1}%", ratio);
}

/// Test 21: Pandoc parity - formatting.epub baseline comparison
///
/// Validates: Styled EPUB extraction matches Pandoc output
/// File: formatting.epub with baseline
#[tokio::test]
async fn test_epub_pandoc_parity_formatting() {
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
        .expect("Should extract formatting.epub successfully");

    let (extracted_len, baseline_len, ratio) = compare_with_baseline(&result.content, "formatting_pandoc_baseline.txt");

    assert!(
        ratio >= 90.0 && ratio <= 110.0,
        "FAIL: Content length {}% of Pandoc baseline. Expected 90-110%. \
         (Extracted: {} bytes, Baseline: {} bytes)",
        ratio as i32,
        extracted_len,
        baseline_len
    );

    println!("✓ Pandoc parity test (formatting.epub) passed - ratio: {:.1}%", ratio);
}

/// Test 22: Pandoc parity - wasteland.epub baseline comparison
///
/// Validates: Complex poetry EPUB extraction quality
/// File: wasteland.epub with baseline
#[tokio::test]
async fn test_epub_pandoc_parity_wasteland() {
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
        .expect("Should extract wasteland.epub successfully");

    let (extracted_len, baseline_len, ratio) = compare_with_baseline(&result.content, "wasteland_pandoc_baseline.txt");

    assert!(
        ratio >= 90.0 && ratio <= 110.0,
        "FAIL: Content length {}% of Pandoc baseline. Expected 90-110%. \
         (Extracted: {} bytes, Baseline: {} bytes)",
        ratio as i32,
        extracted_len,
        baseline_len
    );

    println!("✓ Pandoc parity test (wasteland.epub) passed - ratio: {:.1}%", ratio);
}

/// Test 23: Deterministic extraction - same input produces same output
///
/// Validates: EPUB extraction is deterministic (no randomness/caching issues)
/// File: features.epub
/// Expected: Multiple extractions of same file produce identical results
#[tokio::test]
async fn test_epub_extraction_deterministic() {
    if skip_if_no_pandoc().await {
        println!("Skipping test: Pandoc not installed");
        return;
    }

    let test_file = get_test_epub_path("features.epub");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    // Extract same file twice
    let result1 = extract_file(&test_file, "epub")
        .await
        .expect("First extraction should succeed");

    let result2 = extract_file(&test_file, "epub")
        .await
        .expect("Second extraction should succeed");

    // Content should be identical
    assert_eq!(
        result1.content, result2.content,
        "FAIL: Extraction is not deterministic. Same input produced different outputs."
    );

    // Metadata should be identical
    assert_eq!(
        result1.metadata, result2.metadata,
        "FAIL: Metadata extraction is not deterministic."
    );

    println!("✓ Deterministic extraction test passed");
}

/// Test 24: No critical content loss across all EPUB files
///
/// Validates: None of the EPUB test files lose all their content
/// Critical: Tests the main bug (99.84% loss) is fixed
/// Files: All 8 EPUB test files
/// Expected: Each file extracts meaningful content
#[tokio::test]
async fn test_epub_no_critical_content_loss() {
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
            println!("⚠ Skipping {}: not found", epub_file);
            continue;
        }

        let result = extract_file(&test_file, "epub")
            .await
            .expect(&format!("Should extract {}", epub_file));

        // CRITICAL: Should extract meaningful content (not 0 bytes)
        assert!(
            result.content.len() > 20,
            "CRITICAL: {} extracted only {} bytes. Content loss bug?",
            epub_file,
            result.content.len()
        );

        println!("✓ {} - {} bytes extracted", epub_file, result.content.len());
    }

    println!("✅ All EPUB files extracted successfully - no critical content loss");
}

/// Test 25: UTF-8 and Unicode handling in EPUB content
///
/// Validates: EPUB extraction properly handles Unicode characters
/// Files: Multiple files may contain Unicode content
/// Expected: Extracted content is valid UTF-8, no corruption
#[tokio::test]
async fn test_epub_unicode_validity() {
    if skip_if_no_pandoc().await {
        println!("Skipping test: Pandoc not installed");
        return;
    }

    let epub_files = vec!["wasteland.epub", "features.epub", "formatting.epub"];

    for epub_file in epub_files {
        let test_file = get_test_epub_path(epub_file);
        if !test_file.exists() {
            println!("⚠ Skipping {}: not found", epub_file);
            continue;
        }

        let result = extract_file(&test_file, "epub")
            .await
            .expect(&format!("Should extract {}", epub_file));

        // Validate UTF-8 - this will panic if invalid
        let char_count = result.content.chars().count();
        assert!(char_count > 0, "Should have valid UTF-8 characters");

        // Should not have excessive control characters
        let control_chars = result
            .content
            .chars()
            .filter(|c| c.is_control() && *c != '\n' && *c != '\t' && *c != '\r')
            .count();

        assert!(
            control_chars < (char_count / 10),
            "Should not have excessive control characters ({}% of content)",
            (control_chars * 100) / char_count.max(1)
        );

        println!(
            "✓ {} - valid UTF-8 ({} chars, {} control)",
            epub_file, char_count, control_chars
        );
    }

    println!("✅ Unicode validity test passed");
}

// ============================================================================
// TEST SUMMARY AND DOCUMENTATION
// ============================================================================
//
// Test Suite Summary:
//
// **Total Tests**: 25 (13 original Pandoc-based + 12 enhanced TDD tests)
//
// **Test Categories**:
//
// 1. **Basic Content Extraction** (Test 1, 2, 14)
//    - EPUB2 with cover
//    - EPUB2 without cover
//    - EPUB2 minimal content
//
// 2. **Chapter Navigation** (Test 3, 15)
//    - Cover detection difference
//    - All chapters extracted (CRITICAL - tests 99.84% bug fix)
//
// 3. **Structure Preservation** (Test 7, 8, 16)
//    - EPUB3 features extraction
//    - EPUB3 formatting extraction
//    - Heading/structure preservation
//
// 4. **Image Handling** (Test 5, 6, 17, 18)
//    - EPUB with images
//    - EPUB without covers
//    - Multiple image formats
//    - Image EPUB without cover
//
// 5. **Cover Handling** (Test 1, 2, 3, 4)
//    - EPUB2 with cover
//    - EPUB2 without cover
//    - EPUB2 with picture
//    - Cover detection comparison
//
// 6. **Complex Books** (Test 9, 19)
//    - Wasteland (T.S. Eliot poetry)
//    - Large, multi-section documents
//
// 7. **Metadata Extraction** (Test 10)
//    - Dublin Core field extraction
//    - Creator/author extraction
//    - Title, identifier, other metadata
//
// 8. **Pandoc Parity** (Test 20, 21, 22)
//    - features.epub (90-110% baseline)
//    - formatting.epub (90-110% baseline)
//    - wasteland.epub (90-110% baseline)
//
// 9. **Quality Assurance** (Test 11, 12, 23, 24, 25)
//    - Content quality validation
//    - Pandoc baseline compliance
//    - Deterministic extraction
//    - No critical content loss
//    - UTF-8/Unicode validity
//
// 10. **Statistics & Reporting** (Test 13)
//     - Comprehensive extraction statistics
//     - Metadata field counts
//     - Content size distribution
//
// **Expected Results**:
// - ALL TESTS SHOULD PASS (100% pass rate)
// - No content loss (each file extracts >20 bytes minimum)
// - Deterministic behavior (same input = same output)
// - Pandoc parity within tolerance
//
// **Critical Bug Being Tested**:
// - Historical issue: 99.84% content loss due to first-chapter-only extraction
// - Test 15 (all_chapters_extracted) is the primary validation
// - Expected to PASS if bug was fixed in commit b74adad9
