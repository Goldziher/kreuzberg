//! Built-in document extractors.
//!
//! This module contains the default extractors that ship with Kreuzberg.
//! All extractors implement the `DocumentExtractor` plugin trait.

use crate::Result;
use crate::plugins::registry::get_document_extractor_registry;
use once_cell::sync::Lazy;
use std::sync::Arc;

pub mod email;
pub mod excel;
pub mod html;
pub mod pdf;
pub mod pptx;
pub mod structured;
pub mod text;
pub mod xml;

pub use email::EmailExtractor;
pub use excel::ExcelExtractor;
pub use html::HtmlExtractor;
pub use pdf::PdfExtractor;
pub use pptx::PptxExtractor;
pub use structured::StructuredExtractor;
pub use text::{MarkdownExtractor, PlainTextExtractor};
pub use xml::XmlExtractor;

/// Lazy-initialized flag that ensures extractors are registered exactly once.
///
/// This static is accessed on first extraction operation to automatically
/// register all built-in extractors with the plugin registry.
static EXTRACTORS_INITIALIZED: Lazy<Result<()>> = Lazy::new(register_default_extractors);

/// Ensure built-in extractors are registered.
///
/// This function is called automatically on first extraction operation.
/// It's safe to call multiple times - registration only happens once.
pub fn ensure_initialized() -> Result<()> {
    EXTRACTORS_INITIALIZED
        .as_ref()
        .map(|_| ())
        .map_err(|e| crate::KreuzbergError::Plugin {
            message: format!("Failed to register default extractors: {}", e),
            plugin_name: "built-in-extractors".to_string(),
        })
}

/// Register all built-in extractors with the global registry.
///
/// This function should be called once at application startup to register
/// the default extractors (PlainText, Markdown, XML, etc.).
///
/// **Note:** This is called automatically on first extraction operation.
/// Explicit calling is optional.
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

    // Register PDF extractor
    registry.register(Arc::new(PdfExtractor::new()))?;

    // Register Excel extractor
    registry.register(Arc::new(ExcelExtractor::new()))?;

    // Register PowerPoint extractor
    registry.register(Arc::new(PptxExtractor::new()))?;

    // Register Email extractor
    registry.register(Arc::new(EmailExtractor::new()))?;

    // Register HTML extractor
    registry.register(Arc::new(HtmlExtractor::new()))?;

    // Register structured data extractor (JSON, YAML, TOML)
    registry.register(Arc::new(StructuredExtractor::new()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_default_extractors() {
        // Clear registry for clean test
        let registry = get_document_extractor_registry();
        {
            let mut reg = registry.write().unwrap();
            *reg = crate::plugins::registry::DocumentExtractorRegistry::new();
        }

        // Register extractors
        register_default_extractors().expect("Failed to register extractors");

        // Verify all extractors are registered
        let reg = registry.read().unwrap();
        let extractor_names = reg.list();

        // Should have 9 extractors: PlainText, Markdown, XML, PDF, Excel, PPTX, Email, HTML, Structured
        assert_eq!(extractor_names.len(), 9, "Expected 9 extractors to be registered");

        // Verify each extractor by name
        assert!(extractor_names.contains(&"plain-text-extractor".to_string()));
        assert!(extractor_names.contains(&"markdown-extractor".to_string()));
        assert!(extractor_names.contains(&"xml-extractor".to_string()));
        assert!(extractor_names.contains(&"pdf-extractor".to_string()));
        assert!(extractor_names.contains(&"excel-extractor".to_string()));
        assert!(extractor_names.contains(&"pptx-extractor".to_string()));
        assert!(extractor_names.contains(&"email-extractor".to_string()));
        assert!(extractor_names.contains(&"html-extractor".to_string()));
        assert!(extractor_names.contains(&"structured-extractor".to_string()));
    }

    #[test]
    fn test_ensure_initialized() {
        // Should not fail
        ensure_initialized().expect("Failed to ensure extractors initialized");
    }
}
