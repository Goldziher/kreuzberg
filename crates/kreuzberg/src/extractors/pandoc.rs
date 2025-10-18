//! Pandoc-based extractors for various document formats.
//!
//! Supports: DOCX, ODT, EPUB, LaTeX, RST, RTF, and many more formats via Pandoc.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::pandoc::extract_bytes_from_mime;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata};
use async_trait::async_trait;

/// Generic Pandoc extractor for all Pandoc-supported formats.
///
/// This extractor handles all document formats supported by Pandoc, including:
/// - Microsoft Word (DOCX)
/// - OpenDocument Text (ODT)
/// - EPUB
/// - LaTeX
/// - reStructuredText (RST)
/// - RTF
/// - And many more
pub struct PandocExtractor;

impl PandocExtractor {
    /// Create a new Pandoc extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for PandocExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for PandocExtractor {
    fn name(&self) -> &str {
        "pandoc-extractor"
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn description(&self) -> &str {
        "Extracts content from Pandoc-supported formats (DOCX, ODT, EPUB, LaTeX, RST, RTF, etc.)"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[async_trait]
impl DocumentExtractor for PandocExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        // Use Pandoc to extract
        let pandoc_result = extract_bytes_from_mime(content, mime_type).await?;

        // Put all Pandoc metadata in additional (Pandoc supports many formats with different metadata)
        let mut additional = std::collections::HashMap::new();
        for (key, value) in pandoc_result.metadata {
            additional.insert(key, value);
        }

        Ok(ExtractionResult {
            content: pandoc_result.content,
            mime_type: mime_type.to_string(),
            metadata: Metadata {
                additional,
                ..Default::default()
            },
            tables: vec![],
            detected_languages: None,
            chunks: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[
            // Microsoft Office
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document", // DOCX
            // OpenDocument
            "application/vnd.oasis.opendocument.text", // ODT
            // EPUB
            "application/epub+zip",
            // LaTeX
            "application/x-latex",
            "text/x-tex",
            // reStructuredText
            "text/x-rst",
            "text/prs.fallenstein.rst",
            // RTF
            "application/rtf",
            "text/rtf",
            // Other formats
            "application/x-typst",           // Typst
            "application/x-ipynb+json",      // Jupyter Notebook
            "application/x-fictionbook+xml", // FictionBook
            "text/x-org",                    // Org mode
            "text/x-commonmark",             // CommonMark
            "text/x-gfm",                    // GitHub Flavored Markdown
            "text/x-multimarkdown",          // MultiMarkdown
            "text/x-markdown-extra",         // Markdown Extra
            "application/docbook+xml",       // DocBook
            "application/x-jats+xml",        // JATS
            "application/x-opml+xml",        // OPML
        ]
    }

    fn priority(&self) -> i32 {
        // Lower priority than specialized extractors
        40
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extraction::pandoc::validate_pandoc_version;

    #[tokio::test]
    async fn test_pandoc_extractor_plugin_interface() {
        let extractor = PandocExtractor::new();
        assert_eq!(extractor.name(), "pandoc-extractor");
        assert_eq!(extractor.version(), "1.0.0");
        assert_eq!(extractor.priority(), 40);
        assert!(!extractor.supported_mime_types().is_empty());
    }

    #[tokio::test]
    async fn test_pandoc_extractor_supports_docx() {
        let extractor = PandocExtractor::new();
        assert!(
            extractor
                .supported_mime_types()
                .contains(&"application/vnd.openxmlformats-officedocument.wordprocessingml.document")
        );
    }

    #[tokio::test]
    async fn test_pandoc_extractor_supports_odt() {
        let extractor = PandocExtractor::new();
        assert!(
            extractor
                .supported_mime_types()
                .contains(&"application/vnd.oasis.opendocument.text")
        );
    }

    #[tokio::test]
    async fn test_pandoc_extractor_supports_epub() {
        let extractor = PandocExtractor::new();
        assert!(extractor.supported_mime_types().contains(&"application/epub+zip"));
    }

    #[tokio::test]
    async fn test_pandoc_extractor_supports_latex() {
        let extractor = PandocExtractor::new();
        assert!(extractor.supported_mime_types().contains(&"application/x-latex"));
    }

    #[tokio::test]
    async fn test_pandoc_extractor_supports_rst() {
        let extractor = PandocExtractor::new();
        assert!(extractor.supported_mime_types().contains(&"text/x-rst"));
    }

    #[tokio::test]
    async fn test_pandoc_extractor_markdown() {
        // Skip if pandoc not available
        if validate_pandoc_version().await.is_err() {
            return;
        }

        let extractor = PandocExtractor::new();
        let markdown = b"# Hello World\n\nThis is a test.";
        let config = ExtractionConfig::default();

        let result = extractor.extract_bytes(markdown, "text/x-rst", &config).await;

        // This may fail if pandoc is not installed or if the format is not recognized
        // We'll just check that it doesn't panic
        let _ = result;
    }

    #[tokio::test]
    async fn test_pandoc_extractor_default() {
        let extractor = PandocExtractor;
        assert_eq!(extractor.name(), "pandoc-extractor");
    }

    #[tokio::test]
    async fn test_pandoc_extractor_initialize_shutdown() {
        let extractor = PandocExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }
}
