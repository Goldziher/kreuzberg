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
//! - **Plugin System**: Language-agnostic plugin architecture
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

#![deny(unsafe_code)]

pub mod cache;
pub mod core;
pub mod error;
pub mod extraction;
pub mod extractors;
pub mod plugins;
pub mod text;
pub mod types;

#[cfg(feature = "api")]
pub mod api;

#[cfg(feature = "mcp")]
pub mod mcp;

#[cfg(feature = "chunking")]
pub mod chunking;

#[cfg(feature = "ocr")]
pub mod image;

#[cfg(feature = "language-detection")]
pub mod language_detection;

#[cfg(feature = "stopwords")]
pub mod stopwords;

#[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
pub mod keywords;

#[cfg(feature = "ocr")]
pub mod ocr;

#[cfg(feature = "pdf")]
pub mod pdf;

pub use error::{KreuzbergError, Result};
pub use types::*;

pub use core::extractor::{batch_extract_bytes, batch_extract_file, extract_bytes, extract_file};

pub use core::extractor::{batch_extract_bytes_sync, batch_extract_file_sync, extract_bytes_sync, extract_file_sync};

pub use core::config::{
    ChunkingConfig, ExtractionConfig, ImageExtractionConfig, LanguageDetectionConfig, OcrConfig, PdfConfig,
    PostProcessorConfig, TokenReductionConfig,
};

pub use core::mime::{
    DOCX_MIME_TYPE, EXCEL_MIME_TYPE, HTML_MIME_TYPE, JSON_MIME_TYPE, MARKDOWN_MIME_TYPE, PDF_MIME_TYPE,
    PLAIN_TEXT_MIME_TYPE, POWER_POINT_MIME_TYPE, XML_MIME_TYPE, detect_mime_type, detect_or_validate,
    validate_mime_type,
};

pub use plugins::registry::{
    get_document_extractor_registry, get_ocr_backend_registry, get_post_processor_registry, get_validator_registry,
};
