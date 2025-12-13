//! REST API server for Kreuzberg document extraction.
//!
//! This module provides an Axum-based HTTP server for document extraction
//! with endpoints for single and batch extraction operations.
//!
//! # Endpoints
//!
//! - `POST /extract` - Extract text from uploaded files (multipart form data)
//! - `GET /health` - Health check endpoint
//! - `GET /info` - Server information
//!
//! # Examples
//!
//! ## Starting the server
//!
//! ```no_run
//! use kreuzberg::api::serve;
//!
//! #[tokio::main]
//! async fn main() -> kreuzberg::Result<()> {
//!     // Local development
//!     serve("127.0.0.1", 8000).await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Embedding the router in your app
//!
//! ```no_run
//! use kreuzberg::{ExtractionConfig, api::create_router};
//! use axum::Router;
//!
//! #[tokio::main]
//! async fn main() -> kreuzberg::Result<()> {
//!     // Load config (from file or use default)
//!     let config = ExtractionConfig::default();
//!     let kreuzberg_router = create_router(config);
//!
//!     // Nest under /api prefix
//!     let app = Router::new().nest("/api", kreuzberg_router);
//!
//!     // Add your own routes
//!     // ...
//!
//!     Ok(())
//! }
//! ```
//!
//! # cURL Examples
//!
//! ```bash
//! # Single file extraction
//! curl -F "files=@document.pdf" http://localhost:8000/extract
//!
//! # Multiple files with OCR config
//! curl -F "files=@doc1.pdf" -F "files=@doc2.jpg" \
//!      -F 'config={"ocr":{"language":"eng"}}' \
//!      http://localhost:8000/extract
//!
//! # Health check
//! curl http://localhost:8000/health
//!
//! # Server info
//! curl http://localhost:8000/info
//! ```

mod error;
mod handlers;
mod server;
mod types;

pub use error::ApiError;
pub use server::{
    create_router, create_router_with_limits, serve, serve_default, serve_with_config, serve_with_config_and_limits,
};
pub use types::{
    ApiSizeLimits, ApiState, CacheClearResponse, CacheStatsResponse, ErrorResponse, ExtractResponse, HealthResponse,
    InfoResponse,
};
