//! PPTX extraction module
//!
//! This module incorporates code from pptx-to-md v0.4.0 (MIT/Apache-2.0)
//! See ATTRIBUTION.md for full attribution details.
//!
//! Architecture:
//! - Streaming-first design for handling huge files
//! - Bounded memory usage with LRU caching
//! - Clean separation of concerns

pub mod config;
pub mod container;
pub mod content_builder;
pub mod extractor;
pub mod metadata;
pub mod notes;
pub mod parser;
pub mod slide;
pub mod streaming;
pub mod types;
pub mod utils;
