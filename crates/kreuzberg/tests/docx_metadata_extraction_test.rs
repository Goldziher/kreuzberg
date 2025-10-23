//! End-to-end integration test for DOCX metadata extraction

use kreuzberg::extraction::pandoc::extract_file;

#[tokio::test]
async fn test_docx_full_metadata_extraction() {
    // Skip if pandoc is not available
    if kreuzberg::extraction::pandoc::validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    // Path is relative to workspace root when running tests
    let path = std::path::Path::new("../../test_documents/documents/word_sample.docx");
    if !path.exists() {
        // Try alternative path (from workspace root)
        let path = std::path::Path::new("test_documents/documents/word_sample.docx");
        if !path.exists() {
            println!("Skipping test: Test file not found");
            return;
        }
    }

    let result = extract_file(
        std::path::Path::new("../../test_documents/documents/word_sample.docx"),
        "docx",
    )
    .await
    .expect("Should extract DOCX successfully");

    // Verify content extraction
    assert!(!result.content.is_empty(), "Content should not be empty");
    assert!(
        result.content.to_lowercase().contains("swim"),
        "Content should contain 'swim'"
    );

    // Verify core properties
    assert_eq!(
        result.metadata.get("created_by").and_then(|v| v.as_str()),
        Some("Christoph Auer"),
        "Should have correct creator"
    );
    assert_eq!(
        result.metadata.get("modified_by").and_then(|v| v.as_str()),
        Some("Maxim Lysak"),
        "Should have correct last modified by"
    );
    assert_eq!(
        result.metadata.get("created_at").and_then(|v| v.as_str()),
        Some("2024-10-09T12:43:00Z"),
        "Should have correct creation date"
    );
    assert_eq!(
        result.metadata.get("revision").and_then(|v| v.as_str()),
        Some("7"),
        "Should have revision number"
    );

    // Verify app properties
    assert_eq!(
        result.metadata.get("page_count").and_then(|v| v.as_i64()),
        Some(2),
        "Should have 2 pages"
    );
    assert_eq!(
        result.metadata.get("word_count").and_then(|v| v.as_i64()),
        Some(108),
        "Should have 108 words"
    );
    assert_eq!(
        result.metadata.get("character_count").and_then(|v| v.as_i64()),
        Some(620),
        "Should have 620 characters"
    );
    assert_eq!(
        result.metadata.get("line_count").and_then(|v| v.as_i64()),
        Some(5),
        "Should have 5 lines"
    );
    assert_eq!(
        result.metadata.get("paragraph_count").and_then(|v| v.as_i64()),
        Some(1),
        "Should have 1 paragraph"
    );

    println!("✅ DOCX metadata extraction test passed!");
    println!("   Found {} metadata fields", result.metadata.len());
}

#[tokio::test]
async fn test_docx_minimal_metadata_extraction() {
    // Skip if pandoc is not available
    if kreuzberg::extraction::pandoc::validate_pandoc_version().await.is_err() {
        println!("Skipping test: Pandoc not available");
        return;
    }

    let result = extract_file(
        std::path::Path::new("../../test_documents/documents/lorem_ipsum.docx"),
        "docx",
    )
    .await
    .expect("Should extract DOCX successfully");

    // Verify content extraction
    assert!(!result.content.is_empty(), "Content should not be empty");

    // This file has empty core properties, but should have app properties
    assert_eq!(
        result.metadata.get("page_count").and_then(|v| v.as_i64()),
        Some(1),
        "Should have 1 page"
    );
    assert_eq!(
        result.metadata.get("word_count").and_then(|v| v.as_i64()),
        Some(520),
        "Should have 520 words"
    );

    println!("✅ DOCX minimal metadata extraction test passed!");
}
