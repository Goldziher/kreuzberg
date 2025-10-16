//! API request and response types.

use serde::{Deserialize, Serialize};

use crate::types::ExtractionResult;

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
