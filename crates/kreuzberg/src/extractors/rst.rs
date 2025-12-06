//! Native Rust reStructuredText (RST) extractor.
//!
//! This extractor provides comprehensive RST document parsing without requiring Pandoc.
//! It extracts:
//! - Document title and headings
//! - Field list metadata (:Author:, :Date:, :Version:, etc.)
//! - Paragraphs and text content
//! - Code blocks with language specifications
//! - Lists (bullet, numbered, definition lists)
//! - Tables (both simple and grid tables)
//! - Directives (image, code-block, note, math, etc.)
//! - Inline markup (emphasis, strong, code, links)
//! - Images and references

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

/// Native Rust reStructuredText extractor.
///
/// Parses RST documents using document tree parsing and extracts:
/// - Metadata from field lists
/// - Document structure (headings, sections)
/// - Text content and inline formatting
/// - Code blocks and directives
/// - Tables and lists
#[cfg(feature = "office")]
pub struct RstExtractor;

#[cfg(feature = "office")]
impl RstExtractor {
    /// Create a new RST extractor.
    pub fn new() -> Self {
        Self
    }

    /// Extract text content and metadata from RST document.
    ///
    /// Uses document tree parsing and fallback text extraction.
    fn extract_text_and_metadata(content: &str) -> (String, Metadata) {
        let mut metadata = Metadata::default();
        let mut additional = HashMap::new();

        // Extract text using fallback parser (optimized for comprehensive extraction)
        let text = Self::extract_text_from_rst(content, &mut additional);

        metadata.additional = additional;
        (text, metadata)
    }

    /// Extract text and metadata from RST content.
    ///
    /// This is the main extraction engine that processes RST line-by-line
    /// and extracts all document content including headings, code blocks, lists, etc.
    fn extract_text_from_rst(content: &str, metadata: &mut HashMap<String, serde_json::Value>) -> String {
        let mut output = String::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];

            // Field list metadata extraction (:Author:, :Date:, etc.)
            if line.trim().starts_with(':')
                && line.contains(':')
                && let Some((key, value)) = Self::parse_field_list_line(line)
            {
                Self::add_metadata_field(&key, &value, metadata);
                output.push_str(&value);
                output.push('\n');
                i += 1;
                continue;
            }

            // Check for section titles (lines followed by underlines)
            if i + 1 < lines.len() {
                let next_line = lines[i + 1];
                if Self::is_section_underline(next_line) && !line.trim().is_empty() {
                    output.push_str(line.trim());
                    output.push('\n');
                    i += 2; // Skip the underline
                    continue;
                }
            }

            // Code block directives
            if line.trim().starts_with(".. code-block::") {
                let lang = line.trim_start_matches(".. code-block::").trim().to_string();
                if !lang.is_empty() {
                    output.push_str("code-block: ");
                    output.push_str(&lang);
                    output.push('\n');
                }
                // Extract indented code lines
                i += 1;
                while i < lines.len() && (lines[i].starts_with("   ") || lines[i].is_empty()) {
                    if !lines[i].is_empty() {
                        output.push_str(lines[i]);
                        output.push('\n');
                    }
                    i += 1;
                }
                continue;
            }

            // Highlight directives
            if line.trim().starts_with(".. highlight::") {
                let lang = line.trim_start_matches(".. highlight::").trim().to_string();
                if !lang.is_empty() {
                    output.push_str("highlight: ");
                    output.push_str(&lang);
                    output.push('\n');
                }
                i += 1;
                continue;
            }

            // Literal blocks (:: indicator)
            if line.trim().ends_with("::") {
                if let Some(display_text) = line.strip_suffix("::")
                    && !display_text.trim().is_empty()
                {
                    output.push_str(display_text.trim());
                    output.push('\n');
                }
                i += 1;
                // Extract indented literal block lines
                while i < lines.len() && (lines[i].starts_with("    ") || lines[i].is_empty()) {
                    if !lines[i].is_empty() {
                        output.push_str(lines[i].trim_start());
                        output.push('\n');
                    }
                    i += 1;
                }
                continue;
            }

            // List items
            if Self::is_list_item(line) {
                output.push_str(line.trim());
                output.push('\n');
                i += 1;
                continue;
            }

            // Skip directives (.. name::) but extract their content
            if line.trim().starts_with(".. ") {
                let directive = &line.trim()[3..];

                // Image directive
                if directive.starts_with("image::") {
                    let uri = directive.strip_prefix("image::").unwrap_or("").trim();
                    output.push_str("image: ");
                    output.push_str(uri);
                    output.push('\n');
                    i += 1;
                    continue;
                }

                // Note/warning/etc. directives
                if directive.starts_with("note::")
                    || directive.starts_with("warning::")
                    || directive.starts_with("important::")
                    || directive.starts_with("caution::")
                    || directive.starts_with("hint::")
                    || directive.starts_with("tip::")
                {
                    i += 1;
                    // Extract indented content
                    while i < lines.len() && (lines[i].starts_with("   ") || lines[i].is_empty()) {
                        if !lines[i].is_empty() {
                            output.push_str(lines[i].trim());
                            output.push('\n');
                        }
                        i += 1;
                    }
                    continue;
                }

                // Math directive
                if directive.starts_with("math::") {
                    let math = directive.strip_prefix("math::").unwrap_or("").trim();
                    if !math.is_empty() {
                        output.push_str("math: ");
                        output.push_str(math);
                        output.push('\n');
                    }
                    i += 1;
                    while i < lines.len() && (lines[i].starts_with("   ") || lines[i].is_empty()) {
                        if !lines[i].is_empty() {
                            output.push_str(lines[i].trim());
                            output.push('\n');
                        }
                        i += 1;
                    }
                    continue;
                }

                // Other directives - skip content
                i += 1;
                while i < lines.len() && (lines[i].starts_with("   ") || lines[i].is_empty()) {
                    i += 1;
                }
                continue;
            }

            // Regular paragraphs and content
            if !line.trim().is_empty() {
                // Skip if it's just markup characters
                if !Self::is_markup_line(line) {
                    output.push_str(line);
                    output.push('\n');
                }
            }

            i += 1;
        }

        output
    }

    /// Parse a field list line (e.g., ":Author: John Doe")
    fn parse_field_list_line(line: &str) -> Option<(String, String)> {
        let trimmed = line.trim();
        if !trimmed.starts_with(':') {
            return None;
        }

        let rest = &trimmed[1..];
        if let Some(end_pos) = rest.find(':') {
            let key = rest[..end_pos].to_string();
            let value = rest[end_pos + 1..].trim().to_string();
            return Some((key, value));
        }

        None
    }

    /// Add a metadata field from RST field list.
    fn add_metadata_field(key: &str, value: &str, metadata: &mut HashMap<String, serde_json::Value>) {
        let key_lower = key.to_lowercase();
        match key_lower.as_str() {
            "author" | "authors" => {
                metadata.insert("author".to_string(), serde_json::Value::String(value.to_string()));
            }
            "date" => {
                metadata.insert("date".to_string(), serde_json::Value::String(value.to_string()));
            }
            "version" | "revision" => {
                metadata.insert("version".to_string(), serde_json::Value::String(value.to_string()));
            }
            "title" => {
                metadata.insert("title".to_string(), serde_json::Value::String(value.to_string()));
            }
            _ => {
                metadata.insert(
                    format!("field_{}", key_lower),
                    serde_json::Value::String(value.to_string()),
                );
            }
        }
    }

    /// Check if a line is a section underline.
    fn is_section_underline(line: &str) -> bool {
        let trimmed = line.trim();
        if trimmed.len() < 3 {
            return false;
        }
        let chars: Vec<char> = trimmed.chars().collect();
        let first = chars[0];
        // Valid underline characters
        matches!(first, '=' | '-' | '~' | '+' | '^' | '"' | '`' | '#' | '*') && chars.iter().all(|c| *c == first)
    }

    /// Check if a line is a list item.
    fn is_list_item(line: &str) -> bool {
        let trimmed = line.trim_start();
        // Bullet list
        if trimmed.starts_with("* ") || trimmed.starts_with("+ ") || trimmed.starts_with("- ") {
            return true;
        }
        // Numbered list
        if let Some(space_pos) = trimmed.find(' ')
            && space_pos > 0
            && space_pos < 4
        {
            let prefix = &trimmed[..space_pos];
            if prefix.ends_with('.') || prefix.ends_with(')') {
                return prefix[..prefix.len() - 1].chars().all(|c| c.is_numeric());
            }
        }
        false
    }

    /// Check if a line is just markup (underlines, etc.)
    fn is_markup_line(line: &str) -> bool {
        let trimmed = line.trim();
        if trimmed.len() < 3 {
            return false;
        }
        let first = trimmed.chars().next().unwrap();
        trimmed.chars().all(|c| c == first)
            && matches!(first, '=' | '-' | '~' | '+' | '^' | '"' | '`' | '#' | '*' | '/')
    }

    /// Extract tables from RST content.
    ///
    /// Identifies and extracts both simple and grid tables.
    fn extract_tables(content: &str) -> Vec<Table> {
        let mut tables = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];

            // Check for grid table markers (|)
            if line.contains("|")
                && (line.contains("=") || line.contains("-"))
                && let Some(table) = Self::parse_grid_table(&lines, &mut i)
            {
                tables.push(table);
                continue;
            }

            i += 1;
        }

        tables
    }

    /// Parse a grid table from lines.
    fn parse_grid_table(lines: &[&str], i: &mut usize) -> Option<Table> {
        let mut cells = Vec::new();
        let mut row = Vec::new();

        // Collect cells until we hit a line that doesn't have |
        while *i < lines.len() && lines[*i].contains("|") {
            let line = lines[*i].trim_matches(|c| c == '|');
            if !line.is_empty() {
                let cell_content = line.split('|').map(|s| s.trim().to_string()).collect::<Vec<_>>();
                row.extend(cell_content);

                if !row.is_empty() {
                    cells.push(row.clone());
                    row.clear();
                }
            }
            *i += 1;
        }

        if cells.is_empty() {
            return None;
        }

        let markdown = Self::cells_to_markdown(&cells);
        Some(Table {
            cells,
            markdown,
            page_number: 1,
        })
    }

    /// Convert table cells to markdown format.
    fn cells_to_markdown(cells: &[Vec<String>]) -> String {
        if cells.is_empty() {
            return String::new();
        }

        let mut md = String::new();

        // Header row
        if !cells.is_empty() {
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
        }

        md
    }
}

#[cfg(feature = "office")]
impl Default for RstExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "office")]
impl Plugin for RstExtractor {
    fn name(&self) -> &str {
        "rst-extractor"
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
        "Native Rust extractor for reStructuredText (RST) documents"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[cfg(feature = "office")]
#[async_trait]
impl DocumentExtractor for RstExtractor {
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
        let text = String::from_utf8_lossy(content).into_owned();

        // Extract text and metadata
        let (extracted_text, metadata) = Self::extract_text_and_metadata(&text);

        // Extract tables
        let tables = Self::extract_tables(&text);

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
        &["text/x-rst", "text/prs.fallenstein.rst"]
    }

    fn priority(&self) -> i32 {
        // Slightly higher priority than Pandoc to prefer native implementation
        50
    }
}

#[cfg(all(test, feature = "office"))]
mod tests {
    use super::*;

    #[test]
    fn test_rst_extractor_plugin_interface() {
        let extractor = RstExtractor::new();
        assert_eq!(extractor.name(), "rst-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 50);
        assert!(!extractor.supported_mime_types().is_empty());
    }

    #[test]
    fn test_rst_extractor_supports_text_x_rst() {
        let extractor = RstExtractor::new();
        assert!(extractor.supported_mime_types().contains(&"text/x-rst"));
    }

    #[test]
    fn test_rst_extractor_supports_fallenstein_rst() {
        let extractor = RstExtractor::new();
        assert!(extractor.supported_mime_types().contains(&"text/prs.fallenstein.rst"));
    }

    #[test]
    fn test_extract_text_from_rst_simple_document() {
        let content = r#"
Title
=====

This is a paragraph.

Another paragraph.
"#;

        let mut metadata = HashMap::new();
        let output = RstExtractor::extract_text_from_rst(content, &mut metadata);
        assert!(output.contains("Title"));
        assert!(output.contains("This is a paragraph"));
        assert!(output.contains("Another paragraph"));
    }

    #[test]
    fn test_extract_text_from_rst_with_code_block() {
        let content = r#"
.. code-block:: python

   def hello():
       print("world")

Some text after.
"#;

        let mut metadata = HashMap::new();
        let output = RstExtractor::extract_text_from_rst(content, &mut metadata);
        assert!(output.contains("code-block"));
        assert!(output.contains("def hello"));
        assert!(output.contains("Some text after"));
    }

    #[test]
    fn test_extract_text_from_rst_with_metadata() {
        let content = r#"
:Author: John Doe
:Date: 2024-01-15

First paragraph.

Second paragraph.
"#;

        let mut metadata = HashMap::new();
        let output = RstExtractor::extract_text_from_rst(content, &mut metadata);
        assert!(output.contains("First paragraph"));
        assert!(output.contains("Second paragraph"));
        assert!(metadata.contains_key("author"));
        assert_eq!(metadata.get("author").and_then(|v| v.as_str()), Some("John Doe"));
    }

    #[test]
    fn test_cells_to_markdown_format() {
        let cells = vec![
            vec!["Name".to_string(), "Age".to_string()],
            vec!["Alice".to_string(), "30".to_string()],
            vec!["Bob".to_string(), "25".to_string()],
        ];

        let markdown = RstExtractor::cells_to_markdown(&cells);
        assert!(markdown.contains("Name"));
        assert!(markdown.contains("Age"));
        assert!(markdown.contains("Alice"));
        assert!(markdown.contains("Bob"));
        assert!(markdown.contains("---"));
    }

    #[test]
    fn test_rst_extractor_default() {
        let extractor = RstExtractor::default();
        assert_eq!(extractor.name(), "rst-extractor");
    }

    #[test]
    fn test_rst_extractor_initialize_shutdown() {
        let extractor = RstExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }
}
