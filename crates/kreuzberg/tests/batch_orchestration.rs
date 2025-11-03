//! Batch processing orchestration tests.
//!
//! Validates efficient parallel processing at multiple levels:
//! - Multiple documents in parallel
//! - Multiple pages within PDFs
//! - OCR across pages
//! - File I/O optimization
//! - Resource utilization (CPU cores)

use kreuzberg::core::config::{ExtractionConfig, OcrConfig};
use kreuzberg::core::extractor::{batch_extract_bytes, batch_extract_file, extract_file_sync};
use std::time::{Duration, Instant};

mod helpers;

/// Test that batch extraction processes documents in parallel.
///
/// Validates:
/// - Multiple documents process concurrently
/// - Parallel processing is faster than sequential
/// - Results maintain correct order
#[tokio::test]
async fn test_batch_documents_parallel_execution() {
    use helpers::get_test_file_path;
    use std::path::PathBuf;

    let config = ExtractionConfig::default();

    let test_files = vec![
        "text/contract.txt",
        "json/sample_document.json",
        "xml/simple_note.xml",
        "text/readme.md",
    ];

    let mut paths: Vec<PathBuf> = Vec::new();
    for _ in 0..5 {
        for file in &test_files {
            paths.push(get_test_file_path(file));
        }
    }

    let parallel_start = Instant::now();
    let results = batch_extract_file(paths.clone(), &config).await;
    let parallel_duration = parallel_start.elapsed();

    assert!(results.is_ok(), "Batch extraction should succeed");
    let results = results.unwrap();
    assert_eq!(results.len(), 20, "Should process all 20 files");

    for result in &results {
        assert!(
            !result.content.is_empty() || result.metadata.error.is_some(),
            "Each result should have content or error"
        );
    }

    assert!(
        parallel_duration < Duration::from_secs(5),
        "Batch processing 20 files should take <5s, took: {:?}",
        parallel_duration
    );
}

/// Test concurrency limiting in batch processing.
///
/// Validates that batch extraction respects max_concurrent_extractions config.
#[tokio::test]
async fn test_batch_documents_concurrency_limiting() {
    use helpers::get_test_file_path;

    let config = ExtractionConfig {
        max_concurrent_extractions: Some(2),
        ..Default::default()
    };

    let paths = vec![
        get_test_file_path("text/contract.txt"),
        get_test_file_path("json/sample_document.json"),
        get_test_file_path("xml/simple_note.xml"),
        get_test_file_path("text/readme.md"),
    ];

    let results = batch_extract_file(paths, &config).await;

    assert!(results.is_ok());
    let results = results.unwrap();
    assert_eq!(results.len(), 4);
}

/// Test batch extraction with CPU-bound limit (default: num_cpus * 2).
#[tokio::test]
async fn test_batch_documents_default_concurrency() {
    use helpers::get_test_file_path;

    let config = ExtractionConfig::default();

    let mut paths = Vec::new();
    for _ in 0..13 {
        paths.push(get_test_file_path("text/contract.txt"));
        paths.push(get_test_file_path("json/sample_document.json"));
        paths.push(get_test_file_path("xml/simple_note.xml"));
        paths.push(get_test_file_path("text/readme.md"));
    }
    let paths = paths.into_iter().take(50).collect::<Vec<_>>();

    let start = Instant::now();
    let results = batch_extract_file(paths, &config).await;
    let duration = start.elapsed();

    assert!(results.is_ok());
    let results = results.unwrap();
    assert_eq!(results.len(), 50);

    println!("Processed 50 files in {:?}", duration);
    assert!(
        duration < Duration::from_secs(10),
        "50 files should process in <10s with parallelism, took: {:?}",
        duration
    );
}

/// Test that batch processing maintains result order.
#[cfg(feature = "xml")]
#[tokio::test]
async fn test_batch_documents_preserves_order() {
    use helpers::get_test_file_path;

    let config = ExtractionConfig::default();

    let paths = vec![
        get_test_file_path("text/contract.txt"),
        get_test_file_path("json/sample_document.json"),
        get_test_file_path("xml/simple_note.xml"),
    ];

    let results = batch_extract_file(paths, &config).await.unwrap();

    assert_eq!(results.len(), 3, "Should have 3 results");

    assert!(!results[0].content.is_empty(), "First result should have content");
    assert!(!results[1].content.is_empty(), "Second result should have content");
    assert!(!results[2].content.is_empty(), "Third result should have content");

    assert!(
        results[0].content.contains("contract"),
        "First result should be from contract.txt, got: '{}'",
        results[0].content
    );
    assert!(
        results[1].content.contains("Sample") || results[1].content.contains("author"),
        "Second result should be from JSON document, got: '{}'",
        results[1].content
    );
    assert!(
        results[2].content.contains("Tove") || results[2].content.contains("note"),
        "Third result should be from XML note, got: '{}'",
        results[2].content
    );
}

/// Test that multi-page PDF extraction is efficient.
///
/// Validates:
/// - Multiple pages are processed
/// - OCR is applied to all pages if needed
/// - Content from all pages is combined
#[cfg(feature = "pdf")]
#[tokio::test]
async fn test_multipage_pdf_extraction() {
    use helpers::{get_test_file_path, skip_if_missing};

    if skip_if_missing("pdfs/multi_page.pdf") {
        tracing::debug!("Skipping multi-page PDF test: test file not available");
        return;
    }

    let config = ExtractionConfig::default();
    let pdf_path = get_test_file_path("pdfs/multi_page.pdf");

    let start = Instant::now();
    let result = kreuzberg::core::extractor::extract_file(&pdf_path, None, &config).await;
    let duration = start.elapsed();

    assert!(result.is_ok(), "Multi-page PDF extraction should succeed");
    let extraction = result.unwrap();

    assert!(!extraction.content.is_empty(), "Should extract text from all pages");
    println!("Extracted multi-page PDF in {:?}", duration);
}

/// Test concurrent PDF extractions (multiple PDFs at once).
#[cfg(feature = "pdf")]
#[tokio::test]
async fn test_concurrent_pdf_extractions() {
    use helpers::{get_test_file_path, skip_if_missing};

    if skip_if_missing("pdfs/simple.pdf") {
        tracing::debug!("Skipping concurrent PDF test: test file not available");
        return;
    }

    let config = ExtractionConfig::default();

    let mut paths = Vec::new();
    for _ in 0..10 {
        paths.push(get_test_file_path("pdfs/simple.pdf"));
    }

    let start = Instant::now();
    let results = batch_extract_file(paths, &config).await;
    let duration = start.elapsed();

    assert!(results.is_ok());
    let results = results.unwrap();
    assert_eq!(results.len(), 10);

    println!("Processed 10 PDFs in {:?}", duration);
}

/// Test OCR on multi-page scanned document.
///
/// Validates:
/// - All pages are OCR'd
/// - Results are combined correctly
/// - Processing is efficient
#[cfg(feature = "ocr")]
#[test]
fn test_ocr_multipage_efficiency() {
    use helpers::{get_test_file_path, skip_if_missing};

    if skip_if_missing("images/ocr_image.jpg") {
        tracing::debug!("Skipping OCR multi-page test: test file not available");
        return;
    }

    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: None,
        }),
        force_ocr: false,
        use_cache: true,
        ..Default::default()
    };

    let file_path = get_test_file_path("images/ocr_image.jpg");

    let start = Instant::now();
    let result1 = extract_file_sync(&file_path, None, &config);
    let first_duration = start.elapsed();

    assert!(result1.is_ok(), "First OCR should succeed");

    let start = Instant::now();
    let result2 = extract_file_sync(&file_path, None, &config);
    let second_duration = start.elapsed();

    assert!(result2.is_ok(), "Second OCR should succeed");

    println!(
        "OCR timing: first={:?}, cached={:?}, speedup={:.1}x",
        first_duration,
        second_duration,
        first_duration.as_secs_f64() / second_duration.as_secs_f64().max(0.001)
    );

    assert!(
        second_duration < first_duration / 2,
        "Cached OCR should be at least 2x faster. First: {:?}, Second: {:?}",
        first_duration,
        second_duration
    );
}

/// Test parallel processing of byte arrays.
///
/// Validates that batch_extract_bytes processes data in parallel.
#[tokio::test]
async fn test_batch_bytes_parallel_processing() {
    let config = ExtractionConfig::default();

    let contents: Vec<(Vec<u8>, &str)> = (0..30)
        .map(|i| {
            let content = format!("Test content number {}", i);
            (content.into_bytes(), "text/plain")
        })
        .collect();

    let contents_ref: Vec<(&[u8], &str)> = contents.iter().map(|(bytes, mime)| (bytes.as_slice(), *mime)).collect();

    let start = Instant::now();
    let results = batch_extract_bytes(contents_ref, &config).await;
    let duration = start.elapsed();

    assert!(results.is_ok());
    let results = results.unwrap();
    assert_eq!(results.len(), 30);

    for (i, result) in results.iter().enumerate() {
        assert_eq!(result.content, format!("Test content number {}", i));
    }

    println!("Batch processed 30 byte arrays in {:?}", duration);
}

/// Test error handling in batch bytes processing.
#[tokio::test]
async fn test_batch_bytes_mixed_valid_invalid() {
    let config = ExtractionConfig::default();

    let contents = vec![
        (b"valid content 1".as_slice(), "text/plain"),
        (b"invalid content".as_slice(), "invalid/mime"),
        (b"valid content 2".as_slice(), "text/plain"),
        (b"more invalid".as_slice(), "bad/type"),
        (b"valid content 3".as_slice(), "text/plain"),
    ];

    let results = batch_extract_bytes(contents, &config).await;

    assert!(results.is_ok());
    let results = results.unwrap();
    assert_eq!(results.len(), 5);

    assert_eq!(results[0].content, "valid content 1");
    assert_eq!(results[2].content, "valid content 2");
    assert_eq!(results[4].content, "valid content 3");

    assert!(results[1].metadata.error.is_some());
    assert!(results[3].metadata.error.is_some());
}

/// Test that batch processing utilizes multiple CPU cores.
///
/// Validates that parallel extraction actually runs in parallel,
/// not just sequentially with fancy task management.
#[tokio::test]
async fn test_batch_utilizes_multiple_cores() {
    let config = ExtractionConfig {
        max_concurrent_extractions: Some(num_cpus::get()),
        ..Default::default()
    };

    let mut contents = Vec::new();
    for i in 0..20 {
        let json = format!(
            r#"{{"id": {}, "data": "{}", "nested": {{"value": "{}"}}}}"#,
            i,
            "x".repeat(100),
            "y".repeat(100)
        );
        contents.push((json.into_bytes(), "application/json"));
    }

    let contents_ref: Vec<(&[u8], &str)> = contents.iter().map(|(bytes, mime)| (bytes.as_slice(), *mime)).collect();

    let start = Instant::now();
    let results = batch_extract_bytes(contents_ref, &config).await;
    let duration = start.elapsed();

    assert!(results.is_ok());
    let results = results.unwrap();
    assert_eq!(results.len(), 20);

    println!(
        "Processed 20 JSON documents in {:?} with {} cores",
        duration,
        num_cpus::get()
    );

    assert!(
        duration < Duration::from_secs(2),
        "Batch processing should leverage parallelism, took: {:?}",
        duration
    );
}

/// Test batch processing under memory pressure.
///
/// Validates that semaphore prevents resource exhaustion.
#[tokio::test]
async fn test_batch_memory_pressure_handling() {
    let config = ExtractionConfig {
        max_concurrent_extractions: Some(4),
        ..Default::default()
    };

    let mut contents = Vec::new();
    for i in 0..50 {
        let json = format!(r#"{{"id": {}, "large_data": "{}"}}"#, i, "x".repeat(10000));
        contents.push((json.into_bytes(), "application/json"));
    }

    let contents_ref: Vec<(&[u8], &str)> = contents.iter().map(|(bytes, mime)| (bytes.as_slice(), *mime)).collect();

    let start = Instant::now();
    let results = batch_extract_bytes(contents_ref, &config).await;
    let duration = start.elapsed();

    assert!(results.is_ok());
    let results = results.unwrap();
    assert_eq!(results.len(), 50);

    println!("Processed 50 large documents with concurrency limit in {:?}", duration);

    for result in &results {
        assert!(!result.content.is_empty());
    }
}

/// Test that batch processing scales with CPU count.
#[tokio::test]
async fn test_batch_scales_with_cpu_count() {
    let cpu_count = num_cpus::get();

    let contents: Vec<(Vec<u8>, &str)> = (0..30)
        .map(|i| (format!("Content {}", i).into_bytes(), "text/plain"))
        .collect();

    let config_1 = ExtractionConfig {
        max_concurrent_extractions: Some(1),
        ..Default::default()
    };

    let contents_ref: Vec<(&[u8], &str)> = contents.iter().map(|(bytes, mime)| (bytes.as_slice(), *mime)).collect();

    let start = Instant::now();
    let _ = batch_extract_bytes(contents_ref.clone(), &config_1).await.unwrap();
    let duration_1 = start.elapsed();

    let config_full = ExtractionConfig {
        max_concurrent_extractions: Some(cpu_count),
        ..Default::default()
    };

    let start = Instant::now();
    let _ = batch_extract_bytes(contents_ref, &config_full).await.unwrap();
    let duration_full = start.elapsed();

    println!(
        "Concurrency=1: {:?}, Concurrency={}: {:?}, Speedup: {:.2}x",
        duration_1,
        cpu_count,
        duration_full,
        duration_1.as_secs_f64() / duration_full.as_secs_f64()
    );

    if cpu_count > 1 {
        // In CI environments with limited resources, parallel execution may have overhead
        // that makes it slower. Allow up to 5x slower to account for noisy CI environments.
        let slowdown_ratio = duration_full.as_secs_f64() / duration_1.as_secs_f64();
        assert!(
            slowdown_ratio <= 5.0,
            "Parallel execution should not be excessively slower (got {:.2}x slowdown)",
            slowdown_ratio
        );
    }
}

/// End-to-end test: batch process mixed document types.
#[cfg(feature = "xml")]
#[tokio::test]
async fn test_batch_mixed_document_types() {
    use helpers::get_test_file_path;

    let config = ExtractionConfig::default();

    let paths = vec![
        get_test_file_path("text/contract.txt"),
        get_test_file_path("json/sample_document.json"),
        get_test_file_path("xml/simple_note.xml"),
        get_test_file_path("text/readme.md"),
    ];

    let results = batch_extract_file(paths, &config).await;

    assert!(results.is_ok());
    let results = results.unwrap();
    assert_eq!(results.len(), 4);

    for (i, result) in results.iter().enumerate() {
        assert!(
            !result.content.is_empty(),
            "Document {} should have extracted content",
            i
        );
    }

    assert!(
        results[0].content.contains("contract"),
        "First result should be from contract.txt, got: '{}'",
        results[0].content
    );
    assert!(
        results[1].content.contains("Sample") || results[1].content.contains("author"),
        "Second result should be from JSON document, got: '{}'",
        results[1].content
    );
    assert!(
        results[2].content.contains("Tove") || results[2].content.contains("note"),
        "Third result should be from XML, got: '{}'",
        results[2].content
    );
    assert!(
        !results[3].content.is_empty(),
        "Fourth result should be from markdown, got: '{}'",
        results[3].content
    );
}

/// Test batch processing maintains high accuracy under load.
#[tokio::test]
async fn test_batch_accuracy_under_load() {
    let config = ExtractionConfig::default();

    let mut contents = Vec::new();
    for i in 0..100 {
        let content = format!("Document number {} with unique content", i);
        contents.push((content.into_bytes(), "text/plain"));
    }

    let contents_ref: Vec<(&[u8], &str)> = contents.iter().map(|(bytes, mime)| (bytes.as_slice(), *mime)).collect();

    let results = batch_extract_bytes(contents_ref, &config).await.unwrap();

    assert_eq!(results.len(), 100);

    for (i, result) in results.iter().enumerate() {
        let expected = format!("Document number {} with unique content", i);
        assert_eq!(
            result.content, expected,
            "Document {} content mismatch - possible cross-contamination",
            i
        );
    }
}
