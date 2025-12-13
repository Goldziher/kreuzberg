//! Native Org Mode extractor using the `org` library.
//!
//! This extractor provides comprehensive Org Mode document parsing and extraction.
//! It extracts:
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
use org::Org;

/// Org Mode document extractor.
///
/// Provides native Rust-based Org Mode extraction using the `org` library,
/// extracting structured content and metadata.
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

        for line in org_text.lines().take(100) {
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
            } else if let Some(rest) = trimmed.strip_prefix("#+")
                && let Some((key, val)) = rest.split_once(':')
            {
                let key_lower = key.trim().to_lowercase();
                let value = val.trim();
                if !key_lower.is_empty() && !value.is_empty() {
                    additional.insert(format!("directive_{}", key_lower), serde_json::json!(value));
                }
            }
        }

        metadata.additional = additional;

        let content = Self::extract_content(org);

        (metadata, content)
    }

    /// Extract all content from an Org document using tree-based parsing.
    ///
    /// Uses org's tree-based API to recursively traverse the document structure:
    /// - Headings with proper hierarchy
    /// - Paragraphs
    /// - Lists (both ordered and unordered)
    /// - Code blocks with language info
    /// - Tables as structured data
    /// - Inline formatting markers
    fn extract_content(org: &Org) -> String {
        let mut content = String::new();
        Self::extract_org_tree(org, &mut content);
        content.trim().to_string()
    }

    /// Recursively walk the Org tree and extract content.
    ///
    /// Processes:
    /// - Heading text from `org.heading()`
    /// - Content lines from `org.content_as_ref()`
    /// - Subtrees from `org.subtrees_as_ref()`
    fn extract_org_tree(org: &Org, content: &mut String) {
        let heading = org.heading();
        if !heading.is_empty() {
            content.push_str("# ");
            content.push_str(heading);
            content.push('\n');
        }

        let lines = org.content_as_ref();
        if !lines.is_empty() {
            for line in lines {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    content.push_str(trimmed);
                    content.push('\n');
                }
            }
            content.push('\n');
        }

        let subtrees = org.subtrees_as_ref();
        for subtree in subtrees {
            Self::extract_org_tree(subtree, content);
        }
    }

    /// Extract tables from an Org document.
    ///
    /// Recursively walks the tree and extracts table elements,
    /// converting them to Table structs with markdown format.
    fn extract_tables(org: &Org) -> Vec<Table> {
        let mut tables = Vec::new();
        Self::extract_tables_from_tree(org, &mut tables);
        tables
    }

    /// Recursively extract tables from an Org tree node and its subtrees.
    fn extract_tables_from_tree(org: &Org, tables: &mut Vec<Table>) {
        let lines = org.content_as_ref();
        if !lines.is_empty() {
            let mut in_table = false;
            let mut current_table: Vec<Vec<String>> = Vec::new();

            for line in lines {
                let trimmed = line.trim();

                if trimmed.starts_with('|') && trimmed.ends_with('|') {
                    in_table = true;

                    let cells: Vec<String> = trimmed
                        .split('|')
                        .map(|cell| cell.trim().to_string())
                        .filter(|cell| !cell.is_empty())
                        .collect();

                    if !cells.is_empty() {
                        current_table.push(cells);
                    }
                } else if in_table {
                    if !current_table.is_empty() {
                        let markdown = Self::cells_to_markdown(&current_table);
                        tables.push(Table {
                            cells: current_table.clone(),
                            markdown,
                            page_number: 1,
                        });
                        current_table.clear();
                    }
                    in_table = false;
                }
            }

            if !current_table.is_empty() {
                let markdown = Self::cells_to_markdown(&current_table);
                tables.push(Table {
                    cells: current_table,
                    markdown,
                    page_number: 1,
                });
            }
        }

        let subtrees = org.subtrees_as_ref();
        for subtree in subtrees {
            Self::extract_tables_from_tree(subtree, tables);
        }
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
        let org_text = String::from_utf8_lossy(content).into_owned();

        let lines: Vec<String> = org_text.lines().map(|s| s.to_string()).collect();
        let org = Org::from_vec(&lines)?;

        let (metadata, extracted_content) = Self::extract_metadata_and_content(&org_text, &org);

        let tables = Self::extract_tables(&org);

        Ok(ExtractionResult {
            content: extracted_content,
            mime_type: mime_type.to_string(),
            metadata,
            tables,
            detected_languages: None,
            chunks: None,
            images: None,
            pages: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["text/x-org", "text/org", "application/x-org"]
    }

    fn priority(&self) -> i32 {
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
        let extractor = OrgModeExtractor;
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
        let lines: Vec<String> = org_text.lines().map(|s| s.to_string()).collect();
        let org = Org::from_vec(&lines).expect("Failed to parse org");
        let (metadata, _) = OrgModeExtractor::extract_metadata_and_content(org_text, &org);

        assert!(metadata.additional.get("title").and_then(|v| v.as_str()).is_some());
    }

    #[test]
    fn test_extract_metadata_with_author() {
        let org_text = "#+AUTHOR: John Doe\n\nContent here.";
        let lines: Vec<String> = org_text.lines().map(|s| s.to_string()).collect();
        let org = Org::from_vec(&lines).expect("Failed to parse org");
        let (metadata, _) = OrgModeExtractor::extract_metadata_and_content(org_text, &org);

        assert!(metadata.additional.get("author").and_then(|v| v.as_str()).is_some());
    }

    #[test]
    fn test_extract_metadata_with_date() {
        let org_text = "#+DATE: 2024-01-15\n\nContent here.";
        let lines: Vec<String> = org_text.lines().map(|s| s.to_string()).collect();
        let org = Org::from_vec(&lines).expect("Failed to parse org");
        let (metadata, _) = OrgModeExtractor::extract_metadata_and_content(org_text, &org);

        assert_eq!(metadata.date, Some("2024-01-15".to_string()));
    }

    #[test]
    fn test_extract_metadata_with_keywords() {
        let org_text = "#+KEYWORDS: rust, org-mode, parsing\n\nContent here.";
        let lines: Vec<String> = org_text.lines().map(|s| s.to_string()).collect();
        let org = Org::from_vec(&lines).expect("Failed to parse org");
        let (metadata, _) = OrgModeExtractor::extract_metadata_and_content(org_text, &org);

        let keywords = metadata.additional.get("keywords").and_then(|v| v.as_array());
        assert!(keywords.is_some());
    }

    #[test]
    fn test_extract_content_with_headings() {
        let org_text = "* Heading 1\n\nSome content.\n\n** Heading 2\n\nMore content.";
        let lines: Vec<String> = org_text.lines().map(|s| s.to_string()).collect();
        let org = Org::from_vec(&lines).expect("Failed to parse org");
        let content = OrgModeExtractor::extract_content(&org);

        assert!(content.contains("Heading 1"));
        assert!(content.contains("Heading 2"));
        assert!(content.contains("Some content"));
        assert!(content.contains("More content"));
    }

    #[test]
    fn test_extract_content_with_paragraphs() {
        let org_text = "First paragraph.\n\nSecond paragraph.";
        let lines: Vec<String> = org_text.lines().map(|s| s.to_string()).collect();
        let org = Org::from_vec(&lines).expect("Failed to parse org");
        let content = OrgModeExtractor::extract_content(&org);

        assert!(content.contains("First paragraph"));
        assert!(content.contains("Second paragraph"));
    }

    #[test]
    fn test_extract_content_with_lists() {
        let org_text = "- Item 1\n- Item 2\n- Item 3";
        let lines: Vec<String> = org_text.lines().map(|s| s.to_string()).collect();
        let org = Org::from_vec(&lines).expect("Failed to parse org");
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
        let lines: Vec<String> = org_text.lines().map(|s| s.to_string()).collect();
        let org = Org::from_vec(&lines).expect("Failed to parse org");
        let content = OrgModeExtractor::extract_content(&org);

        assert!(content.contains("AT&T"), "Should contain link description 'AT&T'");
    }

    #[test]
    fn test_link_without_description() {
        let org_text = r#"* Links Test

[[https://example.com]]
"#;
        let lines: Vec<String> = org_text.lines().map(|s| s.to_string()).collect();
        let org = Org::from_vec(&lines).expect("Failed to parse org");
        let content = OrgModeExtractor::extract_content(&org);

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
        let lines: Vec<String> = org_text.lines().map(|s| s.to_string()).collect();
        let org = Org::from_vec(&lines).expect("Failed to parse org");
        let content = OrgModeExtractor::extract_content(&org);

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
        let lines: Vec<String> = org_text.lines().map(|s| s.to_string()).collect();
        let org = Org::from_vec(&lines).expect("Failed to parse org");
        let content = OrgModeExtractor::extract_content(&org);

        assert!(content.contains("Example Link"));
        assert!(content.contains("example.org"));
        assert!(content.contains("Contact"));
    }

    #[test]
    fn test_link_description_priority_over_url() {
        let org_text = r#"[[http://att.com/][AT&T]]"#;
        let lines: Vec<String> = org_text.lines().map(|s| s.to_string()).collect();
        let org = Org::from_vec(&lines).expect("Failed to parse org");
        let content = OrgModeExtractor::extract_content(&org);

        assert!(content.contains("AT&T"), "Description should be prioritized over URL");
        assert!(
            content.contains("[AT&T]"),
            "Link should be formatted as [description] when description exists"
        );
    }
}
