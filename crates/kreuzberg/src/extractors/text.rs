//! Plain text and Markdown extractors.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::text::parse_text;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::ExtractionResult;
use async_trait::async_trait;

/// Plain text extractor.
///
/// Extracts content from plain text files (.txt).
pub struct PlainTextExtractor;

impl PlainTextExtractor {
    /// Create a new plain text extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for PlainTextExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for PlainTextExtractor {
    fn name(&self) -> &str {
        "plain-text-extractor"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn description(&self) -> &str {
        "Extracts content from plain text files"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[async_trait]
impl DocumentExtractor for PlainTextExtractor {
    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, content, _config),
        fields(
            extractor.name = self.name(),
            content.size_bytes = content.len(),
        )
    ))]
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let text = String::from_utf8_lossy(content).into_owned();
        let text = text.trim_end_matches('\n').trim_end_matches('\r').to_string();
        let line_count = text.lines().count();
        let word_count = text.split_whitespace().count();
        let character_count = text.len();

        Ok(ExtractionResult {
            content: text,
            mime_type: mime_type.to_string(),
            metadata: crate::types::Metadata {
                format: Some(crate::types::FormatMetadata::Text(crate::types::TextMetadata {
                    line_count,
                    word_count,
                    character_count,
                    headers: None,
                    links: None,
                    code_blocks: None,
                })),
                ..Default::default()
            },
            pages: None,
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["text/plain"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

/// Markdown extractor.
///
/// Extracts content from Markdown files (.md, .markdown).
/// Preserves markdown syntax and extracts metadata like headers, links, and code blocks.
pub struct MarkdownExtractor;

impl MarkdownExtractor {
    /// Create a new Markdown extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for MarkdownExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for MarkdownExtractor {
    fn name(&self) -> &str {
        "markdown-extractor"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn description(&self) -> &str {
        "Extracts content from Markdown files with metadata parsing"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[async_trait]
impl DocumentExtractor for MarkdownExtractor {
    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, content, _config),
        fields(
            extractor.name = self.name(),
            content.size_bytes = content.len(),
        )
    ))]
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let text_result = parse_text(content, true)?;

        Ok(ExtractionResult {
            content: text_result.content,
            mime_type: mime_type.to_string(),
            metadata: crate::types::Metadata {
                format: Some(crate::types::FormatMetadata::Text(crate::types::TextMetadata {
                    line_count: text_result.line_count,
                    word_count: text_result.word_count,
                    character_count: text_result.character_count,
                    headers: text_result.headers,
                    links: text_result.links,
                    code_blocks: text_result.code_blocks,
                })),
                ..Default::default()
            },
            pages: None,
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["text/markdown", "text/x-markdown"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plain_text_extractor() {
        let extractor = PlainTextExtractor::new();
        let content = b"Hello, World!\nThis is a test.";
        let config = ExtractionConfig::default();

        let result = extractor.extract_bytes(content, "text/plain", &config).await.unwrap();

        assert_eq!(result.mime_type, "text/plain");
        assert!(result.content.contains("Hello, World!"));
        assert!(result.metadata.format.is_some());
        let text_meta = match result.metadata.format.as_ref().unwrap() {
            crate::types::FormatMetadata::Text(meta) => meta,
            _ => panic!("Expected Text metadata"),
        };
        assert_eq!(text_meta.line_count, 2);
        assert_eq!(text_meta.word_count, 6);
    }

    #[tokio::test]
    async fn test_markdown_extractor() {
        let extractor = MarkdownExtractor::new();
        let content = b"# Header\n\nThis is [a link](https://example.com).\n\n```python\nprint(\"hello\")\n```";
        let config = ExtractionConfig::default();

        let result = extractor
            .extract_bytes(content, "text/markdown", &config)
            .await
            .unwrap();

        assert_eq!(result.mime_type, "text/markdown");
        assert!(result.content.contains("# Header"));
        assert!(result.metadata.format.is_some());
        let text_meta = match result.metadata.format.as_ref().unwrap() {
            crate::types::FormatMetadata::Text(meta) => meta,
            _ => panic!("Expected Text metadata"),
        };
        assert!(text_meta.headers.is_some());
        assert!(text_meta.links.is_some());
        assert!(text_meta.code_blocks.is_some());
    }

    #[test]
    fn test_plain_text_plugin_interface() {
        let extractor = PlainTextExtractor::new();
        assert_eq!(extractor.name(), "plain-text-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.supported_mime_types(), &["text/plain"]);
        assert_eq!(extractor.priority(), 50);
    }

    #[test]
    fn test_markdown_plugin_interface() {
        let extractor = MarkdownExtractor::new();
        assert_eq!(extractor.name(), "markdown-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.supported_mime_types(), &["text/markdown", "text/x-markdown"]);
        assert_eq!(extractor.priority(), 50);
    }
}
