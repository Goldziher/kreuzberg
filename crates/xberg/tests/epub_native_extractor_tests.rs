//! Integration tests for the native EPUB extractor
//!
//! These tests validate the native Rust EPUB extractor (EpubExtractor)
//! which uses zip + roxmltree + html-to-markdown-rs (permissive licenses).
//!
//! This test suite verifies the fix for the two-pass OPF parsing bug that
//! caused 99.84% content loss due to single-pass manifest/spine resolution.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test/bench binaries print by design; org logging policy exempts tests
#![cfg(feature = "office")]

use std::path::PathBuf;
use xberg::ExtractInput;
use xberg::core::config::{ExtractionConfig, OutputFormat};
use xberg::extractors::EpubExtractor;
use xberg::plugins::DocumentExtractor;

/// Helper to resolve workspace root and construct test file paths
fn get_test_epub_path(filename: &str) -> PathBuf {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Operation failed")
        .parent()
        .expect("Operation failed");
    workspace_root.join(format!("test_documents/epub/{}", filename))
}

/// Test 1: Basic EPUB extraction - wasteland.epub
///
/// Validates:
/// - Two-pass OPF parsing works correctly
/// - Manifest is fully populated before spine resolution
/// - Content is extracted successfully (>2000 bytes expected)
/// - Metadata is extracted correctly
#[tokio::test]
async fn test_native_epub_wasteland_extraction() {
    let test_file = get_test_epub_path("wasteland.epub");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let bytes = std::fs::read(&test_file).expect("Failed to read wasteland.epub");
    let extractor = EpubExtractor;
    let config = ExtractionConfig::default();
    let input = ExtractInput::from_bytes(bytes, "application/epub+zip", None);
    let result = extractor
        .extract(input, &config)
        .await
        .expect("Should extract wasteland.epub successfully");

    assert!(
        result.content.len() > 2000,
        "Should extract substantial content from Wasteland, got {} bytes",
        result.content.len()
    );

    assert!(result.metadata.title.is_some(), "Should extract title metadata");
    assert_eq!(
        result.metadata.title.as_deref(),
        Some("The Waste Land"),
        "Should have correct title"
    );

    assert!(result.metadata.authors.is_some(), "Should extract creator metadata");

    assert!(
        result.content.contains("April") || result.content.contains("cruellest"),
        "Should contain key phrases from The Waste Land"
    );

    println!("✅ Wasteland extraction test passed ({} bytes)", result.content.len());
}

/// Test 2: EPUB with images - img.epub
///
/// Validates:
/// - EPUB with embedded images extracts successfully
/// - Text content is extracted (images are in manifest but not in content)
/// - Metadata is extracted
#[tokio::test]
async fn test_native_epub_images_extraction() {
    let test_file = get_test_epub_path("img.epub");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let bytes = std::fs::read(&test_file).expect("Failed to read img.epub");
    let extractor = EpubExtractor;
    let config = ExtractionConfig::default();
    let input = ExtractInput::from_bytes(bytes, "application/epub+zip", None);
    let result = extractor
        .extract(input, &config)
        .await
        .expect("Should extract img.epub successfully");

    assert!(
        result.content.len() > 50,
        "Should extract text content from EPUB with images, got {} bytes",
        result.content.len()
    );

    assert!(result.metadata.title.is_some(), "Should extract title metadata");

    println!("✅ Images EPUB extraction test passed ({} bytes)", result.content.len());
}

/// Test 3: Features EPUB - features.epub
///
/// Validates:
/// - Complex EPUB3 features document extracts successfully
/// - Multiple chapters/sections are extracted (not just first)
/// - Substantial content is present (>1000 bytes)
#[tokio::test]
async fn test_native_epub_features_extraction() {
    let test_file = get_test_epub_path("features.epub");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let bytes = std::fs::read(&test_file).expect("Failed to read features.epub");
    let extractor = EpubExtractor;
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    let input = ExtractInput::from_bytes(bytes.clone(), "application/epub+zip", None);
    let result = extractor
        .extract(input, &config)
        .await
        .expect("Should extract features.epub successfully");

    assert!(
        result.content.len() > 1000,
        "CRITICAL: Should extract from ALL chapters, got only {} bytes. \
         This indicates the two-pass bug is not fixed!",
        result.content.len()
    );
    assert_eq!(
        result.content.lines().filter(|line| line.trim() == "PASS").count(),
        1,
        "epub:switch should select its default branch"
    );
    assert_eq!(
        result.content.lines().filter(|line| line.trim() == "FAIL").count(),
        0,
        "epub:switch should omit unsupported and fallback branches"
    );
    assert!(
        result.content.lines().any(|line| line.trim() == "2 \u{2061} x + y - z"),
        "Markdown output should retain the selected MathML equation, got:\n{}",
        result.content
    );

    let plain_config = ExtractionConfig {
        output_format: OutputFormat::Plain,
        ..Default::default()
    };
    let plain_input = ExtractInput::from_bytes(bytes, "application/epub+zip", None);
    let plain_result = extractor
        .extract(plain_input, &plain_config)
        .await
        .expect("Should extract plain features.epub successfully");
    assert_eq!(
        plain_result
            .content
            .lines()
            .filter(|line| line.trim() == "PASS")
            .count(),
        1,
        "Plain output should retain the default for an unsupported namespace"
    );
    assert_eq!(
        plain_result
            .content
            .lines()
            .filter(|line| line.trim() == "FAIL")
            .count(),
        1,
        "Plain output should retain the readable fallback when MathML is unsupported"
    );

    println!(
        "✅ Features EPUB extraction test passed ({} bytes)",
        result.content.len()
    );
}

/// Test 4: EPUB2 with cover - epub2_cover.epub
///
/// Validates:
/// - EPUB2 format is supported
/// - Cover handling works correctly
/// - Content and metadata extracted
#[tokio::test]
async fn test_native_epub2_cover_extraction() {
    let test_file = get_test_epub_path("epub2_cover.epub");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let bytes = std::fs::read(&test_file).expect("Failed to read epub2_cover.epub");
    let extractor = EpubExtractor;
    let config = ExtractionConfig::default();
    let input = ExtractInput::from_bytes(bytes, "application/epub+zip", None);
    let result = extractor
        .extract(input, &config)
        .await
        .expect("Should extract epub2_cover.epub successfully");

    assert!(
        result.content.len() > 10,
        "Should extract content from EPUB2 with cover, got {} bytes",
        result.content.len()
    );

    assert_eq!(
        result.metadata.title.as_deref(),
        Some("Pandoc EPUB Test"),
        "Should have correct title"
    );

    println!("✅ EPUB2 cover extraction test passed ({} bytes)", result.content.len());
}

/// Test 5: Deterministic extraction
///
/// Validates:
/// - Same input produces same output (no randomness)
/// - Extraction is stable and reproducible
#[tokio::test]
async fn test_native_epub_deterministic_extraction() {
    let test_file = get_test_epub_path("features.epub");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let bytes = std::fs::read(&test_file).expect("Failed to read features.epub");
    let extractor = EpubExtractor;
    let config = ExtractionConfig::default();

    let input1 = ExtractInput::from_bytes(bytes.clone(), "application/epub+zip", None);
    let result1 = extractor
        .extract(input1, &config)
        .await
        .expect("First extraction should succeed");

    let input2 = ExtractInput::from_bytes(bytes, "application/epub+zip", None);
    let result2 = extractor
        .extract(input2, &config)
        .await
        .expect("Second extraction should succeed");

    assert_eq!(
        result1.content, result2.content,
        "Extraction should be deterministic - same input should produce same output"
    );

    assert_eq!(
        result1.metadata.additional, result2.metadata.additional,
        "Metadata extraction should be deterministic"
    );

    println!("✅ Deterministic extraction test passed");
}

/// Test 6: No content loss across multiple EPUBs
///
/// Validates:
/// - All test EPUB files extract successfully
/// - No file has empty or nearly-empty content
/// - Bug causing 99.84% content loss is fixed
#[tokio::test]
async fn test_native_epub_no_content_loss() {
    let epub_files = vec![
        ("epub2_cover.epub", 10),
        ("epub2_no_cover.epub", 10),
        ("img.epub", 50),
        ("features.epub", 1000),
        ("wasteland.epub", 2000),
    ];

    let extractor = EpubExtractor;
    let config = ExtractionConfig::default();

    for (epub_file, min_bytes) in epub_files {
        let test_file = get_test_epub_path(epub_file);
        if !test_file.exists() {
            println!("⚠ Skipping {}: not found", epub_file);
            continue;
        }

        let bytes = std::fs::read(&test_file).unwrap_or_else(|_| panic!("Failed to read {}", epub_file));
        let input = ExtractInput::from_bytes(bytes, "application/epub+zip", None);
        let result = extractor
            .extract(input, &config)
            .await
            .unwrap_or_else(|_| panic!("Should extract {}", epub_file));

        assert!(
            result.content.len() >= min_bytes,
            "CRITICAL: {} extracted only {} bytes (expected >= {}). Content loss bug?",
            epub_file,
            result.content.len(),
            min_bytes
        );

        println!("✓ {} - {} bytes extracted", epub_file, result.content.len());
    }

    println!("✅ All EPUBs extracted successfully - no content loss!");
}

/// Tests that real embedded MathML (from the EPUB 3 accessibility/conformance
/// test suite fixture) is converted to genuine LaTeX rather than being
/// mangled into raw concatenated symbol text or leaking tag names.
#[tokio::test]
async fn test_native_epub_features_mathml_converts_to_latex() {
    let test_file = get_test_epub_path("features.epub");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let bytes = std::fs::read(&test_file).expect("Failed to read features.epub");
    let extractor = EpubExtractor;
    let config = ExtractionConfig::default();
    let input = ExtractInput::from_bytes(bytes, "application/epub+zip", None);
    let result = extractor
        .extract(input, &config)
        .await
        .expect("Should extract features.epub successfully");

    assert!(
        result.content.contains("x=\\frac{"),
        "Expected the quadratic-formula MathML to convert to a LaTeX \\frac, got: {}",
        result.content
    );
    assert!(
        result.content.contains("\\sum_{n=1}^{\\infty"),
        "Expected the summation MathML to convert to \\sum with its limits, got: {}",
        result.content
    );
    assert!(
        result.content.contains("\\sum\\limits_{"),
        "Expected the movablelimits=\"false\" summation to keep stacked \\limits, got: {}",
        result.content
    );
    assert!(
        !result.content.contains("mfrac") && !result.content.contains("munderover"),
        "Raw MathML tag names must not leak into extracted content, got: {}",
        result.content
    );

    println!("✅ EPUB MathML-to-LaTeX conversion test passed!");
}
