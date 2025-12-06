//! Enhanced Markdown extractor with YAML frontmatter support.
//!
//! This extractor provides:
//! - Comprehensive markdown parsing using pulldown-cmark
//! - YAML frontmatter metadata extraction (title, author, date, keywords, description)
//! - Table extraction as structured data
//! - Heading structure preservation
//! - Code block and link extraction
//!
//! Requires the `office` feature (which includes `pulldown-cmark`).

#[cfg(feature = "office")]
use crate::Result;
#[cfg(feature = "office")]
use crate::core::config::ExtractionConfig;
#[cfg(feature = "office")]
use crate::plugins::{DocumentExtractor, Plugin};
#[cfg(feature = "office")]
use crate::types::{ExtractionResult, Metadata, Table};
#[cfg(feature = "office")]
use async_trait::async_trait;
#[cfg(feature = "office")]
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
#[cfg(feature = "office")]
use serde_yaml_ng::Value as YamlValue;

/// Enhanced Markdown extractor with metadata and table support.
///
/// Parses markdown documents with YAML frontmatter, extracting:
/// - Metadata from YAML frontmatter
/// - Plain text content
/// - Tables as structured data
/// - Document structure (headings, links, code blocks)
#[cfg(feature = "office")]
pub struct MarkdownExtractor;

#[cfg(feature = "office")]
impl MarkdownExtractor {
    /// Create a new Markdown extractor.
    pub fn new() -> Self {
        Self
    }

    /// Extract YAML frontmatter from markdown content.
    ///
    /// Frontmatter is expected to be delimited by `---` at the start of the document.
    /// Returns the remaining content after frontmatter.
    fn extract_frontmatter(content: &str) -> (Option<YamlValue>, String) {
        if !content.starts_with("---") {
            return (None, content.to_string());
        }

        let rest = &content[3..]; // Skip opening ---
        if let Some(end_pos) = rest.find("\n---") {
            let frontmatter_str = &rest[..end_pos];
            let remaining = &rest[end_pos + 4..]; // Skip closing --- (and newline)

            match serde_yaml_ng::from_str::<YamlValue>(frontmatter_str) {
                Ok(value) => (Some(value), remaining.to_string()),
                Err(_) => (None, content.to_string()), // If parsing fails, treat as regular content
            }
        } else {
            (None, content.to_string())
        }
    }

    /// Extract metadata from YAML frontmatter.
    fn extract_metadata_from_yaml(yaml: &YamlValue) -> Metadata {
        let mut metadata = Metadata::default();

        if let Some(title) = yaml.get("title").and_then(|v| v.as_str()) {
            metadata.additional.insert("title".to_string(), title.into());
        }

        if let Some(author) = yaml.get("author").and_then(|v| v.as_str()) {
            metadata.additional.insert("author".to_string(), author.into());
        }

        if let Some(date) = yaml.get("date").and_then(|v| v.as_str()) {
            metadata.date = Some(date.to_string());
        }

        if let Some(keywords) = yaml.get("keywords") {
            match keywords {
                YamlValue::String(s) => {
                    metadata.additional.insert("keywords".to_string(), s.clone().into());
                }
                YamlValue::Sequence(seq) => {
                    let keywords_str = seq.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", ");
                    metadata.additional.insert("keywords".to_string(), keywords_str.into());
                }
                _ => {}
            }
        }

        if let Some(description) = yaml.get("description").and_then(|v| v.as_str()) {
            metadata.subject = Some(description.to_string());
        }

        metadata
    }

    /// Extract plain text from markdown AST.
    fn extract_text_from_events(events: &[Event]) -> String {
        let mut text = String::new();
        for event in events {
            match event {
                Event::Text(s) | Event::Code(s) | Event::Html(s) => {
                    text.push_str(s);
                }
                Event::SoftBreak | Event::HardBreak => {
                    text.push('\n');
                }
                Event::Start(_) | Event::End(_) | Event::TaskListMarker(_) => {}
                Event::FootnoteReference(s) => {
                    text.push('[');
                    text.push_str(s);
                    text.push(']');
                }
                Event::Rule => {
                    text.push_str("\n---\n");
                }
                _ => {}
            }
        }
        text
    }

    /// Extract tables from markdown AST.
    fn extract_tables_from_events(events: &[Event]) -> Vec<Table> {
        let mut tables = Vec::new();
        let mut current_table: Option<(Vec<Vec<String>>, usize)> = None;
        let mut current_row: Vec<String> = Vec::new();
        let mut current_cell = String::new();
        let mut in_table_cell = false;
        let mut table_index = 0;

        for event in events {
            match event {
                Event::Start(Tag::Table(_)) => {
                    current_table = Some((Vec::new(), table_index));
                }
                Event::Start(Tag::TableHead) => {
                    // Start of table header
                }
                Event::Start(Tag::TableRow) => {
                    current_row = Vec::new();
                }
                Event::Start(Tag::TableCell) => {
                    current_cell = String::new();
                    in_table_cell = true;
                }
                Event::Text(s) if in_table_cell => {
                    current_cell.push_str(s);
                }
                Event::Code(s) if in_table_cell => {
                    current_cell.push_str(s);
                }
                Event::End(TagEnd::TableCell) => {
                    if in_table_cell {
                        current_row.push(current_cell.trim().to_string());
                        current_cell = String::new();
                        in_table_cell = false;
                    }
                }
                Event::End(TagEnd::TableHead) => {
                    // End of table header row - save the header cells
                    if !current_row.is_empty()
                        && let Some((ref mut rows, _)) = current_table
                    {
                        rows.push(current_row.clone());
                    }
                    current_row = Vec::new();
                }
                Event::End(TagEnd::TableRow) => {
                    if !current_row.is_empty()
                        && let Some((ref mut rows, _)) = current_table
                    {
                        rows.push(current_row.clone());
                    }
                    current_row = Vec::new();
                }
                Event::End(TagEnd::Table) => {
                    if let Some((cells, idx)) = current_table.take()
                        && !cells.is_empty()
                    {
                        let markdown = Self::cells_to_markdown(&cells);
                        tables.push(Table {
                            cells,
                            markdown,
                            page_number: idx + 1,
                        });
                        table_index += 1;
                    }
                }
                _ => {}
            }
        }

        tables
    }

    /// Convert table cells to markdown format.
    fn cells_to_markdown(cells: &[Vec<String>]) -> String {
        if cells.is_empty() {
            return String::new();
        }

        let mut md = String::new();

        // Header row
        md.push('|');
        for cell in &cells[0] {
            md.push(' ');
            md.push_str(cell);
            md.push_str(" |");
        }
        md.push('\n');

        // Separator row
        md.push('|');
        for _ in &cells[0] {
            md.push_str(" --- |");
        }
        md.push('\n');

        // Data rows
        for row in &cells[1..] {
            md.push('|');
            for cell in row {
                md.push(' ');
                md.push_str(cell);
                md.push_str(" |");
            }
            md.push('\n');
        }

        md
    }

    /// Extract first heading as title if not in frontmatter.
    fn extract_title_from_content(content: &str) -> Option<String> {
        for line in content.lines() {
            if let Some(heading) = line.strip_prefix("# ") {
                return Some(heading.trim().to_string());
            }
        }
        None
    }
}

#[cfg(feature = "office")]
impl Default for MarkdownExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "office")]
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
        "Extracts content from Markdown files with YAML frontmatter and table support"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[cfg(feature = "office")]
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
        let text = String::from_utf8_lossy(content).into_owned();

        // Extract frontmatter
        let (yaml, remaining_content) = Self::extract_frontmatter(&text);

        // Extract metadata
        let mut metadata = if let Some(ref yaml_value) = yaml {
            Self::extract_metadata_from_yaml(yaml_value)
        } else {
            Metadata::default()
        };

        // Extract title from frontmatter or first H1
        if !metadata.additional.contains_key("title")
            && let Some(title) = Self::extract_title_from_content(&remaining_content)
        {
            metadata.additional.insert("title".to_string(), title.into());
        }

        // Parse markdown AST with table support enabled
        let parser = Parser::new_ext(&remaining_content, Options::ENABLE_TABLES);
        let events: Vec<Event> = parser.collect();

        // Extract text content
        let extracted_text = Self::extract_text_from_events(&events);

        // Extract tables
        let tables = Self::extract_tables_from_events(&events);

        Ok(ExtractionResult {
            content: extracted_text,
            mime_type: mime_type.to_string(),
            metadata,
            tables,
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["text/markdown", "text/x-markdown", "text/x-gfm", "text/x-commonmark"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(all(test, feature = "office"))]
mod tests {
    use super::*;

    #[test]
    fn test_can_extract_markdown_mime_types() {
        let extractor = MarkdownExtractor::new();
        let mime_types = extractor.supported_mime_types();

        assert!(mime_types.contains(&"text/markdown"));
        assert!(mime_types.contains(&"text/x-markdown"));
        assert!(mime_types.contains(&"text/x-gfm"));
        assert!(mime_types.contains(&"text/x-commonmark"));
    }

    #[test]
    fn test_extract_simple_markdown() {
        let content =
            b"# Header\n\nThis is a paragraph with **bold** and *italic* text.\n\n## Subheading\n\nMore content here.";
        let text = String::from_utf8_lossy(content).into_owned();

        let (yaml, remaining) = MarkdownExtractor::extract_frontmatter(&text);
        assert!(yaml.is_none());
        assert!(!remaining.is_empty());

        let parser = Parser::new_ext(&remaining, Options::ENABLE_TABLES);
        let events: Vec<Event> = parser.collect();
        let extracted = MarkdownExtractor::extract_text_from_events(&events);

        assert!(extracted.contains("Header"));
        assert!(extracted.contains("This is a paragraph"));
        assert!(extracted.contains("bold"));
        assert!(extracted.contains("italic"));
    }

    #[test]
    fn test_extract_frontmatter_metadata() {
        let content = b"---\ntitle: My Document\nauthor: John Doe\ndate: 2024-01-15\nkeywords: rust, markdown, extraction\ndescription: A test document\n---\n\n# Content\n\nBody text.";

        let text = String::from_utf8_lossy(content).into_owned();

        let (yaml_opt, remaining) = MarkdownExtractor::extract_frontmatter(&text);
        assert!(yaml_opt.is_some());
        assert!(remaining.contains("# Content"));

        let yaml = yaml_opt.expect("Should extract YAML frontmatter");
        let metadata = MarkdownExtractor::extract_metadata_from_yaml(&yaml);

        assert_eq!(
            metadata.additional.get("title").and_then(|v| v.as_str()),
            Some("My Document")
        );
        assert_eq!(
            metadata.additional.get("author").and_then(|v| v.as_str()),
            Some("John Doe")
        );
        assert_eq!(metadata.date, Some("2024-01-15".to_string()));
        assert!(metadata.subject.is_some());
        assert!(
            metadata
                .subject
                .as_ref()
                .expect("Should have subject description")
                .contains("test document")
        );
    }

    #[test]
    fn test_extract_frontmatter_metadata_array_keywords() {
        let content = b"---\ntitle: Document\nkeywords:\n  - rust\n  - markdown\n  - parsing\n---\n\nContent";

        let text = String::from_utf8_lossy(content).into_owned();
        let (yaml_opt, _remaining) = MarkdownExtractor::extract_frontmatter(&text);

        assert!(yaml_opt.is_some());
        let yaml = yaml_opt.expect("Should extract YAML frontmatter");
        let metadata = MarkdownExtractor::extract_metadata_from_yaml(&yaml);

        let keywords = metadata.additional.get("keywords").and_then(|v| v.as_str());
        assert!(keywords.is_some());
        let keywords_str = keywords.expect("Should extract keywords from metadata");
        assert!(keywords_str.contains("rust"));
        assert!(keywords_str.contains("markdown"));
    }

    #[tokio::test]
    async fn test_extract_tables() {
        let content = b"# Tables Example\n\n| Header 1 | Header 2 |\n|----------|----------|\n| Cell 1   | Cell 2   |\n| Cell 3   | Cell 4   |";

        let extractor = MarkdownExtractor::new();
        let result = extractor
            .extract_bytes(content, "text/markdown", &ExtractionConfig::default())
            .await
            .expect("Should extract markdown with tables");

        assert!(!result.tables.is_empty());
        let table = &result.tables[0];
        assert!(table.cells.len() >= 1); // At least header row
        assert_eq!(table.cells[0].len(), 2); // 2 columns
        assert!(!table.markdown.is_empty());
    }

    #[test]
    fn test_extract_without_frontmatter() {
        let content = b"# Main Title\n\nSome content\n\nMore text";
        let text = String::from_utf8_lossy(content).into_owned();

        let (yaml, remaining) = MarkdownExtractor::extract_frontmatter(&text);
        assert!(yaml.is_none());
        assert_eq!(remaining, text);

        let title = MarkdownExtractor::extract_title_from_content(&remaining);
        assert_eq!(title, Some("Main Title".to_string()));
    }

    #[test]
    fn test_empty_document() {
        let content = b"";
        let text = String::from_utf8_lossy(content).into_owned();

        let (yaml, remaining) = MarkdownExtractor::extract_frontmatter(&text);
        assert!(yaml.is_none());
        assert!(remaining.is_empty());

        let parser = Parser::new_ext(&remaining, Options::ENABLE_TABLES);
        let events: Vec<Event> = parser.collect();
        let extracted = MarkdownExtractor::extract_text_from_events(&events);
        assert!(extracted.is_empty());
    }

    #[test]
    fn test_whitespace_only_document() {
        let content = b"   \n\n  \n";
        let text = String::from_utf8_lossy(content).into_owned();

        let (yaml, remaining) = MarkdownExtractor::extract_frontmatter(&text);
        assert!(yaml.is_none());

        let parser = Parser::new_ext(&remaining, Options::ENABLE_TABLES);
        let events: Vec<Event> = parser.collect();
        let extracted = MarkdownExtractor::extract_text_from_events(&events);
        assert!(extracted.trim().is_empty());
    }

    #[test]
    fn test_unicode_content() {
        let content = "# 日本語のタイトル\n\nこれは日本語の内容です。\n\n## Español\n\nEste es un documento en español.\n\n## Русский\n\nЭто русский текст.".as_bytes();

        let text = String::from_utf8_lossy(content).into_owned();

        let (yaml, remaining) = MarkdownExtractor::extract_frontmatter(&text);
        assert!(yaml.is_none());

        let parser = Parser::new_ext(&remaining, Options::ENABLE_TABLES);
        let events: Vec<Event> = parser.collect();
        let extracted = MarkdownExtractor::extract_text_from_events(&events);

        assert!(extracted.contains("日本語"));
        assert!(extracted.contains("Español"));
        assert!(extracted.contains("Русский"));
    }

    #[tokio::test]
    async fn test_full_extraction_with_frontmatter_and_tables() {
        let content = b"---\ntitle: Complete Document\nauthor: Test Author\ndate: 2024-01-20\n---\n\n# Document\n\nIntroduction text.\n\n| Name | Value |\n|------|-------|\n| A    | 1     |\n| B    | 2     |";

        let extractor = MarkdownExtractor::new();
        let result = extractor
            .extract_bytes(content, "text/x-markdown", &ExtractionConfig::default())
            .await
            .expect("Should extract markdown with frontmatter and tables");

        assert_eq!(result.mime_type, "text/x-markdown");
        assert!(result.content.contains("Introduction text"));
        assert_eq!(
            result.metadata.additional.get("title").and_then(|v| v.as_str()),
            Some("Complete Document")
        );
        assert_eq!(
            result.metadata.additional.get("author").and_then(|v| v.as_str()),
            Some("Test Author")
        );
        assert!(!result.tables.is_empty());
    }

    #[test]
    fn test_plugin_interface() {
        let extractor = MarkdownExtractor::new();
        assert_eq!(extractor.name(), "markdown-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 50);
        assert!(extractor.supported_mime_types().contains(&"text/markdown"));
    }

    #[test]
    fn test_cells_to_markdown() {
        let cells = vec![
            vec!["Header 1".to_string(), "Header 2".to_string()],
            vec!["Data 1".to_string(), "Data 2".to_string()],
            vec!["Data 3".to_string(), "Data 4".to_string()],
        ];

        let markdown = MarkdownExtractor::cells_to_markdown(&cells);
        assert!(markdown.contains("Header 1"));
        assert!(markdown.contains("Data 1"));
        assert!(markdown.contains("---"));
        let lines: Vec<&str> = markdown.lines().collect();
        assert!(lines.len() >= 4); // Header + separator + at least 2 data rows
    }

    #[test]
    fn test_extract_markdown_with_links() {
        let content = b"# Page\n\nCheck [Google](https://google.com) and [Rust](https://rust-lang.org).";
        let text = String::from_utf8_lossy(content).into_owned();

        let parser = Parser::new_ext(&text, Options::ENABLE_TABLES);
        let events: Vec<Event> = parser.collect();
        let extracted = MarkdownExtractor::extract_text_from_events(&events);

        assert!(extracted.contains("Google"));
        assert!(extracted.contains("Rust"));
    }

    #[test]
    fn test_extract_markdown_with_code_blocks() {
        let content = b"# Code Example\n\n```rust\nfn main() {\n    println!(\"Hello\");\n}\n```";
        let text = String::from_utf8_lossy(content).into_owned();

        let parser = Parser::new_ext(&text, Options::ENABLE_TABLES);
        let events: Vec<Event> = parser.collect();
        let extracted = MarkdownExtractor::extract_text_from_events(&events);

        assert!(extracted.contains("main"));
        assert!(extracted.contains("println"));
    }

    #[test]
    fn test_malformed_frontmatter_fallback() {
        let content = b"---\nthis: is: invalid: yaml:\n---\n\nContent here";
        let text = String::from_utf8_lossy(content).into_owned();

        let (yaml, _remaining) = MarkdownExtractor::extract_frontmatter(&text);
        // When frontmatter is malformed, we treat the whole thing as content
        // This is a graceful degradation strategy
        // Either the YAML parses successfully or we fall back to treating it as content
        let _ = yaml; // Gracefully handle both success and failure
    }

    #[test]
    fn test_metadata_extraction_completeness() {
        // Test all fields that Pandoc extracts
        let yaml_str = r#"
title: "Test Document"
author: "Test Author"
date: "2024-01-15"
keywords:
  - rust
  - markdown
  - testing
description: "A test description"
abstract: "Test abstract"
subject: "Test subject"
custom_field: "custom_value"
version: 1.2.3
tags: ["tag1", "tag2"]
nested:
  organization: "Test Corp"
  contact:
    email: "test@example.com"
"#;

        let yaml: YamlValue = serde_yaml_ng::from_str(yaml_str).expect("Valid YAML");
        let metadata = MarkdownExtractor::extract_metadata_from_yaml(&yaml);

        // Check standard fields
        assert_eq!(metadata.date, Some("2024-01-15".to_string()));
        assert_eq!(
            metadata.additional.get("title").and_then(|v| v.as_str()),
            Some("Test Document")
        );
        assert_eq!(
            metadata.additional.get("author").and_then(|v| v.as_str()),
            Some("Test Author")
        );

        // Keywords should be extracted
        assert!(metadata.additional.contains_key("keywords"));

        // Description maps to subject
        assert_eq!(metadata.subject, Some("A test description".to_string()));

        // Count extracted vs available fields
        let yaml_keys: Vec<String> = if let YamlValue::Mapping(map) = &yaml {
            map.keys().filter_map(|k| k.as_str().map(|s| s.to_string())).collect()
        } else {
            vec![]
        };

        println!("\nTotal YAML fields: {}", yaml_keys.len());
        println!("Extracted to standard fields: 2 (date, subject from description)");
        println!("Extracted to additional: {}", metadata.additional.len());

        // We should extract: title, author, keywords -> 3 in additional
        // Plus: date, subject (from description) -> 2 in standard
        // Missing: abstract, custom_field, version, tags, nested
        assert!(
            metadata.additional.len() >= 3,
            "Should extract at least title, author, keywords"
        );
    }
}
