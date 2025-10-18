//! Comprehensive concurrency and parallelism stress tests.
//!
//! Validates that the Kreuzberg core handles concurrent operations correctly:
//! - Parallel extractions don't interfere with each other
//! - OCR processing is thread-safe and efficient
//! - Pipeline processing works correctly under concurrent load
//! - Cache access is safe with multiple readers/writers
//! - Registry access is thread-safe
//!
//! These tests ensure production workloads with high concurrency work correctly.

use async_trait::async_trait;
use kreuzberg::Result;
use kreuzberg::core::config::{ExtractionConfig, OcrConfig, PostProcessorConfig};
use kreuzberg::core::extractor::{batch_extract_bytes, extract_bytes, extract_file_sync};
use kreuzberg::core::pipeline::run_pipeline;
use kreuzberg::plugins::registry::{get_document_extractor_registry, get_post_processor_registry};
use kreuzberg::plugins::{Plugin, PostProcessor, ProcessingStage};
use kreuzberg::types::{ExtractionResult, Metadata};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;
use tokio::time::timeout;

mod helpers;

// ============================================================================
// Concurrent Extraction Tests
// ============================================================================

/// Test many concurrent extractions of different MIME types.
///
/// Validates that:
/// - Registry lookups don't block each other unnecessarily
/// - Different extractors can run in parallel
/// - No data races or corruption
#[tokio::test]
async fn test_concurrent_extractions_mixed_formats() {
    let config = ExtractionConfig::default();

    // Create test data for different formats (all supported)
    let test_cases = vec![
        (b"Plain text content" as &[u8], "text/plain"),
        (b"{\"key\": \"value\"}", "application/json"),
        (b"<root><item>XML content</item></root>", "application/xml"),
        (b"# Markdown\n\nContent here", "text/markdown"),
    ];

    // Spawn 50 concurrent tasks (10 per format)
    let mut handles = vec![];
    for _ in 0..10 {
        for (data, mime_type) in &test_cases {
            let config = config.clone();
            let data = data.to_vec();
            let mime_type = mime_type.to_string();

            handles.push(tokio::spawn(
                async move { extract_bytes(&data, &mime_type, &config).await },
            ));
        }
    }

    // Wait for all with timeout
    let results = timeout(Duration::from_secs(30), async {
        let mut results = vec![];
        for handle in handles {
            results.push(handle.await.expect("Task should not panic"));
        }
        results
    })
    .await
    .expect("All extractions should complete within 30s");

    // Verify all succeeded
    for result in results {
        assert!(
            result.is_ok(),
            "Concurrent extraction should succeed: {:?}",
            result.err()
        );
    }
}

/// Test concurrent batch extractions.
///
/// Validates that batch processing correctly handles parallelism internally.
#[tokio::test]
async fn test_concurrent_batch_extractions() {
    let config = ExtractionConfig::default();

    // Prepare test data as byte slices
    let contents: Vec<Vec<u8>> = (0..20).map(|i| format!("Content {}", i).into_bytes()).collect();

    // Run 5 concurrent batch extractions
    let mut handles = vec![];
    for _ in 0..5 {
        let config = config.clone();
        let contents_clone = contents.clone();

        handles.push(tokio::spawn(async move {
            // Create references for batch_extract_bytes
            let data: Vec<(&[u8], &str)> = contents_clone.iter().map(|c| (c.as_slice(), "text/plain")).collect();
            batch_extract_bytes(data, &config).await
        }));
    }

    // Wait for all
    for handle in handles {
        let results = handle.await.expect("Task should not panic");
        assert!(results.is_ok(), "Batch extraction should succeed");
        let results = results.unwrap();
        assert_eq!(results.len(), 20, "Should return all results");
    }
}

/// Test concurrent extractions with caching enabled.
///
/// Validates that:
/// - Cache reads/writes are thread-safe
/// - No cache corruption under concurrent access
/// - Cache hits work correctly across threads
#[tokio::test]
async fn test_concurrent_extractions_with_cache() {
    let config = ExtractionConfig {
        use_cache: true,
        postprocessor: Some(PostProcessorConfig {
            enabled: false, // Disable to avoid interference from other tests
            enabled_processors: None,
            disabled_processors: None,
        }),
        ..Default::default()
    };

    let test_data = b"Cached content for concurrent access test";

    // First, populate cache
    let _ = extract_bytes(test_data, "text/plain", &config).await.unwrap();

    // Now spawn 100 concurrent reads (should all hit cache)
    let mut handles = vec![];
    for _ in 0..100 {
        let config = config.clone();
        let data = test_data.to_vec();

        handles.push(tokio::spawn(async move {
            extract_bytes(&data, "text/plain", &config).await
        }));
    }

    // All should succeed and return same content
    let expected_content = "Cached content for concurrent access test";
    for handle in handles {
        let result = handle.await.expect("Task should not panic");
        assert!(result.is_ok(), "Cache read should succeed");
        let extraction = result.unwrap();
        assert_eq!(extraction.content, expected_content);
    }
}

// ============================================================================
// Concurrent OCR Tests
// ============================================================================

/// Test concurrent OCR processing of different images.
///
/// Validates that:
/// - OCR backend is thread-safe
/// - Multiple OCR operations don't interfere
/// - OCR cache handles concurrent access correctly
#[cfg(feature = "ocr")]
#[tokio::test]
async fn test_concurrent_ocr_processing() {
    use helpers::{get_test_file_path, skip_if_missing};

    if skip_if_missing("images/ocr_image.jpg") {
        eprintln!("Skipping concurrent OCR test: test file not available");
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

    // Spawn 20 concurrent OCR tasks on the same file
    let mut handles = vec![];
    for _ in 0..20 {
        let file_path = file_path.clone();
        let config = config.clone();

        handles.push(tokio::task::spawn_blocking(move || {
            extract_file_sync(&file_path, None, &config)
        }));
    }

    // Wait for all with timeout
    let results = timeout(Duration::from_secs(60), async {
        let mut results = vec![];
        for handle in handles {
            results.push(handle.await.expect("Task should not panic"));
        }
        results
    })
    .await
    .expect("All OCR operations should complete within 60s");

    // Verify all succeeded and have consistent results
    let mut extracted_texts = vec![];
    for result in results {
        assert!(result.is_ok(), "OCR should succeed: {:?}", result.err());
        let extraction = result.unwrap();
        assert!(!extraction.content.is_empty(), "OCR should extract text");
        extracted_texts.push(extraction.content);
    }

    // All results should be identical (same file, same config)
    let first_text = &extracted_texts[0];
    for text in &extracted_texts[1..] {
        assert_eq!(text, first_text, "Concurrent OCR should produce identical results");
    }
}

/// Test concurrent OCR with cache warming.
///
/// Validates cache performance under concurrent load.
///
/// Note: This test is simplified to avoid runtime nesting issues.
/// It validates that concurrent OCR extractions work correctly with caching.
#[cfg(feature = "ocr")]
#[test]
fn test_concurrent_ocr_cache_stress() {
    use helpers::{get_test_file_path, skip_if_missing};

    if skip_if_missing("images/ocr_image.jpg") {
        eprintln!("Skipping OCR cache stress test: test file not available");
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

    // Warm the cache with first extraction
    let first_result = extract_file_sync(&file_path, None, &config);
    assert!(first_result.is_ok(), "Initial OCR should succeed");

    // Now spawn 50 concurrent threads (should all hit cache)
    let cache_hit_count = Arc::new(AtomicUsize::new(0));

    let mut handles = vec![];
    for _ in 0..50 {
        let file_path = file_path.clone();
        let config = config.clone();
        let hit_count = Arc::clone(&cache_hit_count);

        handles.push(std::thread::spawn(move || {
            let start = std::time::Instant::now();
            let result = extract_file_sync(&file_path, None, &config);
            let duration = start.elapsed();

            // Cache hits should be much faster than actual OCR (<100ms vs >500ms)
            if duration < Duration::from_millis(100) {
                hit_count.fetch_add(1, Ordering::Relaxed);
            }

            result
        }));
    }

    // Wait for all
    for handle in handles {
        let result = handle.join().expect("Thread should not panic");
        assert!(result.is_ok(), "Cached OCR should succeed");
    }

    // Most should be cache hits (allow some misses due to timing)
    let hits = cache_hit_count.load(Ordering::Relaxed);
    assert!(
        hits >= 40,
        "At least 40/50 requests should hit cache, got {} hits",
        hits
    );
}

// ============================================================================
// Concurrent Pipeline Tests
// ============================================================================

/// Test concurrent pipeline processing.
///
/// Validates that:
/// - Pipeline can process multiple results in parallel
/// - Processors don't interfere with each other
/// - Registry reads are thread-safe
#[tokio::test]
async fn test_concurrent_pipeline_processing() {
    // Create a simple processor for testing
    struct ConcurrentTestProcessor;

    impl Plugin for ConcurrentTestProcessor {
        fn name(&self) -> &str {
            "concurrent-test"
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl PostProcessor for ConcurrentTestProcessor {
        async fn process(&self, result: &mut ExtractionResult, _: &ExtractionConfig) -> Result<()> {
            // Simulate some processing work
            tokio::time::sleep(Duration::from_millis(10)).await;
            result.content.push_str("[processed]");
            Ok(())
        }

        fn processing_stage(&self) -> ProcessingStage {
            ProcessingStage::Early
        }
    }

    // Register processor once (production scenario)
    let registry = get_post_processor_registry();
    {
        let mut reg = registry.write().expect("Should acquire write lock");
        let processor = Arc::new(ConcurrentTestProcessor);
        // Remove if already registered (from other tests)
        let _ = reg.remove("concurrent-test");
        reg.register(processor, 50).expect("Should register processor");
    }

    let config = ExtractionConfig {
        postprocessor: Some(PostProcessorConfig {
            enabled: true,
            enabled_processors: Some(vec!["concurrent-test".to_string()]),
            disabled_processors: None,
        }),
        ..Default::default()
    };

    // Spawn 50 concurrent pipeline runs
    let mut handles = vec![];
    for i in 0..50 {
        let config = config.clone();

        handles.push(tokio::spawn(async move {
            let result = ExtractionResult {
                content: format!("Content {}", i),
                mime_type: "text/plain".to_string(),
                metadata: Metadata::default(),
                tables: vec![],
                detected_languages: None,
                chunks: None,
            };

            run_pipeline(result, &config).await
        }));
    }

    // Wait for all
    for handle in handles {
        let result = handle.await.expect("Task should not panic");
        assert!(result.is_ok(), "Pipeline should succeed");
        let processed = result.unwrap();
        assert!(processed.content.contains("[processed]"), "Processor should run");
    }

    // Cleanup
    {
        let mut reg = registry.write().expect("Should acquire write lock");
        let _ = reg.remove("concurrent-test");
    }
}

// ============================================================================
// Concurrent Registry Access Tests
// ============================================================================

/// Test concurrent registry reads don't block unnecessarily.
///
/// Validates that:
/// - Multiple readers can access registry simultaneously
/// - Registry lookups are fast under concurrent load
#[tokio::test]
async fn test_concurrent_registry_reads() {
    let registry = get_document_extractor_registry();

    // Spawn 200 concurrent registry reads
    let mut handles = vec![];
    for _ in 0..200 {
        let registry_clone = Arc::clone(&registry);
        handles.push(tokio::spawn(async move {
            let start = std::time::Instant::now();

            // Perform registry lookup
            let reg = registry_clone.read().expect("Should acquire read lock");
            let _extractor = reg.get("text/plain");

            start.elapsed()
        }));
    }

    // All should complete quickly
    let mut max_duration = Duration::from_secs(0);
    for handle in handles {
        let duration = handle.await.expect("Task should not panic");
        if duration > max_duration {
            max_duration = duration;
        }
    }

    // Registry reads should be very fast (< 10ms even under high concurrency)
    assert!(
        max_duration < Duration::from_millis(10),
        "Registry reads should be fast, max duration: {:?}",
        max_duration
    );
}

/// Test that extraction throughput scales with concurrency.
///
/// Validates that:
/// - Parallel extractions are actually running in parallel
/// - No global bottlenecks limiting throughput
#[tokio::test]
async fn test_extraction_throughput_scales() {
    let config = ExtractionConfig::default();
    let test_data = b"Throughput test content";

    // Measure sequential baseline
    let sequential_start = std::time::Instant::now();
    for _ in 0..20 {
        let _ = extract_bytes(test_data, "text/plain", &config).await.unwrap();
    }
    let sequential_duration = sequential_start.elapsed();

    // Measure parallel throughput
    let parallel_start = std::time::Instant::now();
    let mut handles = vec![];
    for _ in 0..20 {
        let config = config.clone();
        let data = test_data.to_vec();

        handles.push(tokio::spawn(async move {
            extract_bytes(&data, "text/plain", &config).await
        }));
    }

    for handle in handles {
        let _ = handle.await.expect("Task should not panic");
    }
    let parallel_duration = parallel_start.elapsed();

    // Parallel should be at least 2x faster (conservative check)
    // On systems with 4+ cores, should be much faster
    println!(
        "Sequential: {:?}, Parallel: {:?}, Speedup: {:.2}x",
        sequential_duration,
        parallel_duration,
        sequential_duration.as_secs_f64() / parallel_duration.as_secs_f64()
    );

    assert!(
        parallel_duration < sequential_duration / 2,
        "Parallel execution should be at least 2x faster than sequential. Sequential: {:?}, Parallel: {:?}",
        sequential_duration,
        parallel_duration
    );
}

// ============================================================================
// Stress Tests
// ============================================================================

/// High-load stress test with many concurrent operations.
///
/// Validates system stability under sustained concurrent load.
#[tokio::test]
async fn test_high_concurrency_stress() {
    let config = ExtractionConfig {
        use_cache: true,
        ..Default::default()
    };

    // Use only fully supported formats
    let formats = vec![
        (b"Text content" as &[u8], "text/plain"),
        (b"{\"json\": true}", "application/json"),
        (b"<xml><item>content</item></xml>", "application/xml"),
        (b"# Markdown\n\nContent", "text/markdown"),
    ];

    // Spawn 400 concurrent tasks
    let mut handles = vec![];
    for _ in 0..100 {
        for (data, mime_type) in &formats {
            let config = config.clone();
            let data = data.to_vec();
            let mime_type = mime_type.to_string();

            handles.push(tokio::spawn(
                async move { extract_bytes(&data, &mime_type, &config).await },
            ));
        }
    }

    // Should complete within reasonable time (60s for 400 tasks)
    let results = timeout(Duration::from_secs(60), async {
        let mut results = vec![];
        for handle in handles {
            results.push(handle.await.expect("Task should not panic"));
        }
        results
    })
    .await
    .expect("High-load stress test should complete within 60s");

    // Verify all succeeded
    let success_count = results.iter().filter(|r| r.is_ok()).count();
    assert_eq!(
        success_count, 400,
        "All extractions should succeed under stress, got {} successes",
        success_count
    );
}
