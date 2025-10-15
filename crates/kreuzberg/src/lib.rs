//! Kreuzberg - High-Performance Document Intelligence Library
//!
//! Kreuzberg is a Rust-first document extraction library with language-agnostic plugin support.
//! It provides fast, accurate extraction from PDFs, images, Office documents, emails, and more.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use kreuzberg::{extract_file_sync, ExtractionConfig};
//!
//! # fn main() -> kreuzberg::Result<()> {
//! // Extract content from a file
//! let config = ExtractionConfig::default();
//! let result = extract_file_sync("document.pdf", None, &config)?;
//! println!("Extracted: {}", result.content);
//! # Ok(())
//! # }
//! ```
//!
//! # Architecture
//!
//! - **Core Module** (`core`): Main extraction orchestration, MIME detection, config loading
//! - **Plugin System** (coming in Phase 2): Language-agnostic plugin architecture
//! - **Extractors**: Format-specific extraction (PDF, images, Office docs, email, etc.)
//! - **OCR**: Multiple OCR backend support (Tesseract, EasyOCR, PaddleOCR)
//!
//! # Features
//!
//! - Fast parallel processing with async/await
//! - Priority-based extractor selection
//! - Comprehensive MIME type detection (118+ file extensions)
//! - Configurable caching and quality processing
//! - Cross-language plugin support (Python, Node.js planned)

pub mod cache;
pub mod chunking;
pub mod core;
pub mod error;
pub mod extraction;
pub mod image;
pub mod ocr;
pub mod pdf;
pub mod text;
pub mod types;

// Core exports
pub use error::{KreuzbergError, Result};
pub use types::*;

// Main extraction API - async versions
pub use core::extractor::{
    extract_file,
    extract_bytes,
    batch_extract_file,
    batch_extract_bytes,
};

// Main extraction API - sync versions
pub use core::extractor::{
    extract_file_sync,
    extract_bytes_sync,
    batch_extract_file_sync,
    batch_extract_bytes_sync,
};

// Configuration
pub use core::config::{ExtractionConfig, OcrConfig, ChunkingConfig};

// MIME detection utilities
pub use core::mime::{
    detect_mime_type,
    validate_mime_type,
    detect_or_validate,
    // MIME type constants
    PDF_MIME_TYPE,
    HTML_MIME_TYPE,
    MARKDOWN_MIME_TYPE,
    PLAIN_TEXT_MIME_TYPE,
    JSON_MIME_TYPE,
    XML_MIME_TYPE,
    EXCEL_MIME_TYPE,
    POWER_POINT_MIME_TYPE,
    DOCX_MIME_TYPE,
};

// Registry for advanced usage
pub use core::registry::{ExtractorRegistry, get_registry, DEFAULT_PRIORITY};
