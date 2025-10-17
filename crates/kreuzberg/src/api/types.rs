//! API request and response types.

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{ExtractionConfig, types::ExtractionResult};

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
