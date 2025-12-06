//! Native Org Mode extractor using the `orgize` library.
//!
//! This extractor provides comprehensive Org Mode document parsing and extraction,
//! replacing Pandoc for .org files. It extracts:
//!
//! - **Metadata**: #+TITLE, #+AUTHOR, #+DATE, #+KEYWORDS from document preamble
//! - **Properties**: :PROPERTIES: drawers with additional metadata
//! - **Headings**: Multi-level headings with proper hierarchy (* to *****)
//! - **Content**: Paragraphs and text blocks
//! - **Lists**: Ordered, unordered, and nested lists
//! - **Code blocks**: #+BEGIN_SRC...#+END_SRC with language specification
//! - **Tables**: Pipe tables (| cell | cell |) converted to Table structs
//! - **Inline formatting**: *bold*, /italic/, =code=, ~verbatim~, [[links]]
//!
//! Requires the `office` feature.

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
use std::collections::HashMap;

#[cfg(feature = "office")]
use orgize::{Element, Org};

/// Org Mode document extractor.
///
/// Provides native Rust-based Org Mode extraction using the `orgize` library,
/// extracting structured content and metadata without external dependencies like Pandoc.
#[cfg(feature = "office")]
pub struct OrgModeExtractor;

#[cfg(feature = "office")]
impl OrgModeExtractor {
    /// Create a new Org Mode extractor.
    pub fn new() -> Self {
        Self
    }

    /// Extract metadata and content from Org document in a single pass.
    ///
    /// Combines metadata extraction from directives and full document parsing
    /// into one efficient operation. Looks for:
    /// - #+TITLE: → title
    /// - #+AUTHOR: → author/authors
    /// - #+DATE: → date
    /// - #+KEYWORDS: → keywords
    /// - Other #+DIRECTIVE: entries
    ///
    /// Also extracts document structure and content in parallel.
    fn extract_metadata_and_content(org_text: &str, org: &Org) -> (Metadata, String) {
        let mut metadata = Metadata::default();
        let mut additional = HashMap::new();

        // Parse directives from the raw content (single pass through first ~100 lines)
        for line in org_text.lines().take(100) {
            // Only check first ~100 lines for directives (they typically come early)
            let trimmed = line.trim();

            if let Some(rest) = trimmed.strip_prefix("#+TITLE:") {
                let value = rest.trim().to_string();
                additional.insert("title".to_string(), serde_json::json!(value));
            } else if let Some(rest) = trimmed.strip_prefix("#+AUTHOR:") {
                let value = rest.trim().to_string();
                additional.insert("author".to_string(), serde_json::json!(&value));
                additional.insert("authors".to_string(), serde_json::json!(vec![value]));
            } else if let Some(rest) = trimmed.strip_prefix("#+DATE:") {
                let value = rest.trim().to_string();
                metadata.date = Some(value.clone());
                additional.insert("date".to_string(), serde_json::json!(value));
            } else if let Some(rest) = trimmed.strip_prefix("#+KEYWORDS:") {
                let value = rest.trim();
                let keywords: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
                additional.insert("keywords".to_string(), serde_json::json!(keywords));
            } else if let Some(rest) = trimmed.strip_prefix("#+") {
                // Generic directive extraction
                if let Some((key, val)) = rest.split_once(':') {
                    let key_lower = key.trim().to_lowercase();
                    let value = val.trim();
                    if !key_lower.is_empty() && !value.is_empty() {
                        additional.insert(format!("directive_{}", key_lower), serde_json::json!(value));
                    }
                }
            }
        }

        metadata.additional = additional;

        // Extract content in parallel with metadata extraction
        let content = Self::extract_content(org);

        (metadata, content)
    }

    /// Extract all content from an Org document using event iteration.
    ///
    /// Uses orgize's event-based iteration to handle:
    /// - Headings with proper hierarchy
    /// - Paragraphs
    /// - Lists (both ordered and unordered)
    /// - Code blocks with language info
    /// - Tables as structured data
    /// - Inline formatting markers
    fn extract_content(org: &Org) -> String {
        let mut content = String::new();
        let mut in_src_block = false;
        let mut list_stack = Vec::new(); // Track list depth

        for event in org.iter() {
            match event {
                orgize::Event::Start(Element::Headline { level }) => {
                    let hashes = "#".repeat(std::cmp::min(*level, 6));
                    content.push_str(&format!("{} ", hashes));
                }
                orgize::Event::End(Element::Headline { .. }) => {
                    content.push('\n');
                }
                orgize::Event::Start(Element::Title(_)) => {
                    // Title content follows - just continue
                }
                orgize::Event::End(Element::Title(_)) => {
                    content.push('\n');
                }
                orgize::Event::Start(Element::Paragraph { .. }) => {
                    // Paragraph start - content follows
                }
                orgize::Event::End(Element::Paragraph { .. }) => {
                    content.push_str("\n\n");
                }
                orgize::Event::Start(Element::List(list)) => {
                    // Start new list - track if ordered or unordered
                    list_stack.push((0, !list.ordered));
                }
                orgize::Event::End(Element::List(_)) => {
                    list_stack.pop();
                    if list_stack.is_empty() {
                        content.push('\n');
                    }
                }
                orgize::Event::Start(Element::ListItem(_)) => {
                    let depth = list_stack.len().saturating_sub(1);
                    let indent = "  ".repeat(depth);
                    if let Some((counter, is_unordered)) = list_stack.last_mut() {
                        *counter += 1;
                        if *is_unordered {
                            content.push_str(&format!("{}- ", indent));
                        } else {
                            content.push_str(&format!("{}{}. ", indent, counter));
                        }
                    } else {
                        content.push_str(&format!("{}- ", indent));
                    }
                }
                orgize::Event::End(Element::ListItem(_)) => {
                    content.push('\n');
                }
                orgize::Event::Start(Element::SourceBlock(sb)) => {
                    in_src_block = true;
                    let lang = sb.language.as_ref();
                    content.push_str(&format!("```{}\n", lang));
                }
                orgize::Event::End(Element::SourceBlock { .. }) => {
                    in_src_block = false;
                    content.push_str("```\n\n");
                }
                orgize::Event::Start(Element::Table(_)) => {
                    // Table handling - rows follow
                }
                orgize::Event::End(Element::Table(_)) => {
                    content.push('\n');
                }
                orgize::Event::Start(Element::TableRow(_)) => {
                    content.push('|');
                }
                orgize::Event::End(Element::TableRow(_)) => {
                    content.push('\n');
                }
                orgize::Event::Start(Element::TableCell(_)) => {
                    content.push(' ');
                }
                orgize::Event::End(Element::TableCell(_)) => {
                    content.push_str(" |");
                }
                orgize::Event::Start(Element::Bold) => {
                    content.push('*');
                }
                orgize::Event::End(Element::Bold) => {
                    content.push('*');
                }
                orgize::Event::Start(Element::Italic) => {
                    content.push('/');
                }
                orgize::Event::End(Element::Italic) => {
                    content.push('/');
                }
                orgize::Event::Start(Element::Underline) => {
                    content.push('_');
                }
                orgize::Event::End(Element::Underline) => {
                    content.push('_');
                }
                orgize::Event::Start(Element::Strike) => {
                    content.push_str("~~");
                }
                orgize::Event::End(Element::Strike) => {
                    content.push_str("~~");
                }
                orgize::Event::Start(Element::Code { value }) => {
                    content.push('`');
                    content.push_str(value);
                    content.push('`');
                }
                orgize::Event::End(Element::Code { .. }) => {
                    // Already added content in Start event
                }
                orgize::Event::Start(Element::Verbatim { value }) => {
                    content.push('~');
                    content.push_str(value);
                    content.push('~');
                }
                orgize::Event::End(Element::Verbatim { .. }) => {
                    // Already added content in Start event
                }
                orgize::Event::Start(Element::Link(link)) => {
                    // Use description if available, otherwise fall back to path
                    // [[url][description]] → description
                    // [[url]] → url
                    let link_text = if let Some(desc) = &link.desc { desc } else { &link.path };
                    content.push_str(&format!("[{}]", link_text));
                }
                orgize::Event::End(Element::Link { .. }) => {
                    // Handled in Start
                }
                orgize::Event::Start(Element::Text { value }) => {
                    // Add text content
                    if !in_src_block {
                        // Clean up whitespace but preserve intentional breaks
                        let cleaned = value.trim_end_matches('\n');
                        if !cleaned.is_empty() {
                            content.push_str(cleaned);
                        }
                    } else {
                        content.push_str(value);
                    }
                }
                orgize::Event::End(Element::Text { .. }) => {
                    // Text is handled in Start
                }
                orgize::Event::Start(Element::ExampleBlock(_)) => {
                    content.push_str("> ");
                }
                orgize::Event::End(Element::ExampleBlock { .. }) => {
                    content.push('\n');
                }
                orgize::Event::Start(Element::QuoteBlock(_)) => {
                    content.push_str("> ");
                }
                orgize::Event::End(Element::QuoteBlock(_)) => {
                    content.push('\n');
                }
                _ => {
                    // Ignore other elements like comments, affiliated keywords, etc.
                }
            }
        }

        content.trim().to_string()
    }

    /// Extract tables from an Org document.
    ///
    /// Parses table elements and converts them to Table structs with markdown format.
    fn extract_tables(org: &Org) -> Vec<Table> {
        let mut tables = Vec::new();
        let mut current_table: Vec<Vec<String>> = Vec::new();
        let mut current_row: Vec<String> = Vec::new();
        let mut current_cell = String::new();
        let mut in_table = false;

        for event in org.iter() {
            match event {
                orgize::Event::Start(Element::Table(_)) => {
                    in_table = true;
                    current_table.clear();
                }
                orgize::Event::End(Element::Table(_)) => {
                    in_table = false;
                    if !current_table.is_empty() {
                        let markdown = Self::cells_to_markdown(&current_table);
                        tables.push(Table {
                            cells: current_table.clone(),
                            markdown,
                            page_number: 1,
                        });
                        current_table.clear();
                    }
                }
                orgize::Event::Start(Element::TableRow(_)) if in_table => {
                    current_row.clear();
                }
                orgize::Event::End(Element::TableRow(_)) if in_table => {
                    if !current_row.is_empty() {
                        current_table.push(current_row.clone());
                        current_row.clear();
                    }
                }
                orgize::Event::Start(Element::TableCell(_)) if in_table => {
                    current_cell.clear();
                }
                orgize::Event::End(Element::TableCell(_)) if in_table => {
                    current_row.push(current_cell.trim().to_string());
                    current_cell.clear();
                }
                orgize::Event::Start(Element::Text { value }) if in_table => {
                    current_cell.push_str(value);
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

        for (row_idx, row) in cells.iter().enumerate() {
            md.push('|');
            for cell in row {
                md.push(' ');
                md.push_str(cell);
                md.push_str(" |");
            }
            md.push('\n');

            // Add separator after header (first row)
            if row_idx == 0 && cells.len() > 1 {
                md.push('|');
                for _ in row {
                    md.push_str(" --- |");
                }
                md.push('\n');
            }
        }

        md
    }
}

#[cfg(feature = "office")]
impl Default for OrgModeExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "office")]
impl Plugin for OrgModeExtractor {
    fn name(&self) -> &str {
        "orgmode-extractor"
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
        "Native Rust extractor for Org Mode documents with comprehensive metadata extraction"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[cfg(feature = "office")]
#[async_trait]
impl DocumentExtractor for OrgModeExtractor {
    #[cfg_attr(
        feature = "otel",
        tracing::instrument(
            skip(self, content, _config),
            fields(
                extractor.name = self.name(),
                content.size_bytes = content.len(),
            )
        )
    )]
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        // Convert bytes to string
        let org_text = String::from_utf8_lossy(content).into_owned();

        // Parse Org document
        let org = Org::parse(&org_text);

        // Extract metadata and content in a single combined pass
        let (metadata, extracted_content) = Self::extract_metadata_and_content(&org_text, &org);

        // Extract tables
        let tables = Self::extract_tables(&org);

        Ok(ExtractionResult {
            content: extracted_content,
            mime_type: mime_type.to_string(),
            metadata,
            tables,
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["text/x-org", "text/org", "application/x-org"]
    }

    fn priority(&self) -> i32 {
        // Higher than Pandoc (40) to prefer native implementation
        50
    }
}

#[cfg(all(test, feature = "office"))]
mod tests {
    use super::*;

    #[test]
    fn test_orgmode_extractor_plugin_interface() {
        let extractor = OrgModeExtractor::new();
        assert_eq!(extractor.name(), "orgmode-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 50);
        assert!(!extractor.supported_mime_types().is_empty());
    }

    #[test]
    fn test_orgmode_extractor_supports_text_x_org() {
        let extractor = OrgModeExtractor::new();
        assert!(extractor.supported_mime_types().contains(&"text/x-org"));
    }

    #[test]
    fn test_orgmode_extractor_default() {
        let extractor = OrgModeExtractor::default();
        assert_eq!(extractor.name(), "orgmode-extractor");
    }

    #[test]
    fn test_orgmode_extractor_initialize_shutdown() {
        let extractor = OrgModeExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_extract_metadata_with_title() {
        let org_text = "#+TITLE: Test Document\n\nContent here.";
        let org = Org::parse(org_text);
        let (metadata, _) = OrgModeExtractor::extract_metadata_and_content(org_text, &org);

        assert!(metadata.additional.get("title").and_then(|v| v.as_str()).is_some());
    }

    #[test]
    fn test_extract_metadata_with_author() {
        let org_text = "#+AUTHOR: John Doe\n\nContent here.";
        let org = Org::parse(org_text);
        let (metadata, _) = OrgModeExtractor::extract_metadata_and_content(org_text, &org);

        assert!(metadata.additional.get("author").and_then(|v| v.as_str()).is_some());
    }

    #[test]
    fn test_extract_metadata_with_date() {
        let org_text = "#+DATE: 2024-01-15\n\nContent here.";
        let org = Org::parse(org_text);
        let (metadata, _) = OrgModeExtractor::extract_metadata_and_content(org_text, &org);

        assert_eq!(metadata.date, Some("2024-01-15".to_string()));
    }

    #[test]
    fn test_extract_metadata_with_keywords() {
        let org_text = "#+KEYWORDS: rust, org-mode, parsing\n\nContent here.";
        let org = Org::parse(org_text);
        let (metadata, _) = OrgModeExtractor::extract_metadata_and_content(org_text, &org);

        let keywords = metadata.additional.get("keywords").and_then(|v| v.as_array());
        assert!(keywords.is_some());
    }

    #[test]
    fn test_extract_content_with_headings() {
        let org_text = "* Heading 1\n\nSome content.\n\n** Heading 2\n\nMore content.";
        let org = Org::parse(org_text);
        let content = OrgModeExtractor::extract_content(&org);

        assert!(content.contains("Heading 1"));
        assert!(content.contains("Heading 2"));
        assert!(content.contains("Some content"));
        assert!(content.contains("More content"));
    }

    #[test]
    fn test_extract_content_with_paragraphs() {
        let org_text = "First paragraph.\n\nSecond paragraph.";
        let org = Org::parse(org_text);
        let content = OrgModeExtractor::extract_content(&org);

        assert!(content.contains("First paragraph"));
        assert!(content.contains("Second paragraph"));
    }

    #[test]
    fn test_extract_content_with_lists() {
        let org_text = "- Item 1\n- Item 2\n- Item 3";
        let org = Org::parse(org_text);
        let content = OrgModeExtractor::extract_content(&org);

        assert!(content.contains("Item 1"));
        assert!(content.contains("Item 2"));
        assert!(content.contains("Item 3"));
    }

    #[test]
    fn test_cells_to_markdown_format() {
        let cells = vec![
            vec!["Name".to_string(), "Age".to_string()],
            vec!["Alice".to_string(), "30".to_string()],
            vec!["Bob".to_string(), "25".to_string()],
        ];

        let markdown = OrgModeExtractor::cells_to_markdown(&cells);
        assert!(markdown.contains("Name"));
        assert!(markdown.contains("Age"));
        assert!(markdown.contains("Alice"));
        assert!(markdown.contains("Bob"));
        assert!(markdown.contains("---"));
    }

    #[test]
    fn test_orgmode_extractor_supported_mime_types() {
        let extractor = OrgModeExtractor::new();
        let supported = extractor.supported_mime_types();
        assert!(supported.contains(&"text/x-org"));
    }

    #[test]
    fn test_link_with_description() {
        let org_text = r#"* Links Test

[[http://att.com/][AT&T]]
"#;
        let org = Org::parse(org_text);
        let content = OrgModeExtractor::extract_content(&org);

        // Link description should be extracted, not the URL
        assert!(content.contains("AT&T"), "Should contain link description 'AT&T'");
        // The URL should still be present in the extracted content, but wrapped in brackets
        // The actual format depends on how orgize structures the link
    }

    #[test]
    fn test_link_without_description() {
        let org_text = r#"* Links Test

[[https://example.com]]
"#;
        let org = Org::parse(org_text);
        let content = OrgModeExtractor::extract_content(&org);

        // Link path should be used when no description is provided
        assert!(
            content.contains("example.com"),
            "Should contain link path when no description provided"
        );
    }

    #[test]
    fn test_link_with_ampersand_in_description() {
        let org_text = r#"* Company Links

[[http://att.com/][AT&T Company]]
"#;
        let org = Org::parse(org_text);
        let content = OrgModeExtractor::extract_content(&org);

        // Description with ampersand should be preserved
        assert!(
            content.contains("AT&T"),
            "Should preserve ampersand in link description"
        );
    }

    #[test]
    fn test_multiple_links_with_mixed_descriptions() {
        let org_text = r#"* Multiple Links

[[https://example.com][Example Link]]

[[https://example.org]]

[[mailto:test@example.com][Contact]]
"#;
        let org = Org::parse(org_text);
        let content = OrgModeExtractor::extract_content(&org);

        // First link with description
        assert!(content.contains("Example Link"));
        // Second link without description
        assert!(content.contains("example.org"));
        // Third link with description
        assert!(content.contains("Contact"));
    }

    #[test]
    fn test_link_description_priority_over_url() {
        // This is the CRITICAL test - ensures description is used over path
        let org_text = r#"[[http://att.com/][AT&T]]"#;
        let org = Org::parse(org_text);
        let content = OrgModeExtractor::extract_content(&org);

        // The description "AT&T" should be in the output
        assert!(content.contains("AT&T"), "Description should be prioritized over URL");
        // The extracted content should contain the description wrapped in brackets: [AT&T]
        assert!(
            content.contains("[AT&T]"),
            "Link should be formatted as [description] when description exists"
        );
    }
}
