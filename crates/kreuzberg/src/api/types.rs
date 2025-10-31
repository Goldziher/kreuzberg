//! API request and response types.

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{ExtractionConfig, types::ExtractionResult};

/// API server size limit configuration.
///
/// Controls maximum sizes for request bodies and multipart uploads.
/// Default limits are designed for typical document processing workloads.
///
/// # Default Values
///
/// - `max_request_body_bytes`: 100 MB (104,857,600 bytes)
/// - `max_multipart_field_bytes`: 100 MB (104,857,600 bytes)
///
/// # Examples
///
/// ```
/// use kreuzberg::api::ApiSizeLimits;
///
/// // Default limits (100 MB)
/// let limits = ApiSizeLimits::default();
///
/// // Custom limits (50 MB for both)
/// let limits = ApiSizeLimits {
///     max_request_body_bytes: 50 * 1024 * 1024,
///     max_multipart_field_bytes: 50 * 1024 * 1024,
/// };
///
/// // Very large documents (500 MB)
/// let limits = ApiSizeLimits {
///     max_request_body_bytes: 500 * 1024 * 1024,
///     max_multipart_field_bytes: 500 * 1024 * 1024,
/// };
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ApiSizeLimits {
    /// Maximum size of the entire request body in bytes.
    ///
    /// This applies to the total size of all uploaded files and form data
    /// in a single request. Default: 100 MB (104,857,600 bytes).
    pub max_request_body_bytes: usize,

    /// Maximum size of a single multipart field in bytes.
    ///
    /// This applies to individual files in a multipart upload.
    /// Default: 100 MB (104,857,600 bytes).
    pub max_multipart_field_bytes: usize,
}

impl Default for ApiSizeLimits {
    fn default() -> Self {
        Self {
            // 100 MB default for request body - handles most document processing workloads
            // including large PDFs, scanned documents, and batch uploads
            max_request_body_bytes: 100 * 1024 * 1024,
            // 100 MB default for individual multipart fields - allows large single documents
            max_multipart_field_bytes: 100 * 1024 * 1024,
        }
    }
}

impl ApiSizeLimits {
    /// Create new size limits with custom values.
    ///
    /// # Arguments
    ///
    /// * `max_request_body_bytes` - Maximum total request size in bytes
    /// * `max_multipart_field_bytes` - Maximum individual file size in bytes
    pub fn new(max_request_body_bytes: usize, max_multipart_field_bytes: usize) -> Self {
        Self {
            max_request_body_bytes,
            max_multipart_field_bytes,
        }
    }

    /// Create size limits from MB values (convenience method).
    ///
    /// # Arguments
    ///
    /// * `max_request_body_mb` - Maximum total request size in megabytes
    /// * `max_multipart_field_mb` - Maximum individual file size in megabytes
    ///
    /// # Examples
    ///
    /// ```
    /// use kreuzberg::api::ApiSizeLimits;
    ///
    /// // 50 MB limits
    /// let limits = ApiSizeLimits::from_mb(50, 50);
    /// ```
    pub fn from_mb(max_request_body_mb: usize, max_multipart_field_mb: usize) -> Self {
        Self {
            max_request_body_bytes: max_request_body_mb * 1024 * 1024,
            max_multipart_field_bytes: max_multipart_field_mb * 1024 * 1024,
        }
    }
}

/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Health status
    pub status: String,
    /// API version
    pub version: String,
}

/// Server information response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoResponse {
    /// API version
    pub version: String,
    /// Whether using Rust backend
    pub rust_backend: bool,
}

/// Extraction response (list of results).
pub type ExtractResponse = Vec<ExtractionResult>;

/// Error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error type name
    pub error_type: String,
    /// Error message
    pub message: String,
    /// Stack trace (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub traceback: Option<String>,
    /// HTTP status code
    pub status_code: u16,
}

/// API server state.
///
/// Holds the default extraction configuration loaded from config file
/// (via discovery or explicit path). Per-request configs override these defaults.
#[derive(Debug, Clone)]
pub struct ApiState {
    /// Default extraction configuration
    pub default_config: Arc<ExtractionConfig>,
}

/// Cache statistics response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatsResponse {
    /// Cache directory path
    pub directory: String,
    /// Total number of cache files
    pub total_files: usize,
    /// Total cache size in MB
    pub total_size_mb: f64,
    /// Available disk space in MB
    pub available_space_mb: f64,
    /// Age of oldest file in days
    pub oldest_file_age_days: f64,
    /// Age of newest file in days
    pub newest_file_age_days: f64,
}

/// Cache clear response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheClearResponse {
    /// Cache directory path
    pub directory: String,
    /// Number of files removed
    pub removed_files: usize,
    /// Space freed in MB
    pub freed_mb: f64,
}
