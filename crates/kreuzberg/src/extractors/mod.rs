//! Built-in document extractors.
//!
//! This module contains the default extractors that ship with Kreuzberg.
//! All extractors implement the `DocumentExtractor` plugin trait.

use crate::plugins::registry::get_document_extractor_registry;
use crate::Result;
use std::sync::Arc;

pub mod text;
pub mod xml;

pub use text::{MarkdownExtractor, PlainTextExtractor};
pub use xml::XmlExtractor;

/// Register all built-in extractors with the global registry.
///
/// This function should be called once at application startup to register
/// the default extractors (PlainText, Markdown, XML, etc.).
///
/// # Example
///
/// ```rust
/// use kreuzberg::extractors::register_default_extractors;
///
/// # fn main() -> kreuzberg::Result<()> {
/// register_default_extractors()?;
/// # Ok(())
/// # }
/// ```
pub fn register_default_extractors() -> Result<()> {
    let registry = get_document_extractor_registry();
    let mut registry = registry.write().unwrap();

    // Register text extractors
    registry.register(Arc::new(PlainTextExtractor::new()))?;
    registry.register(Arc::new(MarkdownExtractor::new()))?;

    // Register XML extractor
    registry.register(Arc::new(XmlExtractor::new()))?;

    Ok(())
}
