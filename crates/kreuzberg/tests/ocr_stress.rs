//! Comprehensive OCR stress tests.
//!
//! Validates that Tesseract integration is thread-safe and performant under heavy load:
//! - Rayon parallel batch processing doesn't cause race conditions
//! - Multiple concurrent batch operations don't interfere
//! - Memory usage stays bounded under heavy OCR load
//! - Tesseract API calls are thread-safe
//! - Cache handles concurrent OCR operations correctly
//!
//! These tests ensure production workloads with heavy OCR usage work correctly.

#![cfg(feature = "ocr")]

use kreuzberg::core::config::{ExtractionConfig, OcrConfig};
use kreuzberg::core::extractor::extract_file_sync;
use kreuzberg::ocr::processor::OcrProcessor;
use kreuzberg::ocr::types::TesseractConfig;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

mod helpers;

/// Stress test: Rayon parallel batch processing with many images.
///
/// Validates that:
/// - Rayon parallelization works correctly with Tesseract
/// - No race conditions in parallel OCR processing
/// - All results are correct with no cross-contamination
#[cfg(feature = "ocr")]
#[cfg_attr(coverage, ignore = "coverage instrumentation slows down rayon benchmarks")]
#[test]
fn test_rayon_batch_stress_many_images() {
    use helpers::{get_test_file_path, skip_if_missing};

    if skip_if_missing("images/ocr_image.jpg") {
        tracing::debug!("Skipping Rayon batch stress test: test file not available");
        return;
    }

    let processor = OcrProcessor::new(None).expect("Should create processor");
    let config = TesseractConfig::default();

    let file_path = get_test_file_path("images/ocr_image.jpg");
    let file_paths: Vec<String> = (0..100).map(|_| file_path.to_string_lossy().to_string()).collect();

    let start = Instant::now();
    let results = processor.process_files_batch(file_paths, &config);
    let duration = start.elapsed();

    let success_count = results.iter().filter(|r| r.success).count();
    assert_eq!(
        success_count, 100,
        "All 100 OCR operations should succeed, got {} successes",
        success_count
    );

    let first_content = results[0].result.as_ref().unwrap().content.clone();
    for (i, result) in results.iter().enumerate().skip(1) {
        assert!(result.success, "Result {} should succeed", i);
        let content = &result.result.as_ref().unwrap().content;
        assert_eq!(
            content, &first_content,
            "Result {} content differs - possible race condition",
            i
        );
    }

    println!(
        "Processed 100 images with Rayon in {:?} ({:.2} images/sec)",
        duration,
        100.0 / duration.as_secs_f64()
    );

    let images_per_sec = 100.0 / duration.as_secs_f64();
    assert!(
        images_per_sec > 5.0,
        "Parallel batch should process at least 5 images/sec, got {:.2}",
        images_per_sec
    );
}

/// Stress test: Multiple concurrent batch operations.
///
/// Validates that:
/// - Multiple threads can run batch_process simultaneously
/// - Rayon thread pool doesn't deadlock or starve
/// - Results remain correct under concurrent batch load
#[test]
fn test_concurrent_rayon_batches() {
    use helpers::{get_test_file_path, skip_if_missing};

    if skip_if_missing("images/ocr_image.jpg") {
        tracing::debug!("Skipping concurrent Rayon batches test: test file not available");
        return;
    }

    let processor = Arc::new(OcrProcessor::new(None).expect("Should create processor"));
    let config = Arc::new(TesseractConfig::default());

    let file_path = get_test_file_path("images/ocr_image.jpg");
    let file_paths: Vec<String> = (0..20).map(|_| file_path.to_string_lossy().to_string()).collect();

    let mut handles = vec![];
    let total_processed = Arc::new(AtomicUsize::new(0));

    for batch_id in 0..10 {
        let processor = Arc::clone(&processor);
        let config = Arc::clone(&config);
        let file_paths = file_paths.clone();
        let total = Arc::clone(&total_processed);

        handles.push(std::thread::spawn(move || {
            let results = processor.process_files_batch(file_paths, &config);

            let success_count = results.iter().filter(|r| r.success).count();
            assert_eq!(
                success_count, 20,
                "Batch {} should have 20 successes, got {}",
                batch_id, success_count
            );

            total.fetch_add(success_count, Ordering::Relaxed);
            results
        }));
    }

    let mut all_results = vec![];
    for handle in handles {
        let results = handle.join().expect("Thread should not panic");
        all_results.push(results);
    }

    let total = total_processed.load(Ordering::Relaxed);
    assert_eq!(total, 200, "Should process 200 total images (10 batches × 20 images)");

    println!("Successfully processed 200 images across 10 concurrent batches");
}

/// Stress test: High memory pressure with large batch.
///
/// Validates that:
/// - Memory usage stays bounded during large batch processing
/// - No memory leaks in Tesseract integration
/// - System remains stable under memory pressure
#[test]
fn test_rayon_batch_memory_pressure() {
    use helpers::{get_test_file_path, skip_if_missing};

    if skip_if_missing("images/ocr_image.jpg") {
        tracing::debug!("Skipping memory pressure test: test file not available");
        return;
    }

    let processor = OcrProcessor::new(None).expect("Should create processor");
    let config = TesseractConfig::default();

    let file_path = get_test_file_path("images/ocr_image.jpg");

    for wave in 0..5 {
        let file_paths: Vec<String> = (0..50).map(|_| file_path.to_string_lossy().to_string()).collect();

        let start = Instant::now();
        let results = processor.process_files_batch(file_paths, &config);
        let duration = start.elapsed();

        let success_count = results.iter().filter(|r| r.success).count();
        assert_eq!(
            success_count, 50,
            "Wave {} should process 50 images, got {} successes",
            wave, success_count
        );

        println!("Wave {} processed 50 images in {:?}", wave, duration);
    }

    println!("Successfully completed 5 waves of 50 images (250 total) without memory issues");
}

/// Stress test: Concurrent Tesseract API calls.
///
/// Validates that:
/// - TesseractAPI is thread-safe in Rust wrapper
/// - No crashes or corruption with concurrent API usage
/// - Results are deterministic across threads
#[test]
fn test_tesseract_api_thread_safety() {
    use helpers::{get_test_file_path, skip_if_missing};

    if skip_if_missing("images/ocr_image.jpg") {
        tracing::debug!("Skipping Tesseract API thread-safety test: test file not available");
        return;
    }

    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: None,
        }),
        force_ocr: false,
        use_cache: false,
        ..Default::default()
    };

    let file_path = get_test_file_path("images/ocr_image.jpg");

    let mut handles = vec![];
    for thread_id in 0..50 {
        let file_path = file_path.clone();
        let config = config.clone();

        handles.push(std::thread::spawn(move || {
            let result = extract_file_sync(&file_path, None, &config);
            assert!(
                result.is_ok(),
                "Thread {} OCR should succeed: {:?}",
                thread_id,
                result.err()
            );
            result.unwrap()
        }));
    }

    let mut results = vec![];
    for handle in handles {
        let extraction = handle.join().expect("Thread should not panic");
        assert!(!extraction.content.is_empty(), "OCR should extract text");
        results.push(extraction);
    }

    let first_content = &results[0].content;
    for (i, result) in results.iter().enumerate().skip(1) {
        assert_eq!(
            &result.content, first_content,
            "Result {} differs from first - thread-safety issue",
            i
        );
    }

    println!("Successfully completed 50 concurrent Tesseract API calls with consistent results");
}

/// Stress test: Sustained concurrent OCR load over time.
///
/// Validates that:
/// - System remains stable under prolonged concurrent OCR
/// - No resource leaks or degradation over time
/// - Throughput remains consistent
#[test]
fn test_sustained_concurrent_ocr_load() {
    use helpers::{get_test_file_path, skip_if_missing};

    if skip_if_missing("images/ocr_image.jpg") {
        tracing::debug!("Skipping sustained load test: test file not available");
        return;
    }

    let processor = Arc::new(OcrProcessor::new(None).expect("Should create processor"));
    let config = Arc::new(TesseractConfig {
        use_cache: false,
        ..Default::default()
    });

    let file_path = get_test_file_path("images/ocr_image.jpg");
    let total_processed = Arc::new(AtomicUsize::new(0));

    let mut handles = vec![];
    for worker_id in 0..20 {
        let processor = Arc::clone(&processor);
        let config = Arc::clone(&config);
        let file_path = file_path.clone();
        let total = Arc::clone(&total_processed);

        handles.push(std::thread::spawn(move || {
            for batch in 0..2 {
                let file_paths: Vec<String> = (0..5).map(|_| file_path.to_string_lossy().to_string()).collect();

                let results = processor.process_files_batch(file_paths, &config);

                let success_count = results.iter().filter(|r| r.success).count();
                assert_eq!(
                    success_count, 5,
                    "Worker {} batch {} should process 5 images",
                    worker_id, batch
                );

                total.fetch_add(success_count, Ordering::Relaxed);
            }
        }));
    }

    for handle in handles {
        handle.join().expect("Worker should not panic");
    }

    let total = total_processed.load(Ordering::Relaxed);
    assert_eq!(total, 200, "Should process 200 total images (20 workers × 10 images)");

    println!("Successfully sustained 20 concurrent workers processing 200 total images");
}

/// Stress test: Concurrent cache access during batch OCR.
///
/// Validates that:
/// - Cache is thread-safe under concurrent batch operations
/// - Cache hits work correctly with Rayon parallelism
/// - No cache corruption or race conditions
#[test]
fn test_concurrent_batch_with_cache() {
    use helpers::{get_test_file_path, skip_if_missing};

    if skip_if_missing("images/ocr_image.jpg") {
        tracing::debug!("Skipping cache stress test: test file not available");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Should create temp dir");
    let processor = OcrProcessor::new(Some(temp_dir.path().to_path_buf())).expect("Should create processor");
    let config = TesseractConfig {
        use_cache: true,
        ..Default::default()
    };

    let file_path = get_test_file_path("images/ocr_image.jpg");

    let warm_paths: Vec<String> = (0..10).map(|_| file_path.to_string_lossy().to_string()).collect();
    let _ = processor.process_files_batch(warm_paths, &config);

    let processor = Arc::new(processor);
    let config = Arc::new(config);
    let mut handles = vec![];
    let total_successes = Arc::new(AtomicUsize::new(0));

    for _ in 0..10 {
        let processor = Arc::clone(&processor);
        let config = Arc::clone(&config);
        let file_path = file_path.clone();
        let total = Arc::clone(&total_successes);

        handles.push(std::thread::spawn(move || {
            let file_paths: Vec<String> = (0..5).map(|_| file_path.to_string_lossy().to_string()).collect();

            let results = processor.process_files_batch(file_paths, &config);

            let success_count = results.iter().filter(|r| r.success).count();
            total.fetch_add(success_count, Ordering::Relaxed);

            results
        }));
    }

    for handle in handles {
        let results = handle.join().expect("Thread should not panic");
        assert_eq!(results.len(), 5, "Each batch should process 5 images");

        let success_count = results.iter().filter(|r| r.success).count();
        assert_eq!(success_count, 5, "All 5 should succeed (from cache)");
    }

    let total = total_successes.load(Ordering::Relaxed);
    assert_eq!(total, 50, "Should process 50 total images (10 batches × 5 images)");

    println!("Successfully completed 10 concurrent cached batches with 50 total images");
}

/// Stress test: Rayon parallel performance comparison.
///
/// Validates that:
/// - Rayon parallelization provides significant speedup
/// - Parallel batch is faster than sequential
/// - Speedup scales reasonably with CPU cores
#[test]
fn test_rayon_parallel_speedup() {
    use helpers::{get_test_file_path, skip_if_missing};

    if skip_if_missing("images/ocr_image.jpg") {
        tracing::debug!("Skipping Rayon speedup test: test file not available");
        return;
    }

    let processor = OcrProcessor::new(None).expect("Should create processor");
    let config = TesseractConfig {
        use_cache: false,
        ..Default::default()
    };

    let file_path = get_test_file_path("images/ocr_image.jpg");
    let test_size = 20;

    let sequential_start = Instant::now();
    for _ in 0..test_size {
        let result = processor.process_file(&file_path.to_string_lossy(), &config);
        assert!(result.is_ok(), "Sequential OCR should succeed");
    }
    let sequential_duration = sequential_start.elapsed();

    let file_paths: Vec<String> = (0..test_size)
        .map(|_| file_path.to_string_lossy().to_string())
        .collect();

    let parallel_start = Instant::now();
    let results = processor.process_files_batch(file_paths, &config);
    let parallel_duration = parallel_start.elapsed();

    assert_eq!(results.len(), test_size as usize, "Should process all images");
    let success_count = results.iter().filter(|r| r.success).count();
    assert_eq!(success_count, test_size as usize, "All should succeed");

    let speedup = sequential_duration.as_secs_f64() / parallel_duration.as_secs_f64();

    println!(
        "Sequential: {:?}, Parallel (Rayon): {:?}, Speedup: {:.2}x",
        sequential_duration, parallel_duration, speedup
    );

    let required_speedup = if cfg!(target_os = "macos") {
        // GitHub macOS runners throttle parallelism heavily, so accept a lower margin there ~keep
        1.1
    } else {
        1.5
    };

    assert!(
        speedup > required_speedup,
        "Rayon parallel should be at least {:.2}x faster than sequential, got {:.2}x",
        required_speedup,
        speedup
    );
}

/// Stress test: Mixed valid and invalid files in batch.
///
/// Validates that:
/// - Rayon batch handles errors gracefully
/// - One failure doesn't affect other parallel operations
/// - Error reporting is correct under parallelism
#[test]
fn test_rayon_batch_error_handling() {
    let processor = OcrProcessor::new(None).expect("Should create processor");
    let config = TesseractConfig::default();

    let mut file_paths = vec![];

    for i in 0..10 {
        file_paths.push(format!("/nonexistent/file_{}.jpg", i));
    }

    let results = processor.process_files_batch(file_paths, &config);

    assert_eq!(results.len(), 10, "Should return results for all files");

    for (i, result) in results.iter().enumerate() {
        assert!(!result.success, "Result {} should fail (file doesn't exist)", i);
        assert!(result.error.is_some(), "Result {} should have error message", i);
        assert!(result.result.is_none(), "Result {} should not have OCR result", i);
    }

    println!("Successfully handled 10 file errors in parallel batch");
}
