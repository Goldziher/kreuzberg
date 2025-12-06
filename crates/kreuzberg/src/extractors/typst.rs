//! Typst document extractor.
//!
//! Supports: Typst documents (.typ)
//!
//! This extractor provides:
//! - Fast text extraction from Typst source
//! - Metadata extraction from document set rules
//! - Support for headings, code blocks, and links

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata};
use async_trait::async_trait;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

/// Typst document extractor.
///
/// This extractor provides:
/// - Fast text extraction from Typst source documents
/// - Comprehensive metadata extraction
/// - Support for Typst-specific constructs
pub struct TypstExtractor;

impl TypstExtractor {
    /// Create a new Typst extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for TypstExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Compiled regex patterns for metadata extraction (cached for performance)
static TITLE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"title\s*:\s*"([^"]*)""#).expect("Invalid title regex"));

static AUTHOR_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"author\s*:\s*"([^"]*)""#).expect("Invalid author regex"));

static DATE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"date\s*:\s*"([^"]*)""#).expect("Invalid date regex"));

static KEYWORDS_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"keywords\s*:\s*"([^"]*)""#).expect("Invalid keywords regex"));

impl Plugin for TypstExtractor {
    fn name(&self) -> &str {
        "typst-extractor"
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
        "Typst document text extraction with metadata support"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

/// Extract plain text from Typst source, removing code and formatting markers.
///
/// This function processes Typst markup to extract readable text content:
/// - Removes code blocks (```)
/// - Removes function calls and decorators (#)
/// - Preserves text content and headings (=, ==, etc.)
/// - Preserves emphasis and other text formatting
fn extract_plain_text(source: &str) -> String {
    let mut result = String::new();
    let mut in_code_block = false;
    let mut in_raw_block = false;
    let lines = source.lines();

    for line in lines {
        let trimmed = line.trim();

        // Check for code block markers
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            if !in_code_block && !result.is_empty() {
                result.push('\n');
            }
            continue;
        }

        // Check for raw block markers
        if trimmed.starts_with("~~~") {
            in_raw_block = !in_raw_block;
            if !in_raw_block && !result.is_empty() {
                result.push('\n');
            }
            continue;
        }

        // Skip content inside code/raw blocks
        if in_code_block || in_raw_block {
            continue;
        }

        // Skip empty lines but preserve paragraph breaks
        if trimmed.is_empty() {
            if !result.is_empty() && !result.ends_with("\n\n") {
                result.push('\n');
            }
            continue;
        }

        // Skip comments
        if trimmed.starts_with("//") {
            continue;
        }

        // Remove function calls and commands (lines starting with #)
        if trimmed.starts_with('#') {
            // Still add to result if it's content-related
            // But skip most # directives
            if !trimmed.starts_with("#set") && !trimmed.starts_with("#let") && !trimmed.starts_with("#show") {
                let content = trimmed.trim_start_matches('#');
                if !content.is_empty() {
                    result.push_str(content);
                    result.push('\n');
                }
            }
            continue;
        }

        // Extract heading (remove = markers but keep content)
        if trimmed.starts_with('=') {
            let heading_level = trimmed.chars().take_while(|&c| c == '=').count();
            let heading_text = trimmed[heading_level..].trim();
            if !heading_text.is_empty() {
                result.push_str(heading_text);
                result.push('\n');
            }
            continue;
        }

        // Regular content line - add as-is
        result.push_str(trimmed);
        result.push('\n');
    }

    // Clean up multiple newlines
    while result.contains("\n\n\n") {
        result = result.replace("\n\n\n", "\n\n");
    }

    result.trim().to_string()
}

/// Extract metadata from Typst source.
///
/// Looks for common metadata patterns like:
/// - #set document(title: "...")
/// - #set document(author: "...")
/// - #set document(date: "...")
/// - #set document(keywords: "...")
///
/// Uses pre-compiled static regex patterns for efficient repeated extraction.
fn extract_metadata_from_source(source: &str) -> HashMap<String, serde_json::Value> {
    let mut metadata = HashMap::new();

    // Extract title using cached regex
    if let Some(cap) = TITLE_RE.captures(source)
        && let Some(title) = cap.get(1)
    {
        metadata.insert(
            "title".to_string(),
            serde_json::Value::String(title.as_str().to_string()),
        );
    }

    // Extract author using cached regex
    if let Some(cap) = AUTHOR_RE.captures(source)
        && let Some(author) = cap.get(1)
    {
        let author_str = author.as_str().to_string();
        metadata.insert(
            "authors".to_string(),
            serde_json::Value::Array(vec![serde_json::Value::String(author_str.clone())]),
        );
        metadata.insert("created_by".to_string(), serde_json::Value::String(author_str));
    }

    // Extract date using cached regex
    if let Some(cap) = DATE_RE.captures(source)
        && let Some(date) = cap.get(1)
    {
        metadata.insert("date".to_string(), serde_json::Value::String(date.as_str().to_string()));
    }

    // Extract keywords using cached regex
    if let Some(cap) = KEYWORDS_RE.captures(source)
        && let Some(keywords) = cap.get(1)
    {
        metadata.insert(
            "keywords".to_string(),
            serde_json::Value::String(keywords.as_str().to_string()),
        );
    }

    metadata
}

#[async_trait]
impl DocumentExtractor for TypstExtractor {
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
        // Convert bytes to string
        let typst_text = std::str::from_utf8(content)
            .map(|s| s.to_string())
            .unwrap_or_else(|_| String::from_utf8_lossy(content).to_string());

        // Extract text content
        let extracted_text = extract_plain_text(&typst_text);

        // Extract metadata from the source string
        let mut metadata_map = extract_metadata_from_source(&typst_text);

        // Build metadata struct
        let mut metadata = Metadata { ..Default::default() };

        // Populate common fields
        if let Some(serde_json::Value::String(title)) = metadata_map.get("title") {
            metadata.subject = Some(title.clone());
            metadata_map.remove("title");
        }

        if let Some(serde_json::Value::String(date)) = metadata_map.get("date") {
            metadata.date = Some(date.clone());
            metadata_map.remove("date");
        }

        metadata.additional = metadata_map;

        Ok(ExtractionResult {
            content: extracted_text,
            mime_type: mime_type.to_string(),
            metadata,
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["text/x-typst", "application/x-typst"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_extract_typst_mime_types() {
        let extractor = TypstExtractor::new();
        let mime_types = extractor.supported_mime_types();
        assert!(mime_types.contains(&"text/x-typst"));
        assert!(mime_types.contains(&"application/x-typst"));
        assert_eq!(mime_types.len(), 2);
    }

    #[test]
    fn test_extractor_metadata() {
        let extractor = TypstExtractor::new();
        assert_eq!(extractor.name(), "typst-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 50);
    }

    #[test]
    fn test_extractor_initialize_shutdown() {
        let extractor = TypstExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_extractor_default() {
        let extractor = TypstExtractor::default();
        assert_eq!(extractor.name(), "typst-extractor");
    }

    #[tokio::test]
    async fn test_extract_simple_typst() {
        let extractor = TypstExtractor::new();
        let content = b"= Heading\n\nSome text content here.";
        let config = ExtractionConfig::default();

        let result = extractor
            .extract_bytes(content, "text/x-typst", &config)
            .await
            .expect("Failed to extract simple Typst document");

        assert_eq!(result.mime_type, "text/x-typst");
        assert!(result.content.contains("Heading"));
        assert!(result.content.contains("Some text content"));
    }

    #[tokio::test]
    async fn test_extract_document_metadata() {
        let extractor = TypstExtractor::new();
        let content = br#"
#set document(
  title: "Test Document",
  author: "John Doe",
  date: "2024-01-15",
  keywords: "test, document, typst"
)

= Introduction

This is the main content.
"#;
        let config = ExtractionConfig::default();

        let result = extractor
            .extract_bytes(content, "text/x-typst", &config)
            .await
            .expect("Failed to extract document metadata");

        assert_eq!(result.mime_type, "text/x-typst");
        assert!(result.content.contains("Introduction"));
        assert!(result.content.contains("main content"));

        // Check metadata
        let additional = &result.metadata.additional;

        // Title should be in subject field
        assert_eq!(result.metadata.subject, Some("Test Document".to_string()));

        // Date should be in date field
        assert_eq!(result.metadata.date, Some("2024-01-15".to_string()));

        // Author should be in additional as arrays and string
        assert!(additional.contains_key("authors"));
        assert!(additional.contains_key("created_by"));

        // Keywords should be in additional
        assert!(additional.contains_key("keywords"));
    }

    #[tokio::test]
    async fn test_extract_headings() {
        let extractor = TypstExtractor::new();
        let content = b"= Level 1\n== Level 2\n=== Level 3\n\nContent under level 3.";
        let config = ExtractionConfig::default();

        let result = extractor
            .extract_bytes(content, "text/x-typst", &config)
            .await
            .expect("Failed to extract headings");

        assert!(result.content.contains("Level 1"));
        assert!(result.content.contains("Level 2"));
        assert!(result.content.contains("Level 3"));
        assert!(result.content.contains("Content under level 3"));
    }

    #[tokio::test]
    async fn test_extract_without_metadata() {
        let extractor = TypstExtractor::new();
        let content = b"= Simple Document\n\nThis has no metadata.";
        let config = ExtractionConfig::default();

        let result = extractor
            .extract_bytes(content, "text/x-typst", &config)
            .await
            .expect("Failed to extract document without metadata");

        assert_eq!(result.mime_type, "text/x-typst");
        assert!(result.content.contains("Simple Document"));
        assert!(result.content.contains("no metadata"));

        // Should have empty or minimal metadata
        assert!(result.metadata.subject.is_none());
    }

    #[tokio::test]
    async fn test_empty_document() {
        let extractor = TypstExtractor::new();
        let content = b"";
        let config = ExtractionConfig::default();

        let result = extractor
            .extract_bytes(content, "text/x-typst", &config)
            .await
            .expect("Failed to extract empty document");

        assert_eq!(result.mime_type, "text/x-typst");
        assert_eq!(result.content, "");
    }

    #[tokio::test]
    async fn test_unicode_content() {
        let extractor = TypstExtractor::new();
        let content = "= Document with Unicode\n\nHere's some international text:\n- Français: café\n- Deutsch: äöü\n- 中文: 你好\n- Emoji: 🎉 🚀"
            .as_bytes();
        let config = ExtractionConfig::default();

        let result = extractor
            .extract_bytes(content, "text/x-typst", &config)
            .await
            .expect("Failed to extract Unicode content");

        assert!(result.content.contains("Français"));
        assert!(result.content.contains("café"));
        assert!(result.content.contains("你好"));
        assert!(result.content.contains("🎉"));
    }

    #[tokio::test]
    async fn test_extract_with_code_blocks() {
        let extractor = TypstExtractor::new();
        let content = b"= Code Example\n\nHere's some code:\n\n```rust\nfn main() {\n    println!(\"Hello, world!\");\n}\n```\n\nEnd of example.";
        let config = ExtractionConfig::default();

        let result = extractor
            .extract_bytes(content, "text/x-typst", &config)
            .await
            .expect("Failed to extract code blocks");

        assert!(result.content.contains("Code Example"));
        // Code block content should be excluded
        assert!(!result.content.contains("fn main"));
        assert!(!result.content.contains("println"));
        assert!(result.content.contains("End of example"));
    }

    #[test]
    fn test_extract_plain_text_simple() {
        let text = "Hello, world!";
        let result = extract_plain_text(text);
        assert_eq!(result, "Hello, world!");
    }

    #[test]
    fn test_extract_plain_text_with_heading() {
        let text = "= My Heading\n\nSome content below.";
        let result = extract_plain_text(text);
        assert!(result.contains("My Heading"));
        assert!(result.contains("Some content below"));
    }

    #[test]
    fn test_extract_plain_text_removes_code_blocks() {
        let text = "Start\n\n```\ncode here\n```\n\nEnd";
        let result = extract_plain_text(text);
        assert!(result.contains("Start"));
        assert!(!result.contains("code here"));
        assert!(result.contains("End"));
    }

    #[test]
    fn test_extract_metadata_empty() {
        let metadata = extract_metadata_from_source("");
        assert!(metadata.is_empty());
    }

    #[test]
    fn test_extract_metadata_title_only() {
        let source = r#"#set document(title: "My Title")"#;
        let metadata = extract_metadata_from_source(source);
        assert!(metadata.contains_key("title"));
        if let Some(serde_json::Value::String(title)) = metadata.get("title") {
            assert_eq!(title, "My Title");
        } else {
            panic!("Expected title in metadata");
        }
    }

    #[test]
    fn test_extract_metadata_all_fields() {
        let source = r#"
#set document(
  title: "Test Doc",
  author: "Jane Doe",
  date: "2024-12-06",
  keywords: "rust, testing"
)
"#;
        let metadata = extract_metadata_from_source(source);
        assert!(metadata.contains_key("title"));
        assert!(metadata.contains_key("authors"));
        assert!(metadata.contains_key("created_by"));
        assert!(metadata.contains_key("date"));
        assert!(metadata.contains_key("keywords"));
    }
}
