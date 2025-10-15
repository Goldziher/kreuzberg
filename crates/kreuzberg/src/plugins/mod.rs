//! Plugin system for extending Kreuzberg functionality.
//!
//! The plugin system provides a trait-based architecture that allows extending
//! Kreuzberg with custom extractors, OCR backends, post-processors, and validators.
//!
//! # Plugin Types
//!
//! - [`Plugin`] - Base trait that all plugins must implement
//! - [`OcrBackend`] - OCR processing plugins
//! - [`DocumentExtractor`] - Document format extraction plugins
//! - [`PostProcessor`] - Content post-processing plugins
//! - [`Validator`] - Validation plugins
//!
//! # Language Support
//!
//! Plugins can be implemented in:
//! - **Rust** (native, highest performance)
//! - **Python** (via PyO3 FFI bridge)
//! - **Node.js** (future - via napi-rs FFI bridge)
//!
//! # Example
//!
//! ```rust
//! use kreuzberg::plugins::{Plugin, DocumentExtractor};
//! use kreuzberg::{Result, ExtractionResult, ExtractionConfig};
//! use async_trait::async_trait;
//! use std::path::Path;
//!
//! struct MyCustomExtractor;
//!
//! impl Plugin for MyCustomExtractor {
//!     fn name(&self) -> &str { "my-custom-extractor" }
//!     fn version(&self) -> &str { "1.0.0" }
//!     fn initialize(&mut self) -> Result<()> { Ok(()) }
//!     fn shutdown(&mut self) -> Result<()> { Ok(()) }
//! }
//!
//! #[async_trait]
//! impl DocumentExtractor for MyCustomExtractor {
//!     async fn extract_bytes(&self, content: &[u8], mime_type: &str, config: &ExtractionConfig)
//!         -> Result<ExtractionResult> {
//!         // Custom extraction logic
//!         todo!()
//!     }
//!
//!     async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig)
//!         -> Result<ExtractionResult> {
//!         // Custom file extraction logic
//!         todo!()
//!     }
//!
//!     fn supported_mime_types(&self) -> &[&str] {
//!         &["application/x-custom"]
//!     }
//!
//!     fn priority(&self) -> i32 { 50 }
//! }
//! ```

mod extractor;
mod ocr;
mod processor;
pub mod registry;
mod traits;
mod validator;

pub use extractor::DocumentExtractor;
pub use ocr::{OcrBackend, OcrBackendType};
pub use processor::{PostProcessor, ProcessingStage};
pub use traits::Plugin;
pub use validator::Validator;
