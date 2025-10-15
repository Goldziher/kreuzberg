//! HTML document extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::ExtractionResult;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;

/// HTML document extractor using html-to-markdown.
pub struct HtmlExtractor;

impl Default for HtmlExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl HtmlExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for HtmlExtractor {
    fn name(&self) -> &str {
        "html-extractor"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn initialize(&mut self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl DocumentExtractor for HtmlExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        // Convert bytes to string
        let html = String::from_utf8_lossy(content).to_string();

        // Convert HTML to markdown
        let markdown = crate::extraction::html::convert_html_to_markdown(&html, None)?;

        Ok(ExtractionResult {
            content: markdown,
            mime_type: mime_type.to_string(),
            metadata: HashMap::new(),
            tables: vec![],
        })
    }

    async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
        let bytes = tokio::fs::read(path).await?;
        self.extract_bytes(&bytes, mime_type, config).await
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["text/html", "application/xhtml+xml"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_extractor_plugin_interface() {
        let mut extractor = HtmlExtractor::new();
        assert_eq!(extractor.name(), "html-extractor");
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_html_extractor_supported_mime_types() {
        let extractor = HtmlExtractor::new();
        let mime_types = extractor.supported_mime_types();
        assert_eq!(mime_types.len(), 2);
        assert!(mime_types.contains(&"text/html"));
        assert!(mime_types.contains(&"application/xhtml+xml"));
    }
}
