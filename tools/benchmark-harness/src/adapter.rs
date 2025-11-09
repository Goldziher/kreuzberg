//! Framework adapter system
//!
//! Adapters provide a unified interface for extracting content across different
//! frameworks and language bindings. This allows benchmarking any extraction
//! framework against the same test fixtures.

use crate::{Result, types::BenchmarkResult};
use async_trait::async_trait;
use std::path::Path;
use std::time::Duration;

/// Unified interface for document extraction frameworks
///
/// Implementations of this trait can extract content from documents using
/// different frameworks (Kreuzberg Rust core, Python bindings, Node.js, etc.)
#[async_trait]
pub trait FrameworkAdapter: Send + Sync {
    /// Get the framework name (e.g., "kreuzberg-native", "kreuzberg-python")
    fn name(&self) -> &str;

    /// Check if this adapter supports the given file type
    ///
    /// # Arguments
    /// * `file_type` - File extension without dot (e.g., "pdf", "docx")
    fn supports_format(&self, file_type: &str) -> bool;

    /// Extract content from a document
    ///
    /// # Arguments
    /// * `file_path` - Path to the document to extract
    /// * `timeout` - Maximum time to wait for extraction
    ///
    /// # Returns
    /// * `Ok(BenchmarkResult)` - Successful extraction with metrics
    /// * `Err(Error)` - Extraction failed
    async fn extract(&self, file_path: &Path, timeout: Duration) -> Result<BenchmarkResult>;

    /// Extract content from multiple documents using framework's batch API
    ///
    /// Frameworks with native batch support should override this method to use
    /// their optimized batch extraction API (e.g., Kreuzberg's `batch_extract_file()`).
    ///
    /// Default implementation calls `extract()` sequentially for each file.
    ///
    /// # Arguments
    /// * `file_paths` - Paths to documents to extract
    /// * `timeout` - Maximum time to wait for each extraction
    ///
    /// # Returns
    /// * `Ok(Vec<BenchmarkResult>)` - Results for all files
    /// * `Err(Error)` - Batch extraction failed
    async fn extract_batch(&self, file_paths: &[&Path], timeout: Duration) -> Result<Vec<BenchmarkResult>> {
        let mut results = Vec::new();
        for path in file_paths {
            results.push(self.extract(path, timeout).await?);
        }
        Ok(results)
    }

    /// Check if this adapter supports batch extraction
    ///
    /// Returns true if the adapter overrides `extract_batch()` with an optimized implementation.
    /// Default is false (uses sequential extraction).
    fn supports_batch(&self) -> bool {
        false
    }

    /// Get version information for this framework
    fn version(&self) -> String {
        "unknown".to_string()
    }

    /// Perform any necessary setup before benchmarking
    async fn setup(&self) -> Result<()> {
        Ok(())
    }

    /// Perform any necessary cleanup after benchmarking
    async fn teardown(&self) -> Result<()> {
        Ok(())
    }

    /// Warm up the framework by performing a test extraction
    ///
    /// This is called once before benchmarking to get the framework into a warm state.
    /// It measures the cold start time (framework load + first extraction).
    ///
    /// The default implementation performs a single extraction on the provided warmup file.
    ///
    /// # Arguments
    /// * `warmup_file` - Path to a small test file for warmup
    /// * `timeout` - Maximum time to wait for warmup
    ///
    /// # Returns
    /// * `Ok(Duration)` - Cold start duration (framework load + first extraction)
    /// * `Err(Error)` - Warmup failed
    async fn warmup(&self, warmup_file: &Path, timeout: Duration) -> Result<Duration> {
        let start = std::time::Instant::now();
        let _ = self.extract(warmup_file, timeout).await?;
        Ok(start.elapsed())
    }
}
