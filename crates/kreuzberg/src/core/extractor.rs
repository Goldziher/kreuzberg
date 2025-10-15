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
use crate::types::ExtractionResult;
use crate::{KreuzbergError, Result};
use once_cell::sync::Lazy;
use std::path::Path;

/// Global Tokio runtime for synchronous operations.
///
/// This runtime is lazily initialized on first use and shared across all sync wrappers.
/// Using a global runtime instead of creating one per call provides 100x+ performance improvement.
static GLOBAL_RUNTIME: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create global Tokio runtime")
});

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

    // 1. Validate file exists
    io::validate_file_exists(path)?;

    // 2. MIME detection/validation
    let detected_mime = mime::detect_or_validate(Some(path), mime_type)?;

    // 3. TODO: Cache check (when cache module is ready)
    // if config.use_cache {
    //     if let Some(cached) = cache::get(path, config).await? {
    //         return Ok(cached);
    //     }
    // }

    // 4. Ensure built-in extractors are registered
    crate::extractors::ensure_initialized()?;

    // 5. Get extractor from plugin registry
    let extractor = {
        let registry = crate::plugins::registry::get_document_extractor_registry();
        let registry_read = registry.read().unwrap();
        registry_read.get(&detected_mime)?
        // Lock released at end of scope
    };

    // 6. Extract content
    let mut result = extractor.extract_file(path, &detected_mime, config).await?;

    // 5. Run post-processing pipeline
    result = crate::core::pipeline::run_pipeline(result, config).await?;

    // 6. TODO: Cache write (when cache module is ready)
    // if config.use_cache {
    //     cache::set(path, config, &result).await?;
    // }

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

    // 1. Validate MIME type
    let validated_mime = mime::validate_mime_type(mime_type)?;

    // 2. Ensure built-in extractors are registered
    crate::extractors::ensure_initialized()?;

    // 3. Get extractor from plugin registry
    let extractor = {
        let registry = crate::plugins::registry::get_document_extractor_registry();
        let registry_read = registry.read().unwrap();
        registry_read.get(&validated_mime)?
        // Lock released at end of scope
    };

    // 4. Extract content
    let mut result = extractor.extract_bytes(content, &validated_mime, config).await?;

    // 3. Run post-processing pipeline
    result = crate::core::pipeline::run_pipeline(result, config).await?;

    Ok(result)
}

/// Extract content from multiple files concurrently.
///
/// This function processes multiple files in parallel, automatically managing
/// concurrency based on CPU count.
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
    use tokio::task::JoinSet;

    if paths.is_empty() {
        return Ok(vec![]);
    }

    // Share config across tasks
    let config = Arc::new(config.clone());

    // Create task set for concurrent execution
    let mut tasks = JoinSet::new();

    for (index, path) in paths.into_iter().enumerate() {
        let path_buf = path.as_ref().to_path_buf();
        let config_clone = Arc::clone(&config);

        tasks.spawn(async move {
            let result = extract_file(&path_buf, None, &config_clone).await;
            (index, result)
        });
    }

    // Collect results in order
    let mut results: Vec<Option<ExtractionResult>> = vec![None; tasks.len()];

    while let Some(task_result) = tasks.join_next().await {
        match task_result {
            Ok((index, Ok(result))) => {
                results[index] = Some(result);
            }
            Ok((index, Err(e))) => {
                // System errors bubble up
                if matches!(e, KreuzbergError::Io(_)) {
                    return Err(e);
                }

                // Other errors: create error result
                use std::collections::HashMap;
                let mut metadata = HashMap::new();
                metadata.insert(
                    "error".to_string(),
                    serde_json::json!({
                        "type": format!("{:?}", e),
                        "message": e.to_string(),
                    }),
                );

                results[index] = Some(ExtractionResult {
                    content: format!("Error: {}", e),
                    mime_type: "text/plain".to_string(),
                    metadata,
                    tables: vec![],
                });
            }
            Err(join_err) => {
                return Err(KreuzbergError::Other(format!("Task panicked: {}", join_err)));
            }
        }
    }

    // Unwrap all results (guaranteed to be Some at this point)
    Ok(results.into_iter().map(|r| r.unwrap()).collect())
}

/// Extract content from multiple byte arrays concurrently.
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
    use tokio::task::JoinSet;

    if contents.is_empty() {
        return Ok(vec![]);
    }

    // Share config across tasks
    let config = Arc::new(config.clone());

    // Convert to owned data for tasks
    let owned_contents: Vec<(Vec<u8>, String)> = contents
        .into_iter()
        .map(|(bytes, mime)| (bytes.to_vec(), mime.to_string()))
        .collect();

    // Create task set for concurrent execution
    let mut tasks = JoinSet::new();

    for (index, (bytes, mime_type)) in owned_contents.into_iter().enumerate() {
        let config_clone = Arc::clone(&config);

        tasks.spawn(async move {
            let result = extract_bytes(&bytes, &mime_type, &config_clone).await;
            (index, result)
        });
    }

    // Collect results in order
    let mut results: Vec<Option<ExtractionResult>> = vec![None; tasks.len()];

    while let Some(task_result) = tasks.join_next().await {
        match task_result {
            Ok((index, Ok(result))) => {
                results[index] = Some(result);
            }
            Ok((index, Err(e))) => {
                // System errors bubble up
                if matches!(e, KreuzbergError::Io(_)) {
                    return Err(e);
                }

                // Other errors: create error result
                use std::collections::HashMap;
                let mut metadata = HashMap::new();
                metadata.insert(
                    "error".to_string(),
                    serde_json::json!({
                        "type": format!("{:?}", e),
                        "message": e.to_string(),
                    }),
                );

                results[index] = Some(ExtractionResult {
                    content: format!("Error: {}", e),
                    mime_type: "text/plain".to_string(),
                    metadata,
                    tables: vec![],
                });
            }
            Err(join_err) => {
                return Err(KreuzbergError::Other(format!("Task panicked: {}", join_err)));
            }
        }
    }

    // Unwrap all results (guaranteed to be Some at this point)
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

        // Test sync wrapper
        let result = extract_file_sync(&file_path, None, &config);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().content, "sync test");

        // Test bytes sync wrapper
        let result = extract_bytes_sync(b"test", "text/plain", &config);
        assert!(result.is_ok());
    }
}
