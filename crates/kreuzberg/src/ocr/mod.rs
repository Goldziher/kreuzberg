//! OCR (Optical Character Recognition) subsystem.
//!
//! This module provides OCR functionality using Tesseract as the backend.
//! It includes caching, table reconstruction, hOCR parsing, and batch processing.
//!
//! # Features
//!
//! - **Tesseract integration**: Native Tesseract backend via `kreuzberg-tesseract`
//! - **Result caching**: Persistent cache for OCR results using file hashing
//! - **Table reconstruction**: Extract and reconstruct tables from hOCR/TSV output
//! - **hOCR to Markdown**: Convert hOCR format to clean Markdown
//! - **Batch processing**: Process multiple images efficiently
//! - **Language support**: Validate and configure Tesseract languages
//! - **PSM modes**: Support for all Tesseract Page Segmentation Modes
//!
//! # Example
//!
//! ```rust,no_run
//! use kreuzberg::ocr::{OcrProcessor, TesseractConfig};
//!
//! # fn example() -> Result<(), kreuzberg::ocr::error::OcrError> {
//! let processor = OcrProcessor::new(None)?;
//! let config = TesseractConfig::default();
//!
//! let image_bytes = std::fs::read("scanned.png").expect("failed to read image");
//! let result = processor.process_image(&image_bytes, &config)?;
//!
//! println!("Extracted text: {}", result.content);
//! # Ok(())
//! # }
//! ```
//!
//! # Optional Feature
//!
//! This module requires the `ocr` feature to be enabled:
//! ```toml
//! [dependencies]
//! kreuzberg = { version = "4.0", features = ["ocr"] }
//! ```
pub mod cache;
pub mod error;
pub mod hocr;
pub mod processor;
pub mod table;
pub mod tesseract_backend;
pub mod types;
pub mod utils;
pub mod validation;

pub use cache::{OcrCache, OcrCacheStats};
pub use error::OcrError;
pub use hocr::convert_hocr_to_markdown;
pub use processor::OcrProcessor;
pub use table::{HocrWord, extract_words_from_tsv, reconstruct_table, table_to_markdown};
pub use tesseract_backend::TesseractBackend;
pub use types::{BatchItemResult, ExtractionResult, PSMMode, Table, TesseractConfig};
pub use utils::compute_hash;
pub use validation::{validate_language_code, validate_tesseract_version};
