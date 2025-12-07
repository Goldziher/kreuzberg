//! End-to-end integration test for DOCX metadata extraction

#![cfg(feature = "office")]

use kreuzberg::{ExtractionConfig, extract_file};

#[tokio::test]
async fn test_docx_full_metadata_extraction() {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/documents/word_sample.docx");

    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let result = extract_file(&test_file, None, &ExtractionConfig::default())
        .await
        .expect("Should extract DOCX successfully");

    assert!(!result.content.is_empty(), "Content should not be empty");
    assert!(
        result.content.to_lowercase().contains("swim"),
        "Content should contain 'swim'"
    );

    assert_eq!(
        result.metadata.additional.get("created_by").and_then(|v| v.as_str()),
        Some("Christoph Auer"),
        "Should have correct creator"
    );
    assert_eq!(
        result.metadata.additional.get("modified_by").and_then(|v| v.as_str()),
        Some("Maxim Lysak"),
        "Should have correct last modified by"
    );
    assert_eq!(
        result.metadata.additional.get("created_at").and_then(|v| v.as_str()),
        Some("2024-10-09T12:43:00Z"),
        "Should have correct creation date"
    );
    assert_eq!(
        result.metadata.additional.get("revision").and_then(|v| v.as_str()),
        Some("7"),
        "Should have revision number"
    );

    assert_eq!(
        result.metadata.additional.get("page_count").and_then(|v| v.as_i64()),
        Some(2),
        "Should have 2 pages"
    );
    assert_eq!(
        result.metadata.additional.get("word_count").and_then(|v| v.as_i64()),
        Some(108),
        "Should have 108 words"
    );
    assert_eq!(
        result
            .metadata
            .additional
            .get("character_count")
            .and_then(|v| v.as_i64()),
        Some(620),
        "Should have 620 characters"
    );
    assert_eq!(
        result.metadata.additional.get("line_count").and_then(|v| v.as_i64()),
        Some(5),
        "Should have 5 lines"
    );
    assert_eq!(
        result
            .metadata
            .additional
            .get("paragraph_count")
            .and_then(|v| v.as_i64()),
        Some(1),
        "Should have 1 paragraph"
    );

    println!("✅ DOCX metadata extraction test passed!");
    println!("   Found {} metadata fields", result.metadata.additional.len());
}

#[tokio::test]
async fn test_docx_minimal_metadata_extraction() {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let test_file = workspace_root.join("test_documents/documents/lorem_ipsum.docx");

    if !test_file.exists() {
        println!("Skipping test: Test file not found at {:?}", test_file);
        return;
    }

    let result = extract_file(&test_file, None, &ExtractionConfig::default())
        .await
        .expect("Should extract DOCX successfully");

    assert!(!result.content.is_empty(), "Content should not be empty");

    assert_eq!(
        result.metadata.additional.get("page_count").and_then(|v| v.as_i64()),
        Some(1),
        "Should have 1 page"
    );
    assert_eq!(
        result.metadata.additional.get("word_count").and_then(|v| v.as_i64()),
        Some(520),
        "Should have 520 words"
    );

    println!("✅ DOCX minimal metadata extraction test passed!");
}
