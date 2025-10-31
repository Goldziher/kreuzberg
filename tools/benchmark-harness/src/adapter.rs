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
}
