//! Batch processing integration tests.
//!
//! Tests for `batch_extract_file` and `batch_extract_bytes` functions.
//! Validates concurrent processing, error handling, and performance.

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::core::extractor::{
    batch_extract_bytes, batch_extract_bytes_sync, batch_extract_file, batch_extract_file_sync,
};
use std::path::PathBuf;

mod helpers;
use helpers::{get_test_documents_dir, get_test_file_path, skip_if_missing, test_documents_available};

/// Test batch extraction with multiple file formats (PDF, DOCX, TXT).
#[tokio::test]
async fn test_batch_extract_file_multiple_formats() {
    if !test_documents_available() {
        println!("Skipping test: test_documents/ directory not found");
        return;
    }

    // Skip if any required files are missing
    if skip_if_missing("pdfs/fake_memo.pdf")
        || skip_if_missing("documents/fake.docx")
        || skip_if_missing("text/fake_text.txt")
    {
        return;
    }

    let config = ExtractionConfig::default();

    let paths = vec![
        get_test_file_path("pdfs/fake_memo.pdf"),
        get_test_file_path("documents/fake.docx"),
        get_test_file_path("text/fake_text.txt"),
    ];

    let results = batch_extract_file(paths, &config).await;

    assert!(results.is_ok(), "Batch extraction should succeed");
    let results = results.unwrap();

    // Should have 3 results
    assert_eq!(results.len(), 3);

    // Verify each extraction succeeded
    assert!(!results[0].content.is_empty(), "PDF content should not be empty");
    assert_eq!(results[0].mime_type, "application/pdf");

    assert!(!results[1].content.is_empty(), "DOCX content should not be empty");
    assert_eq!(
        results[1].mime_type,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    );

    assert!(!results[2].content.is_empty(), "TXT content should not be empty");
    assert_eq!(results[2].mime_type, "text/plain");

    // Verify no error metadata
    assert!(results[0].metadata.error.is_none());
    assert!(results[1].metadata.error.is_none());
    assert!(results[2].metadata.error.is_none());
}

/// Test synchronous batch extraction variant.
#[test]
fn test_batch_extract_file_sync_variant() {
    if !test_documents_available() {
        println!("Skipping test: test_documents/ directory not found");
        return;
    }

    if skip_if_missing("pdfs/fake_memo.pdf") || skip_if_missing("text/fake_text.txt") {
        return;
    }

    let config = ExtractionConfig::default();

    let paths = vec![
        get_test_file_path("pdfs/fake_memo.pdf"),
        get_test_file_path("text/fake_text.txt"),
    ];

    let results = batch_extract_file_sync(paths, &config);

    assert!(results.is_ok(), "Sync batch extraction should succeed");
    let results = results.unwrap();

    assert_eq!(results.len(), 2);

    // Verify PDF extraction
    assert!(!results[0].content.is_empty(), "PDF content should not be empty");
    assert_eq!(
        results[0].mime_type, "application/pdf",
        "PDF MIME type should be correct"
    );
    assert!(results[0].metadata.error.is_none(), "PDF should extract without errors");

    // Verify text extraction
    assert!(!results[1].content.is_empty(), "Text content should not be empty");
    assert_eq!(results[1].mime_type, "text/plain", "Text MIME type should be correct");
    assert!(
        results[1].metadata.error.is_none(),
        "Text should extract without errors"
    );
}

/// Test batch extraction from bytes.
#[tokio::test]
async fn test_batch_extract_bytes_multiple() {
    let config = ExtractionConfig::default();

    let text_bytes = b"This is plain text content";
    let markdown_bytes = b"# Markdown Header\n\nThis is markdown content";
    let json_bytes = b"{\"key\": \"value\", \"number\": 42}";

    let contents = vec![
        (text_bytes.as_slice(), "text/plain"),
        (markdown_bytes.as_slice(), "text/markdown"),
        (json_bytes.as_slice(), "application/json"),
    ];

    let results = batch_extract_bytes(contents, &config).await;

    assert!(results.is_ok(), "Batch bytes extraction should succeed");
    let results = results.unwrap();

    assert_eq!(results.len(), 3);

    // Text and markdown should extract cleanly
    assert_eq!(results[0].content, "This is plain text content");
    assert_eq!(results[0].mime_type, "text/plain");

    assert!(results[1].content.contains("Markdown Header"));
    assert_eq!(results[1].mime_type, "text/markdown");

    // JSON should extract key-value pairs
    assert!(results[2].content.contains("key"));
    assert!(results[2].content.contains("value"));
    assert_eq!(results[2].mime_type, "application/json");
}

/// Test batch extraction with empty file list.
#[tokio::test]
async fn test_batch_extract_empty_list() {
    let config = ExtractionConfig::default();

    let paths: Vec<PathBuf> = vec![];
    let results = batch_extract_file(paths, &config).await;

    assert!(results.is_ok(), "Empty batch should succeed");
    assert_eq!(results.unwrap().len(), 0, "Should return empty vector");
}

/// Test batch extraction when one file fails (others should succeed).
#[tokio::test]
async fn test_batch_extract_one_file_fails() {
    if !test_documents_available() {
        println!("Skipping test: test_documents/ directory not found");
        return;
    }

    if skip_if_missing("text/fake_text.txt") {
        return;
    }

    let config = ExtractionConfig::default();

    let paths = vec![
        get_test_file_path("text/fake_text.txt"),              // Valid file
        get_test_documents_dir().join("nonexistent_file.txt"), // Invalid file
        get_test_file_path("text/contract.txt"),               // Valid file
    ];

    let results = batch_extract_file(paths, &config).await;

    assert!(results.is_ok(), "Batch should succeed even with one failure");
    let results = results.unwrap();

    assert_eq!(results.len(), 3);

    // First file should succeed
    assert!(!results[0].content.is_empty());
    assert!(results[0].metadata.error.is_none());

    // Second file should have error metadata
    assert!(results[1].metadata.error.is_some());
    assert!(results[1].content.contains("Error:"));

    // Third file should succeed
    assert!(!results[2].content.is_empty());
    assert!(results[2].metadata.error.is_none());
}

/// Test batch extraction when all files fail.
#[tokio::test]
async fn test_batch_extract_all_fail() {
    let config = ExtractionConfig::default();

    let test_dir = get_test_documents_dir();
    let paths = vec![
        test_dir.join("nonexistent1.txt"),
        test_dir.join("nonexistent2.pdf"),
        test_dir.join("nonexistent3.docx"),
    ];

    let results = batch_extract_file(paths, &config).await;

    assert!(results.is_ok(), "Batch should succeed (errors in metadata)");
    let results = results.unwrap();

    assert_eq!(results.len(), 3);

    // All results should have error metadata
    assert!(results[0].metadata.error.is_some());
    assert!(results[1].metadata.error.is_some());
    assert!(results[2].metadata.error.is_some());

    // All contents should be error messages
    assert!(results[0].content.contains("Error:"));
    assert!(results[1].content.contains("Error:"));
    assert!(results[2].content.contains("Error:"));
}

/// Test concurrent batch processing (verify parallelism).
#[tokio::test]
async fn test_batch_extract_concurrent() {
    if !test_documents_available() {
        println!("Skipping test: test_documents/ directory not found");
        return;
    }

    if skip_if_missing("text/fake_text.txt") {
        return;
    }

    let config = ExtractionConfig::default();

    // Use same file multiple times to test concurrent access
    let base_path = get_test_file_path("text/fake_text.txt");
    let paths: Vec<PathBuf> = (0..20).map(|_| base_path.clone()).collect();

    let start = std::time::Instant::now();
    let results = batch_extract_file(paths, &config).await;
    let duration = start.elapsed();

    assert!(results.is_ok(), "Concurrent batch should succeed");
    let results = results.unwrap();

    assert_eq!(results.len(), 20);

    // Verify all succeeded
    for result in &results {
        assert!(result.metadata.error.is_none(), "Result should not have errors");
        assert!(!result.content.is_empty(), "Result content should not be empty");
        assert_eq!(result.mime_type, "text/plain", "MIME type should be text/plain");
    }

    // Verify actual content from at least first result (all should be identical)
    assert!(
        !results[0].content.is_empty(),
        "Should have extracted actual text content"
    );

    // Concurrent processing should be faster than sequential
    // (This is a heuristic - 20 files in < 5 seconds is reasonable for text files)
    assert!(duration.as_secs() < 5, "Batch processing took too long: {:?}", duration);
}

/// Test large batch (50+ files).
#[tokio::test]
async fn test_batch_extract_large_batch() {
    if !test_documents_available() {
        println!("Skipping test: test_documents/ directory not found");
        return;
    }

    if skip_if_missing("text/fake_text.txt") {
        return;
    }

    let config = ExtractionConfig::default();

    // Use same file 50 times (simulates large batch)
    let base_path = get_test_file_path("text/fake_text.txt");
    let paths: Vec<PathBuf> = (0..50).map(|_| base_path.clone()).collect();

    let results = batch_extract_file(paths, &config).await;

    assert!(results.is_ok(), "Large batch should succeed");
    let results = results.unwrap();

    assert_eq!(results.len(), 50);

    // Verify all succeeded
    for result in &results {
        assert!(result.metadata.error.is_none());
        assert!(!result.content.is_empty());
        assert_eq!(result.mime_type, "text/plain");
    }
}

/// Test sync variant with bytes.
#[test]
fn test_batch_extract_bytes_sync_variant() {
    let config = ExtractionConfig::default();

    let contents = vec![
        (b"content 1".as_slice(), "text/plain"),
        (b"content 2".as_slice(), "text/plain"),
        (b"# content 3".as_slice(), "text/markdown"),
    ];

    let results = batch_extract_bytes_sync(contents, &config);

    assert!(results.is_ok(), "Sync batch bytes extraction should succeed");
    let results = results.unwrap();

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].content, "content 1");
    assert_eq!(results[1].content, "content 2");
    assert!(results[2].content.contains("content 3"));
}
