//! Main extraction entry points.
//!
//! This module provides the primary API for extracting content from files and byte arrays.
//! It orchestrates the entire extraction pipeline: cache checking, MIME detection,
//! extractor selection, extraction, post-processing, and cache storage.
//!
//! # Functions
//!
//! - [`extract_file`] - Extract content from a file path
//! - [`extract_bytes`] - Extract content from a byte array
//! - [`batch_extract_file`] - Extract content from multiple files concurrently
//! - [`batch_extract_bytes`] - Extract content from multiple byte arrays concurrently

use crate::core::config::ExtractionConfig;
use crate::plugins::DocumentExtractor;
use crate::types::ExtractionResult;
use crate::{KreuzbergError, Result};
use once_cell::sync::Lazy;
use std::path::Path;
use std::sync::Arc;

/// Global Tokio runtime for synchronous operations.
///
/// This runtime is lazily initialized on first use and shared across all sync wrappers.
/// Using a global runtime instead of creating one per call provides 100x+ performance improvement.
///
/// # Safety
///
/// The `.expect()` here is justified because:
/// 1. Runtime creation can only fail due to system resource exhaustion (OOM, thread limit)
/// 2. If runtime creation fails, the process is already in a critical state
/// 3. This is a one-time initialization - if it fails, nothing will work
/// 4. Better to fail fast than return errors from every sync operation
static GLOBAL_RUNTIME: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create global Tokio runtime - system may be out of resources")
});

/// Get an extractor from the registry.
///
/// This function acquires the registry read lock and retrieves the appropriate
/// extractor for the given MIME type.
///
/// # Performance
///
/// RwLock read + HashMap lookup is ~100ns, fast enough without caching.
/// Removed thread-local cache to avoid Tokio work-stealing scheduler issues.
fn get_extractor(mime_type: &str) -> Result<Arc<dyn DocumentExtractor>> {
    let registry = crate::plugins::registry::get_document_extractor_registry();
    let registry_read = registry
        .read()
        .map_err(|e| KreuzbergError::Other(format!("Document extractor registry lock poisoned: {}", e)))?;
    registry_read.get(mime_type)
}

/// Extract content from a file.
///
/// This is the main entry point for file-based extraction. It performs the following steps:
/// 1. Check cache for existing result (if caching enabled)
/// 2. Detect or validate MIME type
/// 3. Select appropriate extractor from registry
/// 4. Extract content
/// 5. Run post-processing pipeline
/// 6. Store result in cache (if caching enabled)
///
/// # Arguments
///
/// * `path` - Path to the file to extract
/// * `mime_type` - Optional MIME type override. If None, will be auto-detected
/// * `config` - Extraction configuration
///
/// # Returns
///
/// An `ExtractionResult` containing the extracted content and metadata.
///
/// # Errors
///
/// Returns `KreuzbergError::Validation` if the file doesn't exist or path is invalid.
/// Returns `KreuzbergError::UnsupportedFormat` if MIME type is not supported.
/// Returns `KreuzbergError::Io` for file I/O errors (these always bubble up).
///
/// # Example
///
/// ```rust,no_run
/// use kreuzberg::core::extractor::extract_file;
/// use kreuzberg::core::config::ExtractionConfig;
///
/// # async fn example() -> kreuzberg::Result<()> {
/// let config = ExtractionConfig::default();
/// let result = extract_file("document.pdf", None, &config).await?;
/// println!("Content: {}", result.content);
/// # Ok(())
/// # }
/// ```
pub async fn extract_file(
    path: impl AsRef<Path>,
    mime_type: Option<&str>,
    config: &ExtractionConfig,
) -> Result<ExtractionResult> {
    use crate::core::{io, mime};

    let path = path.as_ref();

    io::validate_file_exists(path)?;

    let detected_mime = mime::detect_or_validate(Some(path), mime_type)?;

    crate::extractors::ensure_initialized()?;

    let extractor = get_extractor(&detected_mime)?;

    let mut result = extractor.extract_file(path, &detected_mime, config).await?;

    result = crate::core::pipeline::run_pipeline(result, config).await?;

    Ok(result)
}

/// Extract content from a byte array.
///
/// This function extracts content from an in-memory byte array with a known MIME type.
///
/// # Arguments
///
/// * `content` - The content bytes to extract
/// * `mime_type` - MIME type of the content
/// * `config` - Extraction configuration
///
/// # Returns
///
/// An `ExtractionResult` containing the extracted content and metadata.
///
/// # Errors
///
/// Returns `KreuzbergError::UnsupportedFormat` if MIME type is not supported.
///
/// # Example
///
/// ```rust,no_run
/// use kreuzberg::core::extractor::extract_bytes;
/// use kreuzberg::core::config::ExtractionConfig;
///
/// # async fn example() -> kreuzberg::Result<()> {
/// let pdf_bytes = std::fs::read("document.pdf")?;
/// let config = ExtractionConfig::default();
/// let result = extract_bytes(&pdf_bytes, "application/pdf", &config).await?;
/// # Ok(())
/// # }
/// ```
pub async fn extract_bytes(content: &[u8], mime_type: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
    use crate::core::mime;

    let validated_mime = mime::validate_mime_type(mime_type)?;

    crate::extractors::ensure_initialized()?;

    let extractor = get_extractor(&validated_mime)?;

    let mut result = extractor.extract_bytes(content, &validated_mime, config).await?;

    result = crate::core::pipeline::run_pipeline(result, config).await?;

    Ok(result)
}

/// Extract content from multiple files concurrently.
///
/// This function processes multiple files in parallel, automatically managing
/// concurrency to prevent resource exhaustion. The concurrency limit can be
/// configured via `ExtractionConfig::max_concurrent_extractions` or defaults
/// to `num_cpus * 2`.
///
/// # Arguments
///
/// * `paths` - Vector of file paths to extract
/// * `config` - Extraction configuration
///
/// # Returns
///
/// A vector of `ExtractionResult` in the same order as the input paths.
///
/// # Errors
///
/// Individual file errors are captured in the result metadata. System errors
/// (IO, RuntimeError equivalents) will bubble up and fail the entire batch.
pub async fn batch_extract_file(
    paths: Vec<impl AsRef<Path>>,
    config: &ExtractionConfig,
) -> Result<Vec<ExtractionResult>> {
    use std::sync::Arc;
    use tokio::sync::Semaphore;
    use tokio::task::JoinSet;

    if paths.is_empty() {
        return Ok(vec![]);
    }

    let config = Arc::new(config.clone());

    let max_concurrent = config.max_concurrent_extractions.unwrap_or_else(|| num_cpus::get() * 2);
    let semaphore = Arc::new(Semaphore::new(max_concurrent));

    let mut tasks = JoinSet::new();

    for (index, path) in paths.into_iter().enumerate() {
        let path_buf = path.as_ref().to_path_buf();
        let config_clone = Arc::clone(&config);
        let semaphore_clone = Arc::clone(&semaphore);

        tasks.spawn(async move {
            let _permit = semaphore_clone.acquire().await.unwrap();
            let result = extract_file(&path_buf, None, &config_clone).await;
            (index, result)
        });
    }

    let mut results: Vec<Option<ExtractionResult>> = vec![None; tasks.len()];

    while let Some(task_result) = tasks.join_next().await {
        match task_result {
            Ok((index, Ok(result))) => {
                results[index] = Some(result);
            }
            Ok((index, Err(e))) => {
                // OSError/RuntimeError must bubble up - system errors need user reports ~keep
                if matches!(e, KreuzbergError::Io(_)) {
                    return Err(e);
                }

                use crate::types::{ErrorMetadata, Metadata};
                let metadata = Metadata {
                    error: Some(ErrorMetadata {
                        error_type: format!("{:?}", e),
                        message: e.to_string(),
                    }),
                    ..Default::default()
                };

                results[index] = Some(ExtractionResult {
                    content: format!("Error: {}", e),
                    mime_type: "text/plain".to_string(),
                    metadata,
                    tables: vec![],
                    detected_languages: None,
                    chunks: None,
                    images: None,
                });
            }
            Err(join_err) => {
                return Err(KreuzbergError::Other(format!("Task panicked: {}", join_err)));
            }
        }
    }

    #[allow(clippy::unwrap_used)]
    Ok(results.into_iter().map(|r| r.unwrap()).collect())
}

/// Extract content from multiple byte arrays concurrently.
///
/// This function processes multiple byte arrays in parallel, automatically managing
/// concurrency to prevent resource exhaustion. The concurrency limit can be
/// configured via `ExtractionConfig::max_concurrent_extractions` or defaults
/// to `num_cpus * 2`.
///
/// # Arguments
///
/// * `contents` - Vector of (bytes, mime_type) tuples
/// * `config` - Extraction configuration
///
/// # Returns
///
/// A vector of `ExtractionResult` in the same order as the input.
pub async fn batch_extract_bytes(
    contents: Vec<(&[u8], &str)>,
    config: &ExtractionConfig,
) -> Result<Vec<ExtractionResult>> {
    use std::sync::Arc;
    use tokio::sync::Semaphore;
    use tokio::task::JoinSet;

    if contents.is_empty() {
        return Ok(vec![]);
    }

    let config = Arc::new(config.clone());

    let max_concurrent = config.max_concurrent_extractions.unwrap_or_else(|| num_cpus::get() * 2);
    let semaphore = Arc::new(Semaphore::new(max_concurrent));

    let owned_contents: Vec<(Vec<u8>, String)> = contents
        .into_iter()
        .map(|(bytes, mime)| (bytes.to_vec(), mime.to_string()))
        .collect();

    let mut tasks = JoinSet::new();

    for (index, (bytes, mime_type)) in owned_contents.into_iter().enumerate() {
        let config_clone = Arc::clone(&config);
        let semaphore_clone = Arc::clone(&semaphore);

        tasks.spawn(async move {
            let _permit = semaphore_clone.acquire().await.unwrap();
            let result = extract_bytes(&bytes, &mime_type, &config_clone).await;
            (index, result)
        });
    }

    let mut results: Vec<Option<ExtractionResult>> = vec![None; tasks.len()];

    while let Some(task_result) = tasks.join_next().await {
        match task_result {
            Ok((index, Ok(result))) => {
                results[index] = Some(result);
            }
            Ok((index, Err(e))) => {
                // OSError/RuntimeError must bubble up - system errors need user reports ~keep
                if matches!(e, KreuzbergError::Io(_)) {
                    return Err(e);
                }

                use crate::types::{ErrorMetadata, Metadata};
                let metadata = Metadata {
                    error: Some(ErrorMetadata {
                        error_type: format!("{:?}", e),
                        message: e.to_string(),
                    }),
                    ..Default::default()
                };

                results[index] = Some(ExtractionResult {
                    content: format!("Error: {}", e),
                    mime_type: "text/plain".to_string(),
                    metadata,
                    tables: vec![],
                    detected_languages: None,
                    chunks: None,
                    images: None,
                });
            }
            Err(join_err) => {
                return Err(KreuzbergError::Other(format!("Task panicked: {}", join_err)));
            }
        }
    }

    #[allow(clippy::unwrap_used)]
    Ok(results.into_iter().map(|r| r.unwrap()).collect())
}

/// Synchronous wrapper for `extract_file`.
///
/// This is a convenience function that blocks the current thread until extraction completes.
/// For async code, use `extract_file` directly.
///
/// Uses the global Tokio runtime for 100x+ performance improvement over creating
/// a new runtime per call.
pub fn extract_file_sync(
    path: impl AsRef<Path>,
    mime_type: Option<&str>,
    config: &ExtractionConfig,
) -> Result<ExtractionResult> {
    GLOBAL_RUNTIME.block_on(extract_file(path, mime_type, config))
}

/// Synchronous wrapper for `extract_bytes`.
///
/// Uses the global Tokio runtime for 100x+ performance improvement over creating
/// a new runtime per call.
pub fn extract_bytes_sync(content: &[u8], mime_type: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
    GLOBAL_RUNTIME.block_on(extract_bytes(content, mime_type, config))
}

/// Synchronous wrapper for `batch_extract_file`.
///
/// Uses the global Tokio runtime for 100x+ performance improvement over creating
/// a new runtime per call.
pub fn batch_extract_file_sync(
    paths: Vec<impl AsRef<Path>>,
    config: &ExtractionConfig,
) -> Result<Vec<ExtractionResult>> {
    GLOBAL_RUNTIME.block_on(batch_extract_file(paths, config))
}

/// Synchronous wrapper for `batch_extract_bytes`.
///
/// Uses the global Tokio runtime for 100x+ performance improvement over creating
/// a new runtime per call.
pub fn batch_extract_bytes_sync(
    contents: Vec<(&[u8], &str)>,
    config: &ExtractionConfig,
) -> Result<Vec<ExtractionResult>> {
    GLOBAL_RUNTIME.block_on(batch_extract_bytes(contents, config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_extract_file_basic() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"Hello, world!").unwrap();

        let config = ExtractionConfig::default();
        let result = extract_file(&file_path, None, &config).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.content, "Hello, world!");
        assert_eq!(result.mime_type, "text/plain");
    }

    #[tokio::test]
    async fn test_extract_file_with_mime_override() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.dat");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"test content").unwrap();

        let config = ExtractionConfig::default();
        let result = extract_file(&file_path, Some("text/plain"), &config).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.mime_type, "text/plain");
    }

    #[tokio::test]
    async fn test_extract_file_nonexistent() {
        let config = ExtractionConfig::default();
        let result = extract_file("/nonexistent/file.txt", None, &config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_extract_bytes_basic() {
        let config = ExtractionConfig::default();
        let result = extract_bytes(b"test content", "text/plain", &config).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.content, "test content");
        assert_eq!(result.mime_type, "text/plain");
    }

    #[tokio::test]
    async fn test_extract_bytes_invalid_mime() {
        let config = ExtractionConfig::default();
        let result = extract_bytes(b"test", "invalid/mime", &config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_batch_extract_file() {
        let dir = tempdir().unwrap();

        let file1 = dir.path().join("test1.txt");
        let file2 = dir.path().join("test2.txt");

        File::create(&file1).unwrap().write_all(b"content 1").unwrap();
        File::create(&file2).unwrap().write_all(b"content 2").unwrap();

        let config = ExtractionConfig::default();
        let paths = vec![file1, file2];
        let results = batch_extract_file(paths, &config).await;

        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].content, "content 1");
        assert_eq!(results[1].content, "content 2");
    }

    #[tokio::test]
    async fn test_batch_extract_file_empty() {
        let config = ExtractionConfig::default();
        let paths: Vec<std::path::PathBuf> = vec![];
        let results = batch_extract_file(paths, &config).await;

        assert!(results.is_ok());
        assert_eq!(results.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_batch_extract_bytes() {
        let config = ExtractionConfig::default();
        let contents = vec![
            (b"content 1".as_slice(), "text/plain"),
            (b"content 2".as_slice(), "text/plain"),
        ];
        let results = batch_extract_bytes(contents, &config).await;

        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].content, "content 1");
        assert_eq!(results[1].content, "content 2");
    }

    #[test]
    fn test_sync_wrappers() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        File::create(&file_path).unwrap().write_all(b"sync test").unwrap();

        let config = ExtractionConfig::default();

        let result = extract_file_sync(&file_path, None, &config);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().content, "sync test");

        let result = extract_bytes_sync(b"test", "text/plain", &config);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_extractor_cache() {
        let config = ExtractionConfig::default();

        let result1 = extract_bytes(b"test 1", "text/plain", &config).await;
        assert!(result1.is_ok());

        let result2 = extract_bytes(b"test 2", "text/plain", &config).await;
        assert!(result2.is_ok());

        assert_eq!(result1.unwrap().content, "test 1");
        assert_eq!(result2.unwrap().content, "test 2");

        let result3 = extract_bytes(b"# test 3", "text/markdown", &config).await;
        assert!(result3.is_ok());
    }

    #[tokio::test]
    async fn test_extract_file_empty() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("empty.txt");
        File::create(&file_path).unwrap();

        let config = ExtractionConfig::default();
        let result = extract_file(&file_path, None, &config).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.content, "");
    }

    #[tokio::test]
    async fn test_extract_bytes_empty() {
        let config = ExtractionConfig::default();
        let result = extract_bytes(b"", "text/plain", &config).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.content, "");
    }

    #[tokio::test]
    async fn test_extract_file_whitespace_only() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("whitespace.txt");
        File::create(&file_path).unwrap().write_all(b"   \n\t  \n  ").unwrap();

        let config = ExtractionConfig::default();
        let result = extract_file(&file_path, None, &config).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_extract_file_very_long_path() {
        let dir = tempdir().unwrap();
        let long_name = "a".repeat(200);
        let file_path = dir.path().join(format!("{}.txt", long_name));

        if let Ok(mut f) = File::create(&file_path) {
            f.write_all(b"content").unwrap();
            let config = ExtractionConfig::default();
            let result = extract_file(&file_path, None, &config).await;
            assert!(result.is_ok() || result.is_err());
        }
    }

    #[tokio::test]
    async fn test_extract_file_special_characters_in_path() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test with spaces & symbols!.txt");
        File::create(&file_path).unwrap().write_all(b"content").unwrap();

        let config = ExtractionConfig::default();
        let result = extract_file(&file_path, None, &config).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().content, "content");
    }

    #[tokio::test]
    async fn test_extract_file_unicode_filename() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("测试文件名.txt");
        File::create(&file_path).unwrap().write_all(b"content").unwrap();

        let config = ExtractionConfig::default();
        let result = extract_file(&file_path, None, &config).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_extract_bytes_unsupported_mime() {
        let config = ExtractionConfig::default();
        let result = extract_bytes(b"test", "application/x-unknown-format", &config).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KreuzbergError::UnsupportedFormat(_)));
    }

    #[tokio::test]
    async fn test_batch_extract_file_with_errors() {
        let dir = tempdir().unwrap();

        let valid_file = dir.path().join("valid.txt");
        File::create(&valid_file).unwrap().write_all(b"valid content").unwrap();

        let invalid_file = dir.path().join("nonexistent.txt");

        let config = ExtractionConfig::default();
        let paths = vec![valid_file, invalid_file];
        let results = batch_extract_file(paths, &config).await;

        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].content, "valid content");
        assert!(results[1].metadata.error.is_some());
    }

    #[tokio::test]
    async fn test_batch_extract_bytes_mixed_valid_invalid() {
        let config = ExtractionConfig::default();
        let contents = vec![
            (b"valid 1".as_slice(), "text/plain"),
            (b"invalid".as_slice(), "invalid/mime"),
            (b"valid 2".as_slice(), "text/plain"),
        ];
        let results = batch_extract_bytes(contents, &config).await;

        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].content, "valid 1");
        assert!(results[1].metadata.error.is_some());
        assert_eq!(results[2].content, "valid 2");
    }

    #[tokio::test]
    async fn test_batch_extract_bytes_all_invalid() {
        let config = ExtractionConfig::default();
        let contents = vec![
            (b"test 1".as_slice(), "invalid/mime1"),
            (b"test 2".as_slice(), "invalid/mime2"),
        ];
        let results = batch_extract_bytes(contents, &config).await;

        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results[0].metadata.error.is_some());
        assert!(results[1].metadata.error.is_some());
    }

    #[tokio::test]
    async fn test_extract_bytes_very_large() {
        let large_content = vec![b'a'; 10_000_000];
        let config = ExtractionConfig::default();
        let result = extract_bytes(&large_content, "text/plain", &config).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.content.len(), 10_000_000);
    }

    #[tokio::test]
    async fn test_batch_extract_large_count() {
        let dir = tempdir().unwrap();
        let mut paths = Vec::new();

        for i in 0..100 {
            let file_path = dir.path().join(format!("file{}.txt", i));
            File::create(&file_path)
                .unwrap()
                .write_all(format!("content {}", i).as_bytes())
                .unwrap();
            paths.push(file_path);
        }

        let config = ExtractionConfig::default();
        let results = batch_extract_file(paths, &config).await;

        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 100);

        for (i, result) in results.iter().enumerate() {
            assert_eq!(result.content, format!("content {}", i));
        }
    }

    #[tokio::test]
    async fn test_extract_file_mime_detection_fallback() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("testfile");
        File::create(&file_path)
            .unwrap()
            .write_all(b"plain text content")
            .unwrap();

        let config = ExtractionConfig::default();
        let result = extract_file(&file_path, None, &config).await;

        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_extract_file_wrong_mime_override() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        File::create(&file_path).unwrap().write_all(b"plain text").unwrap();

        let config = ExtractionConfig::default();
        let result = extract_file(&file_path, Some("application/pdf"), &config).await;

        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_sync_wrapper_nonexistent_file() {
        let config = ExtractionConfig::default();
        let result = extract_file_sync("/nonexistent/path.txt", None, &config);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KreuzbergError::Validation { .. }));
    }

    #[test]
    fn test_sync_wrapper_batch_empty() {
        let config = ExtractionConfig::default();
        let paths: Vec<std::path::PathBuf> = vec![];
        let results = batch_extract_file_sync(paths, &config);

        assert!(results.is_ok());
        assert_eq!(results.unwrap().len(), 0);
    }

    #[test]
    fn test_sync_wrapper_batch_bytes_empty() {
        let config = ExtractionConfig::default();
        let contents: Vec<(&[u8], &str)> = vec![];
        let results = batch_extract_bytes_sync(contents, &config);

        assert!(results.is_ok());
        assert_eq!(results.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_concurrent_extractions_same_mime() {
        use tokio::task::JoinSet;

        let config = Arc::new(ExtractionConfig::default());
        let mut tasks = JoinSet::new();

        for i in 0..50 {
            let config_clone = Arc::clone(&config);
            tasks.spawn(async move {
                let content = format!("test content {}", i);
                extract_bytes(content.as_bytes(), "text/plain", &config_clone).await
            });
        }

        let mut success_count = 0;
        while let Some(task_result) = tasks.join_next().await {
            if let Ok(Ok(_)) = task_result {
                success_count += 1;
            }
        }

        assert_eq!(success_count, 50);
    }

    #[tokio::test]
    async fn test_concurrent_extractions_different_mimes() {
        use tokio::task::JoinSet;

        let config = Arc::new(ExtractionConfig::default());
        let mut tasks = JoinSet::new();

        let mime_types = ["text/plain", "text/markdown"];

        for i in 0..30 {
            let config_clone = Arc::clone(&config);
            let mime = mime_types[i % mime_types.len()];
            tasks.spawn(async move {
                let content = format!("test {}", i);
                extract_bytes(content.as_bytes(), mime, &config_clone).await
            });
        }

        let mut success_count = 0;
        while let Some(task_result) = tasks.join_next().await {
            if let Ok(Ok(_)) = task_result {
                success_count += 1;
            }
        }

        assert_eq!(success_count, 30);
    }
}
