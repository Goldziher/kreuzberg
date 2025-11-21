//! Built-in document extractors.
//!
//! This module contains the default extractors that ship with Kreuzberg.
//! All extractors implement the `DocumentExtractor` plugin trait.

use crate::Result;
use crate::plugins::registry::get_document_extractor_registry;
use once_cell::sync::Lazy;
use std::sync::Arc;

pub mod structured;
pub mod text;

#[cfg(feature = "ocr")]
pub mod image;

#[cfg(feature = "archives")]
pub mod archive;

#[cfg(feature = "email")]
pub mod email;

#[cfg(feature = "excel")]
pub mod excel;

#[cfg(feature = "html")]
pub mod html;

#[cfg(feature = "office")]
pub mod docx;

#[cfg(feature = "office")]
pub mod pandoc;

#[cfg(feature = "pdf")]
pub mod pdf;

#[cfg(feature = "office")]
pub mod pptx;

#[cfg(feature = "xml")]
pub mod xml;

pub use structured::StructuredExtractor;
pub use text::{MarkdownExtractor, PlainTextExtractor};

#[cfg(feature = "ocr")]
pub use image::ImageExtractor;

#[cfg(feature = "archives")]
pub use archive::{SevenZExtractor, TarExtractor, ZipExtractor};

#[cfg(feature = "email")]
pub use email::EmailExtractor;

#[cfg(feature = "excel")]
pub use excel::ExcelExtractor;

#[cfg(feature = "html")]
pub use html::HtmlExtractor;

#[cfg(feature = "office")]
pub use docx::DocxExtractor;

#[cfg(feature = "office")]
pub use pandoc::PandocExtractor;

#[cfg(feature = "pdf")]
pub use pdf::PdfExtractor;

#[cfg(feature = "office")]
pub use pptx::PptxExtractor;

#[cfg(feature = "xml")]
pub use xml::XmlExtractor;

/// Lazy-initialized flag that ensures extractors are registered exactly once.
///
/// This static is accessed on first extraction operation to automatically
/// register all built-in extractors with the plugin registry.
static EXTRACTORS_INITIALIZED: Lazy<Result<()>> = Lazy::new(register_default_extractors);

/// Ensure built-in extractors are registered.
///
/// This function is called automatically on first extraction operation.
/// It's safe to call multiple times - registration only happens once,
/// unless the registry was cleared, in which case extractors are re-registered.
pub fn ensure_initialized() -> Result<()> {
    // First, try the lazy initialization
    EXTRACTORS_INITIALIZED
        .as_ref()
        .map(|_| ())
        .map_err(|e| crate::KreuzbergError::Plugin {
            message: format!("Failed to register default extractors: {}", e),
            plugin_name: "built-in-extractors".to_string(),
        })?;

    // Check if registry is empty (e.g., after clear_document_extractors)
    // If so, re-register the default extractors
    let registry = get_document_extractor_registry();
    let registry_guard = registry
        .read()
        .map_err(|e| crate::KreuzbergError::Other(format!("Document extractor registry lock poisoned: {}", e)))?;

    if registry_guard.list().is_empty() {
        // Drop read lock before acquiring write lock
        drop(registry_guard);
        register_default_extractors()?;
    }

    Ok(())
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
    let mut registry = registry
        .write()
        .map_err(|e| crate::KreuzbergError::Other(format!("Document extractor registry lock poisoned: {}", e)))?;

    registry.register(Arc::new(PlainTextExtractor::new()))?;
    registry.register(Arc::new(MarkdownExtractor::new()))?;
    registry.register(Arc::new(StructuredExtractor::new()))?;

    #[cfg(feature = "ocr")]
    registry.register(Arc::new(ImageExtractor::new()))?;

    #[cfg(feature = "xml")]
    registry.register(Arc::new(XmlExtractor::new()))?;

    #[cfg(feature = "pdf")]
    registry.register(Arc::new(PdfExtractor::new()))?;

    #[cfg(feature = "excel")]
    registry.register(Arc::new(ExcelExtractor::new()))?;

    #[cfg(feature = "office")]
    {
        registry.register(Arc::new(DocxExtractor::new()))?;
        registry.register(Arc::new(PptxExtractor::new()))?;
        registry.register(Arc::new(PandocExtractor::new()))?;
    }

    #[cfg(feature = "email")]
    registry.register(Arc::new(EmailExtractor::new()))?;

    #[cfg(feature = "html")]
    registry.register(Arc::new(HtmlExtractor::new()))?;

    #[cfg(feature = "archives")]
    {
        registry.register(Arc::new(ZipExtractor::new()))?;
        registry.register(Arc::new(TarExtractor::new()))?;
        registry.register(Arc::new(SevenZExtractor::new()))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_default_extractors() {
        let registry = get_document_extractor_registry();
        {
            let mut reg = registry
                .write()
                .expect("Failed to acquire write lock on registry in test");
            *reg = crate::plugins::registry::DocumentExtractorRegistry::new();
        }

        register_default_extractors().expect("Failed to register extractors");

        let reg = registry
            .read()
            .expect("Failed to acquire read lock on registry in test");
        let extractor_names = reg.list();

        #[allow(unused_mut)]
        let mut expected_count = 3;
        assert!(extractor_names.contains(&"plain-text-extractor".to_string()));
        assert!(extractor_names.contains(&"markdown-extractor".to_string()));
        assert!(extractor_names.contains(&"structured-extractor".to_string()));

        #[cfg(feature = "ocr")]
        {
            expected_count += 1;
            assert!(extractor_names.contains(&"image-extractor".to_string()));
        }

        #[cfg(feature = "xml")]
        {
            expected_count += 1;
            assert!(extractor_names.contains(&"xml-extractor".to_string()));
        }

        #[cfg(feature = "pdf")]
        {
            expected_count += 1;
            assert!(extractor_names.contains(&"pdf-extractor".to_string()));
        }

        #[cfg(feature = "excel")]
        {
            expected_count += 1;
            assert!(extractor_names.contains(&"excel-extractor".to_string()));
        }

        #[cfg(feature = "office")]
        {
            expected_count += 3;
            assert!(extractor_names.contains(&"docx-extractor".to_string()));
            assert!(extractor_names.contains(&"pptx-extractor".to_string()));
            assert!(extractor_names.contains(&"pandoc-extractor".to_string()));
        }

        #[cfg(feature = "email")]
        {
            expected_count += 1;
            assert!(extractor_names.contains(&"email-extractor".to_string()));
        }

        #[cfg(feature = "html")]
        {
            expected_count += 1;
            assert!(extractor_names.contains(&"html-extractor".to_string()));
        }

        #[cfg(feature = "archives")]
        {
            expected_count += 3;
            assert!(extractor_names.contains(&"zip-extractor".to_string()));
            assert!(extractor_names.contains(&"tar-extractor".to_string()));
            assert!(extractor_names.contains(&"7z-extractor".to_string()));
        }

        assert_eq!(
            extractor_names.len(),
            expected_count,
            "Expected {} extractors based on enabled features",
            expected_count
        );
    }

    #[test]
    fn test_ensure_initialized() {
        ensure_initialized().expect("Failed to ensure extractors initialized");
    }
}
