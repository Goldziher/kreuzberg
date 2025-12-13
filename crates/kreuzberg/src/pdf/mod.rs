//! PDF document processing utilities.
//!
//! This module provides low-level PDF processing functions for text extraction,
//! metadata parsing, image extraction, and page rendering. Used internally by
//! the PDF extractor plugin.
//!
//! # Features
//!
//! - **Text extraction**: Extract text content from PDFs using `pdfium-render`
//! - **Metadata extraction**: Parse PDF metadata (title, author, creation date, etc.)
//! - **Image extraction**: Extract embedded images from PDF pages
//! - **Page rendering**: Render PDF pages to images for OCR processing
//! - **Error handling**: Comprehensive PDF-specific error types
//!
//! # Example
//!
//! ```rust
//! use kreuzberg::pdf::{extract_text_from_pdf, extract_metadata};
//!
//! # fn example() -> kreuzberg::Result<()> {
//! let pdf_bytes = std::fs::read("document.pdf")?;
//!
//! // Extract text
//! let text = extract_text_from_pdf(&pdf_bytes)?;
//! println!("Text: {}", text);
//!
//! // Extract metadata
//! let metadata = extract_metadata(&pdf_bytes)?;
//! println!("PDF version: {:?}", metadata.pdf_version);
//! # Ok(())
//! # }
//! ```
//!
//! # Note
//!
//! This module requires the `pdf` feature. The `ocr` feature enables additional
//! functionality in the PDF extractor for rendering pages to images.
#[cfg(feature = "pdf")]
pub mod error;
#[cfg(feature = "pdf")]
pub mod images;
#[cfg(feature = "pdf")]
pub mod metadata;
#[cfg(feature = "pdf")]
pub mod rendering;
#[cfg(feature = "pdf")]
pub mod table;
#[cfg(feature = "pdf")]
pub mod text;

#[cfg(feature = "pdf")]
pub use error::PdfError;
#[cfg(feature = "pdf")]
pub use images::{PdfImage, PdfImageExtractor, extract_images_from_pdf};
#[cfg(feature = "pdf")]
pub use metadata::extract_metadata;
#[cfg(feature = "pdf")]
pub use rendering::{PageRenderOptions, render_page_to_image};
#[cfg(feature = "pdf")]
pub use table::extract_words_from_page;
#[cfg(feature = "pdf")]
pub use text::extract_text_from_pdf;
