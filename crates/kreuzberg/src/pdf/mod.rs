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
//! println!("Page count: {:?}", metadata.page_count);
//! # Ok(())
//! # }
//! ```
//!
//! # Note
//!
//! This module is always available. The `ocr` feature enables additional
//! functionality in the PDF extractor for rendering pages to images.
pub mod error;
pub mod images;
pub mod metadata;
pub mod rendering;
pub mod text;

pub use error::PdfError;
pub use images::{PdfImage, PdfImageExtractor, extract_images_from_pdf};
pub use metadata::extract_metadata;
pub use rendering::{PageRenderOptions, render_page_to_image};
pub use text::extract_text_from_pdf;
