//! Core extraction orchestration module.
//!
//! This module contains the main extraction logic and orchestration layer for Kreuzberg.
//! It provides the primary entry points for file and bytes extraction, manages the
//! extractor registry, MIME type detection, configuration, and post-processing pipeline.
//!
//! # Architecture
//!
//! The core module is responsible for:
//! - **Entry Points**: Main `extract_file()` and `extract_bytes()` functions
//! - **Registry**: Mapping MIME types to extractors with priority-based selection
//! - **MIME Detection**: Detecting and validating MIME types from files and extensions
//! - **Pipeline**: Orchestrating post-processing steps (chunking, quality, etc.)
//! - **Configuration**: Loading and managing extraction configuration
//! - **I/O**: File reading and validation utilities
//!
//! # Example
//!
//! ```rust,no_run
//! use kreuzberg::core::extractor::extract_file;
//! use kreuzberg::core::config::ExtractionConfig;
//!
//! # async fn example() -> kreuzberg::Result<()> {
//! let config = ExtractionConfig::default();
//! let result = extract_file("document.pdf", None, &config).await?;
//! println!("Extracted content: {}", result.content);
//! # Ok(())
//! # }
//! ```

pub mod config;
pub mod extractor;
pub mod io;
pub mod mime;
pub mod pipeline;

// Re-export commonly used items
pub use config::ExtractionConfig;
pub use extractor::{batch_extract_bytes, batch_extract_file, extract_bytes, extract_file};
