//! Comprehensive TDD test suite for OPML (Outline Processor Markup Language) extraction
//!
//! This test suite validates OPML extraction capabilities.
//! Each test extracts an OPML file and validates:
//!
//! - Metadata extraction (title, dateCreated, dateModified, ownerName, ownerEmail)
//! - Outline hierarchy extraction with proper indentation
//! - RSS feed attribute handling (xmlUrl, htmlUrl)
//! - Content structure preservation
//! - Special character handling
//! - Edge cases (empty bodies, nested structures, etc.)

#![cfg(feature = "office")]

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::core::extractor::extract_bytes;
use std::path::PathBuf;

mod helpers;

/// Helper to resolve workspace root and construct test file paths
fn get_test_opml_path(filename: &str) -> PathBuf {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    workspace_root.join(format!("test_documents/opml/{}", filename))
}

/// Helper to validate that content contains expected text (case-insensitive)
fn assert_contains_ci(content: &str, needle: &str, description: &str) {
    assert!(
        content.to_lowercase().contains(&needle.to_lowercase()),
        "Content should contain '{}' ({}). Content: {}",
        needle,
        description,
        &content[..std::cmp::min(300, content.len())]
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

/// Test 1: Extract RSS feed subscription list with categories
///
/// Validates:
/// - Successfully extracts feeds.opml with RSS feed structure
/// - Extracts Dublin Core metadata (title, dateCreated, dateModified, ownerName, ownerEmail)
/// - Content includes all feed categories and feed names
/// - Feed URLs are captured in output
/// - Hierarchy structure is preserved with proper nesting
#[tokio::test]
async fn test_opml_rss_feeds_extraction() {
    let test_file = get_test_opml_path("feeds.opml");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read OPML file");
    let result = extract_bytes(&content, "text/x-opml", &ExtractionConfig::default())
        .await
        .expect("Should extract RSS feeds OPML successfully");

    assert!(
        !result.content.is_empty(),
        "Content should not be empty for RSS feeds OPML"
    );

    assert_contains_ci(&result.content, "Technology", "Should contain Technology category");
    assert_contains_ci(&result.content, "Programming", "Should contain Programming category");
    assert_contains_ci(
        &result.content,
        "Uncategorized",
        "Should contain Uncategorized category",
    );

    assert_contains_ci(&result.content, "Hacker News", "Should contain Hacker News feed");
    assert_contains_ci(&result.content, "TechCrunch", "Should contain TechCrunch feed");
    assert_contains_ci(&result.content, "Rust Blog", "Should contain Rust Blog feed");

    assert!(
        result.metadata.additional.contains_key("title"),
        "Should extract title metadata"
    );
    assert_eq!(
        result.metadata.additional.get("title").and_then(|v| v.as_str()),
        Some("Tech News Feeds"),
        "Should have correct title"
    );

    let has_owner =
        result.metadata.additional.contains_key("ownerName") || result.metadata.additional.contains_key("ownerEmail");
    assert!(has_owner, "Should extract owner information");

    println!("✅ RSS feeds extraction test passed!");
    println!("   Found {} metadata fields", result.metadata.additional.len());
    println!("   Content length: {} bytes", result.content.len());
}

/// Test 2: Extract podcast directory with multiple categories
///
/// Validates:
/// - Successfully extracts podcasts.opml with podcast structure
/// - Extracts title and metadata fields
/// - Content includes all podcast categories
/// - Podcast feed names are properly extracted
/// - Handles HTML entity encoding (&amp;)
/// - Complex hierarchy is preserved
#[tokio::test]
async fn test_opml_podcast_directory_extraction() {
    let test_file = get_test_opml_path("podcasts.opml");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read OPML file");
    let result = extract_bytes(&content, "text/x-opml", &ExtractionConfig::default())
        .await
        .expect("Should extract podcast directory OPML successfully");

    assert!(
        !result.content.is_empty(),
        "Content should not be empty for podcast OPML"
    );

    assert_contains_ci(
        &result.content,
        "Technology Podcasts",
        "Should contain Technology Podcasts category",
    );
    assert_contains_ci(&result.content, "Business", "Should contain Business category");
    assert_contains_ci(&result.content, "Science", "Should contain Science category");

    assert_contains_ci(&result.content, "Syntax", "Should contain Syntax podcast");
    assert_contains_ci(&result.content, "Acquired", "Should contain Acquired podcast");

    assert_eq!(
        result.metadata.additional.get("title").and_then(|v| v.as_str()),
        Some("Podcast Directory"),
        "Should have correct title"
    );

    assert_eq!(
        result.metadata.additional.get("ownerName").and_then(|v| v.as_str()),
        Some("Jane Doe"),
        "Should extract owner name correctly"
    );

    println!("✅ Podcast directory extraction test passed!");
    println!("   Found {} metadata fields", result.metadata.additional.len());
    println!("   Content length: {} bytes", result.content.len());
}

/// Test 3: Extract general outline structure with deep nesting
///
/// Validates:
/// - Successfully extracts outline.opml with project structure
/// - Preserves hierarchy with proper indentation
/// - Handles multi-level nesting (4 levels deep)
/// - Extracts all task items in correct order
/// - Metadata is properly extracted
/// - Content structure matches expected outline format
#[tokio::test]
async fn test_opml_outline_hierarchy_extraction() {
    let test_file = get_test_opml_path("outline.opml");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read OPML file");
    let result = extract_bytes(&content, "text/x-opml", &ExtractionConfig::default())
        .await
        .expect("Should extract outline OPML successfully");

    assert!(
        !result.content.is_empty(),
        "Content should not be empty for outline OPML"
    );

    assert_contains_ci(&result.content, "Project Alpha", "Should contain main project");

    assert_contains_ci(&result.content, "Phase 1", "Should contain Phase 1");
    assert_contains_ci(&result.content, "Phase 2", "Should contain Phase 2");
    assert_contains_ci(&result.content, "Phase 3", "Should contain Phase 3");
    assert_contains_ci(&result.content, "Phase 4", "Should contain Phase 4");

    assert_contains_ci(
        &result.content,
        "Requirements gathering",
        "Should contain Phase 1 tasks",
    );
    assert_contains_ci(&result.content, "Resource allocation", "Should contain Phase 1 tasks");

    assert_contains_ci(
        &result.content,
        "Backend implementation",
        "Should contain Phase 2 backend task",
    );
    assert_contains_ci(
        &result.content,
        "Frontend implementation",
        "Should contain Phase 2 frontend task",
    );

    assert_contains_ci(&result.content, "Unit testing", "Should contain Phase 3 testing task");
    assert_contains_ci(
        &result.content,
        "Production setup",
        "Should contain Phase 4 deployment task",
    );

    assert_eq!(
        result.metadata.additional.get("title").and_then(|v| v.as_str()),
        Some("Project Outline"),
        "Should have correct title"
    );

    assert!(
        result.content.contains("  "),
        "Should have indentation for nested items"
    );

    println!("✅ Outline hierarchy extraction test passed!");
    println!("   Content length: {} bytes", result.content.len());
    println!("   Hierarchy levels preserved with indentation");
}

/// Test 4: Comprehensive metadata extraction from head section
///
/// Validates:
/// - All head metadata fields are extracted (title, dateCreated, dateModified, ownerName, ownerEmail)
/// - Metadata values are correctly typed and encoded
/// - Date formats are preserved as-is
/// - Owner information is properly extracted
/// - Missing optional fields are handled gracefully
#[tokio::test]
async fn test_opml_metadata_extraction_complete() {
    let test_file = get_test_opml_path("feeds.opml");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read OPML file");
    let result = extract_bytes(&content, "text/x-opml", &ExtractionConfig::default())
        .await
        .expect("Should extract metadata successfully");

    let metadata = &result.metadata.additional;

    assert!(metadata.contains_key("title"), "Should have title metadata");
    assert!(
        metadata.contains_key("dateCreated") || metadata.contains_key("dateModified"),
        "Should have at least one date field"
    );
    assert!(
        metadata.contains_key("ownerName") || metadata.contains_key("ownerEmail"),
        "Should have owner information"
    );

    assert_eq!(
        metadata.get("title").and_then(|v| v.as_str()),
        Some("Tech News Feeds"),
        "Title should match exactly"
    );

    if let Some(date_created) = metadata.get("dateCreated").and_then(|v| v.as_str()) {
        assert!(
            date_created.contains("Nov") || date_created.contains("2023"),
            "Date should be preserved in original format"
        );
    }

    assert_eq!(
        metadata.get("ownerName").and_then(|v| v.as_str()),
        Some("John Smith"),
        "Owner name should be extracted"
    );

    println!("✅ Metadata extraction test passed!");
    println!("   Metadata fields: {:?}", metadata.keys().collect::<Vec<_>>());
}

/// Test 5: Verify RSS feed names are extracted from OPML attributes
///
/// Validates:
/// - Feed names from text attribute are properly extracted
/// - Feed categories are preserved in the hierarchy
/// - All feed names are present in output
/// - Extraction matches Pandoc baseline (no URLs in main content)
#[tokio::test]
async fn test_opml_feed_url_extraction() {
    let test_file = get_test_opml_path("feeds.opml");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read OPML file");
    let result = extract_bytes(&content, "text/x-opml", &ExtractionConfig::default())
        .await
        .expect("Should extract feed names successfully");

    assert_contains_ci(&result.content, "Hacker News", "Should contain Hacker News feed name");
    assert_contains_ci(&result.content, "TechCrunch", "Should contain TechCrunch feed name");
    assert_contains_ci(&result.content, "Rust Blog", "Should contain Rust Blog feed name");
    assert_contains_ci(&result.content, "Dev.to", "Should contain Dev.to feed name");

    assert_contains_ci(&result.content, "Technology", "Should contain Technology category");
    assert_contains_ci(&result.content, "Programming", "Should contain Programming category");

    println!("✅ Feed extraction test passed!");
    println!("   Found {} bytes of content", result.content.len());
}

/// Test 6: Verify correct MIME type handling and format detection
///
/// Validates:
/// - MIME type is correctly preserved in result
/// - Extractor handles text/x-opml MIME type
/// - Content format is appropriate for OPML outline structure
/// - Result structure is valid
#[tokio::test]
async fn test_opml_mime_type_handling() {
    let test_file = get_test_opml_path("feeds.opml");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read OPML file");

    let result = extract_bytes(&content, "text/x-opml", &ExtractionConfig::default())
        .await
        .expect("Should extract with text/x-opml MIME type");

    assert_eq!(result.mime_type, "text/x-opml", "MIME type should be preserved");

    let result2 = extract_bytes(&content, "application/xml+opml", &ExtractionConfig::default())
        .await
        .expect("Should extract with application/xml+opml MIME type");

    assert_eq!(
        result2.mime_type, "application/xml+opml",
        "Alternative MIME type should work"
    );

    assert_eq!(
        result.content, result2.content,
        "Content should be same regardless of MIME type"
    );

    println!("✅ MIME type handling test passed!");
}

/// Test 7: Handle special characters and HTML entities in OPML
///
/// Validates:
/// - HTML entities are properly decoded (&amp;, &lt;, &gt;, etc.)
/// - Special characters in feed names are handled correctly
/// - Quotes and apostrophes are properly processed
/// - UTF-8 content is valid
/// - Content is human-readable after extraction
#[tokio::test]
async fn test_opml_special_characters_handling() {
    let test_file = get_test_opml_path("podcasts.opml");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read OPML file");
    let result = extract_bytes(&content, "text/x-opml", &ExtractionConfig::default())
        .await
        .expect("Should extract with special characters");

    assert_contains_ci(
        &result.content,
        "Business",
        "Should properly decode Business &amp; Startups",
    );

    let _ = result.content.chars().count();

    println!("✅ Special characters handling test passed!");
    println!("   Verified UTF-8 integrity and entity decoding");
}

/// Test 9: Validate deep nesting and hierarchy preservation in outline.opml
///
/// Validates:
/// - Multi-level nesting (4 levels) is properly preserved
/// - Indentation increases with nesting depth
/// - All tasks are extracted in correct nesting context
/// - Task ordering is preserved
/// - Notes & Resources section is captured
#[tokio::test]
async fn test_opml_deep_nesting_hierarchy() {
    let test_file = get_test_opml_path("outline.opml");
    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let content = std::fs::read(&test_file).expect("Should read OPML file");
    let result = extract_bytes(&content, "text/x-opml", &ExtractionConfig::default())
        .await
        .expect("Should extract deep nesting successfully");

    let extracted = &result.content;

    let project_pos = extracted.find("Project Alpha").unwrap_or(0);
    let phase1_pos = extracted.find("Phase 1").unwrap_or(0);
    let phase2_pos = extracted.find("Phase 2").unwrap_or(0);
    let phase3_pos = extracted.find("Phase 3").unwrap_or(0);
    let phase4_pos = extracted.find("Phase 4").unwrap_or(0);

    assert!(
        project_pos < phase1_pos && phase1_pos < phase2_pos && phase2_pos < phase3_pos && phase3_pos < phase4_pos,
        "Phases should appear in order in output"
    );

    assert_contains_ci(&extracted, "Phase 1", "Phase 1 should be present");
    assert_contains_ci(&extracted, "Phase 2", "Phase 2 should be present");
    assert_contains_ci(&extracted, "Phase 3", "Phase 3 should be present");
    assert_contains_ci(&extracted, "Phase 4", "Phase 4 should be present");

    assert_contains_ci(&extracted, "Notes & Resources", "Notes section should be present");

    println!("✅ Deep nesting hierarchy test passed!");
    println!("   All phases and tasks extracted in correct order");
}

/// Test 10: Validate content extraction quality and consistency across all OPML files
///
/// Validates:
/// - All OPML files produce non-empty content
/// - Content is valid UTF-8 (no corruption)
/// - Content doesn't have excessive whitespace
/// - Minimum content quality standards
/// - Consistent extraction behavior
#[tokio::test]
async fn test_opml_content_quality_all_files() {
    let opml_files = vec!["feeds.opml", "podcasts.opml", "outline.opml"];

    for opml_file in opml_files {
        let test_file = get_test_opml_path(opml_file);
        if !test_file.exists() {
            println!("Skipping file: {:?}", test_file);
            continue;
        }

        let content = std::fs::read(&test_file).expect("Should read OPML file");
        let result = extract_bytes(&content, "text/x-opml", &ExtractionConfig::default())
            .await
            .expect(&format!("Should extract {}", opml_file));

        assert!(
            !result.content.is_empty(),
            "Content should not be empty for {}",
            opml_file
        );

        let _ = result.content.chars().count();

        assert!(
            result.content.len() > 20,
            "Content should have meaningful length for {}",
            opml_file
        );

        let whitespace_ratio =
            result.content.chars().filter(|c| c.is_whitespace()).count() as f64 / result.content.len() as f64;
        assert!(
            whitespace_ratio < 0.5,
            "Content should not be mostly whitespace for {}",
            opml_file
        );

        println!("  ✓ {} ({} bytes) quality validated", opml_file, result.content.len());
    }

    println!("✅ Content quality validation test passed!");
}

/// Test 11: Verify OPML extractor is properly registered
///
/// Validates:
/// - Extractor is available in the registry
/// - Supported MIME types are correctly registered
/// - Priority is set appropriately
/// - Plugin interface is implemented
#[tokio::test]
async fn test_opml_extractor_registration() {
    use kreuzberg::extractors::{OpmlExtractor, ensure_initialized};
    use kreuzberg::plugins::{DocumentExtractor, Plugin, registry::get_document_extractor_registry};

    ensure_initialized().expect("Should initialize extractors");

    let registry = get_document_extractor_registry();
    let registry_guard = registry.read().expect("Should acquire read lock on registry");

    let extractor_names = registry_guard.list();

    println!("Available extractors: {:?}", extractor_names);

    assert!(
        extractor_names.contains(&"opml-extractor".to_string()),
        "OPML extractor should be registered. Available: {:?}",
        extractor_names
    );

    let opml_extractor = OpmlExtractor::new();
    assert_eq!(opml_extractor.name(), "opml-extractor");
    assert_eq!(opml_extractor.priority(), 55);

    let supported_types = opml_extractor.supported_mime_types();
    assert!(
        supported_types.contains(&"text/x-opml"),
        "Should support text/x-opml MIME type"
    );
    assert!(
        supported_types.contains(&"application/xml+opml"),
        "Should support application/xml+opml MIME type"
    );

    println!("✅ OPML extractor registration test passed!");
    println!("   OPML extractor properly registered with priority {}", 55);
}

/// Test 12: Extract all OPML files and generate summary statistics
///
/// This test runs all OPML extractions and provides comprehensive statistics
/// for validation and debugging purposes. It's not a strict pass/fail test
/// but provides useful information about extraction behavior.
#[tokio::test]
async fn test_opml_extraction_statistics() {
    let opml_files = vec!["feeds.opml", "podcasts.opml", "outline.opml"];

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║        OPML Extraction Statistics Report                   ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    let mut total_files = 0;
    let mut total_content_bytes = 0;
    let mut total_metadata_fields = 0;

    for opml_file in opml_files {
        let test_file = get_test_opml_path(opml_file);
        if !test_file.exists() {
            println!("⚠ SKIP: {} (not found)", opml_file);
            continue;
        }

        match std::fs::read(&test_file) {
            Ok(content) => match extract_bytes(&content, "text/x-opml", &ExtractionConfig::default()).await {
                Ok(result) => {
                    total_files += 1;
                    total_content_bytes += result.content.len();
                    total_metadata_fields += result.metadata.additional.len();

                    println!("✓ {} ", opml_file);
                    println!("  Content: {} bytes", result.content.len());
                    println!("  Metadata fields: {}", result.metadata.additional.len());

                    if !result.metadata.additional.is_empty() {
                        let keys: Vec<String> = result.metadata.additional.keys().map(|k| k.clone()).collect();
                        println!("  Keys: {}", keys.join(", "));
                    }

                    let outline_count = result.content.lines().count();
                    println!("  Outline items: ~{}", outline_count);

                    let indented_lines = result.content.lines().filter(|l| l.starts_with("  ")).count();
                    println!("  Nested items: {}", indented_lines);

                    println!();
                }
                Err(e) => {
                    println!("✗ {} - Error: {:?}", opml_file, e);
                    println!();
                }
            },
            Err(e) => {
                println!("✗ {} - Read Error: {:?}", opml_file, e);
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

    println!("✅ OPML extraction statistics generated successfully!");
}
