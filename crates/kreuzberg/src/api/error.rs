//! API error handling.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::error::KreuzbergError;

use super::types::ErrorResponse;

/// API-specific error wrapper.
#[derive(Debug)]
pub struct ApiError {
    /// HTTP status code
    pub status: StatusCode,
    /// Error response body
    pub body: ErrorResponse,
}

impl ApiError {
    /// Create a new API error.
    pub fn new(status: StatusCode, error: KreuzbergError) -> Self {
        let error_type = match &error {
            KreuzbergError::Validation { .. } => "ValidationError",
            KreuzbergError::Parsing { .. } => "ParsingError",
            KreuzbergError::Ocr { .. } => "OCRError",
            KreuzbergError::Io(_) => "IOError",
            KreuzbergError::Cache { .. } => "CacheError",
            KreuzbergError::ImageProcessing { .. } => "ImageProcessingError",
            KreuzbergError::Serialization { .. } => "SerializationError",
            KreuzbergError::MissingDependency(_) => "MissingDependencyError",
            KreuzbergError::Plugin { .. } => "PluginError",
            KreuzbergError::UnsupportedFormat(_) => "UnsupportedFormatError",
            KreuzbergError::Other(_) => "Error",
        };

        Self {
            status,
            body: ErrorResponse {
                error_type: error_type.to_string(),
                message: error.to_string(),
                traceback: None, // Rust doesn't have Python-style tracebacks
                status_code: status.as_u16(),
            },
        }
    }

    /// Create a validation error (400).
    pub fn validation(error: KreuzbergError) -> Self {
        Self::new(StatusCode::BAD_REQUEST, error)
    }

    /// Create an unprocessable entity error (422).
    pub fn unprocessable(error: KreuzbergError) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, error)
    }

    /// Create an internal server error (500).
    pub fn internal(error: KreuzbergError) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

impl From<KreuzbergError> for ApiError {
    fn from(error: KreuzbergError) -> Self {
        // Map Kreuzberg errors to appropriate HTTP status codes
        match &error {
            KreuzbergError::Validation { .. } => Self::validation(error),
            KreuzbergError::Parsing { .. } | KreuzbergError::Ocr { .. } => Self::unprocessable(error),
            _ => Self::internal(error),
        }
    }
}
