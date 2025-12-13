//! HTML document extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extractors::SyncExtractor;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata, Table};
use async_trait::async_trait;
#[cfg(feature = "tokio-runtime")]
use std::path::Path;

// NOTE: scraper dependency has been removed in favor of html-to-markdown-rs

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

/// Extract all tables from HTML content using html-to-markdown-rs.
///
/// Uses html-to-markdown-rs to convert HTML to Markdown, which preserves
/// table structure in markdown format. Tables are then parsed from the
/// resulting markdown to maintain compatibility with existing Table API.
///
/// This approach eliminates the need for the `scraper` dependency as
/// html-to-markdown-rs already handles all table parsing.
fn extract_html_tables(html: &str) -> Result<Vec<Table>> {
    let markdown = crate::extraction::html::convert_html_to_markdown(html, None)?;

    let tables = parse_markdown_tables(&markdown);

    Ok(tables)
}

/// Parse markdown tables from HTML-converted markdown.
///
/// Extracts table data from markdown pipe-delimited format.
/// This maintains the existing Table structure API.
fn parse_markdown_tables(markdown: &str) -> Vec<Table> {
    let mut tables = Vec::new();
    let mut table_index = 0;
    let lines: Vec<&str> = markdown.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        if lines[i].trim_start().starts_with('|')
            && let Some((cells, end_idx)) = extract_markdown_table(&lines, i)
            && !cells.is_empty()
        {
            let markdown_table = reconstruct_markdown_table(&cells);
            tables.push(Table {
                cells,
                markdown: markdown_table,
                page_number: table_index + 1,
            });
            table_index += 1;
            i = end_idx;
            continue;
        }
        i += 1;
    }

    tables
}

/// Extract a single markdown table from lines.
///
/// Returns the parsed table cells and the index after the table ends.
fn extract_markdown_table(lines: &[&str], start_idx: usize) -> Option<(Vec<Vec<String>>, usize)> {
    let header_line = lines.get(start_idx)?;

    if !header_line.trim_start().starts_with('|') {
        return None;
    }

    let mut cells = Vec::new();
    let mut i = start_idx;

    if let Some(header_cells) = parse_markdown_table_row(header_line) {
        cells.push(header_cells);
        i += 1;
    } else {
        return None;
    }

    if i < lines.len() {
        let sep_line = lines[i];
        if is_markdown_table_separator(sep_line) {
            i += 1;
        }
    }

    while i < lines.len() {
        let line = lines[i];
        if let Some(row_cells) = parse_markdown_table_row(line) {
            cells.push(row_cells);
            i += 1;
        } else if !line.trim_start().starts_with('|') {
            break;
        } else {
            i += 1;
        }
    }

    if cells.len() > 1 { Some((cells, i)) } else { None }
}

/// Parse a single markdown table row into cell contents.
fn parse_markdown_table_row(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim_start();

    if !trimmed.starts_with('|') || !trimmed.contains('|') {
        return None;
    }

    let cells: Vec<String> = trimmed
        .split('|')
        .skip(1)
        .map(|cell| cell.trim().to_string())
        .filter(|cell| !cell.is_empty())
        .collect();

    if cells.is_empty() { None } else { Some(cells) }
}

/// Check if a line is a markdown table separator.
fn is_markdown_table_separator(line: &str) -> bool {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('|') {
        return false;
    }

    trimmed
        .split('|')
        .all(|cell| cell.trim().chars().all(|c| c == '-' || c == ':' || c.is_whitespace()))
}

/// Reconstruct markdown table from cells.
///
/// Takes parsed table cells and creates a properly formatted markdown table string.
fn reconstruct_markdown_table(cells: &[Vec<String>]) -> String {
    if cells.is_empty() {
        return String::new();
    }

    let mut markdown = String::new();

    for (row_idx, row) in cells.iter().enumerate() {
        markdown.push('|');
        for cell in row {
            markdown.push(' ');
            markdown.push_str(cell);
            markdown.push(' ');
            markdown.push('|');
        }
        markdown.push('\n');

        if row_idx == 0 {
            markdown.push('|');
            for _ in row {
                markdown.push_str("------|");
            }
            markdown.push('\n');
        }
    }

    markdown
}

impl Plugin for HtmlExtractor {
    fn name(&self) -> &str {
        "html-extractor"
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
}

impl SyncExtractor for HtmlExtractor {
    fn extract_sync(&self, content: &[u8], mime_type: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
        let html = std::str::from_utf8(content)
            .map(|s| s.to_string())
            .unwrap_or_else(|_| String::from_utf8_lossy(content).to_string());

        let tables = extract_html_tables(&html)?;

        let markdown = crate::extraction::html::convert_html_to_markdown(&html, config.html_options.clone())?;

        let (html_metadata, content_without_frontmatter) = crate::extraction::html::parse_html_metadata(&markdown)?;

        Ok(ExtractionResult {
            content: content_without_frontmatter,
            mime_type: mime_type.to_string(),
            metadata: Metadata {
                format: html_metadata.map(|m| crate::types::FormatMetadata::Html(Box::new(m))),
                ..Default::default()
            },
            pages: None,
            tables,
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }
}

#[async_trait]
impl DocumentExtractor for HtmlExtractor {
    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, content, config),
        fields(
            extractor.name = self.name(),
            content.size_bytes = content.len(),
        )
    ))]
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        self.extract_sync(content, mime_type, config)
    }

    #[cfg(feature = "tokio-runtime")]
    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, path, config),
        fields(
            extractor.name = self.name(),
        )
    ))]
    #[cfg(feature = "tokio-runtime")]
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

    fn as_sync_extractor(&self) -> Option<&dyn crate::extractors::SyncExtractor> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_extractor_plugin_interface() {
        let extractor = HtmlExtractor::new();
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

    #[test]
    fn test_extract_html_tables_basic() {
        let html = r#"
            <table>
                <tr><th>Header1</th><th>Header2</th></tr>
                <tr><td>Row1Col1</td><td>Row1Col2</td></tr>
                <tr><td>Row2Col1</td><td>Row2Col2</td></tr>
            </table>
        "#;

        let tables = extract_html_tables(html).unwrap();
        assert_eq!(tables.len(), 1);

        let table = &tables[0];
        assert_eq!(table.cells.len(), 3);
        assert_eq!(table.cells[0], vec!["Header1", "Header2"]);
        assert_eq!(table.cells[1], vec!["Row1Col1", "Row1Col2"]);
        assert_eq!(table.cells[2], vec!["Row2Col1", "Row2Col2"]);
        assert_eq!(table.page_number, 1);

        assert!(table.markdown.contains("| Header1 | Header2 |"));
        assert!(table.markdown.contains("|------|------|"));
        assert!(table.markdown.contains("| Row1Col1 | Row1Col2 |"));
    }

    #[test]
    fn test_extract_html_tables_multiple() {
        let html = r#"
            <table>
                <tr><th>Table1</th></tr>
                <tr><td>Data1</td></tr>
            </table>
            <p>Some text</p>
            <table>
                <tr><th>Table2</th></tr>
                <tr><td>Data2</td></tr>
            </table>
        "#;

        let tables = extract_html_tables(html).unwrap();
        assert_eq!(tables.len(), 2);
        assert_eq!(tables[0].page_number, 1);
        assert_eq!(tables[1].page_number, 2);
    }

    #[test]
    fn test_extract_html_tables_no_thead() {
        let html = r#"
            <table>
                <tr><td>Cell1</td><td>Cell2</td></tr>
                <tr><td>Cell3</td><td>Cell4</td></tr>
            </table>
        "#;

        let tables = extract_html_tables(html).unwrap();
        assert_eq!(tables.len(), 1);

        let table = &tables[0];
        assert_eq!(table.cells.len(), 2);
        assert_eq!(table.cells[0], vec!["Cell1", "Cell2"]);
        assert_eq!(table.cells[1], vec!["Cell3", "Cell4"]);
    }

    #[test]
    fn test_extract_html_tables_empty() {
        let html = "<p>No tables here</p>";
        let tables = extract_html_tables(html).unwrap();
        assert_eq!(tables.len(), 0);
    }

    #[test]
    fn test_extract_html_tables_with_nested_elements() {
        let html = r#"
            <table>
                <tr><th>Header <strong>Bold</strong></th></tr>
                <tr><td>Data with <em>emphasis</em></td></tr>
            </table>
        "#;

        let tables = extract_html_tables(html).unwrap();
        assert_eq!(tables.len(), 1);

        let table = &tables[0];
        assert_eq!(table.cells[0][0], "Header **Bold**");
        assert_eq!(table.cells[1][0], "Data with *emphasis*");
    }

    #[tokio::test]
    async fn test_html_extractor_with_table() {
        let html = r#"
            <html>
                <body>
                    <h1>Test Page</h1>
                    <table>
                        <tr><th>Name</th><th>Age</th></tr>
                        <tr><td>Alice</td><td>30</td></tr>
                        <tr><td>Bob</td><td>25</td></tr>
                    </table>
                </body>
            </html>
        "#;

        let extractor = HtmlExtractor::new();
        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(html.as_bytes(), "text/html", &config)
            .await
            .unwrap();

        assert_eq!(result.tables.len(), 1);
        let table = &result.tables[0];
        assert_eq!(table.cells.len(), 3);
        assert_eq!(table.cells[0], vec!["Name", "Age"]);
        assert_eq!(table.cells[1], vec!["Alice", "30"]);
        assert_eq!(table.cells[2], vec!["Bob", "25"]);
    }
}
