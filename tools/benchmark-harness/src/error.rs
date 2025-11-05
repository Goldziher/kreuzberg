//! Error types for the benchmark harness

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias for benchmark harness operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during benchmark operations
#[derive(Error, Debug)]
pub enum Error {
    /// I/O error occurred
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Fixture validation error
    #[error("Invalid fixture at {path}: {reason}")]
    InvalidFixture { path: PathBuf, reason: String },

    /// Fixture file not found
    #[error("Fixture file not found: {0}")]
    FixtureNotFound(PathBuf),

    /// Test document not found
    #[error("Test document not found: {0}")]
    DocumentNotFound(PathBuf),

    /// Framework extraction error
    #[error("Framework '{framework}' failed on {file}: {message}")]
    ExtractionFailed {
        framework: String,
        file: PathBuf,
        message: String,
    },

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Benchmark execution error
    #[error("Benchmark error: {0}")]
    Benchmark(String),

    /// Timeout error
    #[error("Timeout: {0}")]
    Timeout(String),
}
