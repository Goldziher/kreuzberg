//! Batch extraction with automatic pandoc-server mode optimization
//!
//! This module provides intelligent batch processing for Pandoc extractions with automatic
//! server mode optimization. The `BatchExtractor` automatically detects if pandoc-server is
//! available and uses it when beneficial for performance.
//!
//! # Server Mode Detection
//!
//! The extractor checks for pandoc-server availability in two ways:
//! 1. Direct binary check: Looks for `pandoc-server` in PATH
//! 2. Version detection: Checks if pandoc 3.8+ is installed (supports server mode)
//!
//! # Optimization Heuristic
//!
//! - **>3 files**: Uses server mode (amortizes ~100-200ms startup overhead)
//! - **≤3 files**: Uses subprocess mode (avoids server startup cost)
//! - **Server unavailable**: Always uses subprocess mode (graceful fallback)
//!
//! # Example Usage
//!
//! ```no_run
//! use kreuzberg::extraction::pandoc::BatchExtractor;
//! use std::path::Path;
//!
//! #[tokio::main]
//! async fn main() -> kreuzberg::Result<()> {
//!     // Create extractor (auto-detects server availability)
//!     let extractor = BatchExtractor::new().await;
//!
//!     // Extract multiple files
//!     let paths = vec![
//!         Path::new("doc1.docx"),
//!         Path::new("doc2.docx"),
//!         Path::new("doc3.docx"),
//!         Path::new("doc4.docx"),
//!     ];
//!     let formats = vec!["docx", "docx", "docx", "docx"];
//!
//!     let results = extractor.extract_files(&paths, &formats).await?;
//!
//!     // Process results
//!     for (i, result) in results.iter().enumerate() {
//!         match result {
//!             Ok(extraction) => println!("File {}: {} chars", i, extraction.content.len()),
//!             Err(e) => eprintln!("File {}: Error: {}", i, e),
//!         }
//!     }
//!
//!     // Cleanup (optional, happens automatically on drop)
//!     extractor.shutdown().await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Logging
//!
//! Enable tracing to see server mode detection and usage:
//!
//! ```bash
//! RUST_LOG=kreuzberg=debug cargo run
//! ```
//!
//! Expected logs:
//! - `DEBUG`: Server detection results, file counts
//! - `INFO`: Server startup confirmation
//! - `WARN`: Server failures with troubleshooting guidance

use crate::error::{KreuzbergError, Result};
use crate::types::PandocExtractionResult;
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::server::PandocServer;
use super::subprocess;

/// Batch extractor with automatic server mode optimization
///
/// Automatically detects pandoc-server availability and uses it when beneficial for
/// performance (>3 files). Falls back to subprocess mode gracefully.
///
/// # Performance
///
/// - **Server mode**: ~100-200ms savings per file (startup overhead eliminated)
/// - **Subprocess mode**: Lightweight, suitable for small batches
///
/// # Thread Safety
///
/// This struct is safe to use across multiple async tasks. The server instance is
/// protected by an Arc<Mutex> and can be safely shared.
pub struct BatchExtractor {
    server: Arc<Mutex<Option<PandocServer>>>,
    use_server: bool,
}

impl BatchExtractor {
    /// Create a new batch extractor
    ///
    /// Automatically detects if pandoc-server is available and uses it if beneficial
    pub async fn new() -> Self {
        let use_server = PandocServer::is_server_available().await;

        if use_server {
            tracing::info!("Pandoc server mode available - will use for batch processing");
        } else {
            tracing::debug!("Pandoc server mode not available - using subprocess mode");
            tracing::debug!("To enable server mode:");
            tracing::debug!("  1. Install pandoc 3.8+ (current version may be older)");
            tracing::debug!("  2. OR create symlink: ln -s $(which pandoc) /usr/local/bin/pandoc-server");
        }

        Self {
            server: Arc::new(Mutex::new(None)),
            use_server,
        }
    }

    /// Extract multiple files using optimal strategy
    ///
    /// Automatically starts server if available and beneficial (>3 files)
    pub async fn extract_files(
        &self,
        paths: &[&Path],
        formats: &[&str],
    ) -> Result<Vec<Result<PandocExtractionResult>>> {
        if paths.is_empty() {
            return Ok(vec![]);
        }

        // Use server mode if available and beneficial (>3 files)
        if self.use_server && paths.len() > 3 {
            self.extract_with_server(paths, formats).await
        } else {
            self.extract_with_subprocess(paths, formats).await
        }
    }

    /// Extract using server mode (warm instance)
    async fn extract_with_server(
        &self,
        paths: &[&Path],
        formats: &[&str],
    ) -> Result<Vec<Result<PandocExtractionResult>>> {
        let mut server_lock = self.server.lock().await;

        // Start server if not already running
        if server_lock.is_none() {
            match PandocServer::new(None, None).await {
                Ok(server) => {
                    if let Err(e) = server.start().await {
                        tracing::warn!("Failed to start pandoc-server: {}", e);
                        tracing::warn!("Falling back to subprocess mode");
                        tracing::warn!("To fix:");
                        tracing::warn!("  1. Ensure pandoc 3.8+ is installed: pandoc --version");
                        tracing::warn!("  2. Create symlink: ln -s $(which pandoc) /usr/local/bin/pandoc-server");
                        drop(server_lock);
                        return self.extract_with_subprocess(paths, formats).await;
                    }

                    tracing::info!("Started pandoc-server for batch processing ({} files)", paths.len());
                    *server_lock = Some(server);
                }
                Err(e) => {
                    tracing::warn!("Failed to create pandoc-server: {}", e);
                    tracing::warn!("Falling back to subprocess mode");
                    drop(server_lock);
                    return self.extract_with_subprocess(paths, formats).await;
                }
            }
        } else {
            tracing::debug!("Reusing warm pandoc-server instance for {} files", paths.len());
        }

        let server = server_lock.as_ref().unwrap();
        let mut results = Vec::with_capacity(paths.len());

        for (i, (path, format)) in paths.iter().zip(formats.iter()).enumerate() {
            tracing::debug!("Extracting file {}/{} via server: {:?}", i + 1, paths.len(), path);

            // Read file content
            let content = match tokio::fs::read(path).await {
                Ok(c) => c,
                Err(e) => {
                    results.push(Err(KreuzbergError::Io(e)));
                    continue;
                }
            };

            // Convert using server
            let result = match server.convert(&String::from_utf8_lossy(&content), format, "json").await {
                Ok(json_output) => {
                    // Parse JSON and extract content/metadata
                    match serde_json::from_str::<Value>(&json_output) {
                        Ok(json_data) => {
                            let content = subprocess::extract_content_from_json(&json_data)?;
                            let metadata = subprocess::extract_metadata_from_json(&json_data)?;

                            Ok(PandocExtractionResult { content, metadata })
                        }
                        Err(e) => Err(KreuzbergError::parsing(format!(
                            "Failed to parse JSON from server: {}",
                            e
                        ))),
                    }
                }
                Err(e) => Err(e),
            };

            results.push(result);
        }

        Ok(results)
    }

    /// Extract using subprocess mode
    async fn extract_with_subprocess(
        &self,
        paths: &[&Path],
        formats: &[&str],
    ) -> Result<Vec<Result<PandocExtractionResult>>> {
        tracing::debug!("Extracting {} files via subprocess mode", paths.len());

        let mut results = Vec::with_capacity(paths.len());

        for (path, format) in paths.iter().zip(formats.iter()) {
            let result = subprocess::extract_with_pandoc(path, format)
                .await
                .map(|(content, metadata)| PandocExtractionResult { content, metadata });
            results.push(result);
        }

        Ok(results)
    }

    /// Stop the server if running
    pub async fn shutdown(&self) -> Result<()> {
        let mut server_lock = self.server.lock().await;
        if let Some(server) = server_lock.take() {
            tracing::info!("Shutting down pandoc-server");
            server.stop().await?;
        }
        Ok(())
    }
}

impl Drop for BatchExtractor {
    fn drop(&mut self) {
        // Best-effort cleanup
        if let Some(server) = self.server.try_lock().ok().and_then(|mut s| s.take()) {
            tokio::spawn(async move {
                let _ = server.stop().await;
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_batch_extractor_creation() {
        let extractor = BatchExtractor::new().await;
        // Should not panic
        assert!(extractor.server.lock().await.is_none());
    }

    #[tokio::test]
    async fn test_empty_batch() {
        let extractor = BatchExtractor::new().await;
        let results = extractor.extract_files(&[], &[]).await;
        assert!(results.is_ok());
        assert!(results.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_shutdown() {
        let extractor = BatchExtractor::new().await;
        let result = extractor.shutdown().await;
        assert!(result.is_ok());
    }
}
