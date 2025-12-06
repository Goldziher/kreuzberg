//! HTML document extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::cells_to_markdown;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata, Table};
use async_trait::async_trait;
use scraper::{Html, Selector};
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

/// Extract all tables from HTML content.
///
/// Parses HTML to find `<table>` elements and extracts their structure
/// into `Table` objects with cells and markdown representation.
fn extract_html_tables(html: &str) -> Result<Vec<Table>> {
    let document = Html::parse_document(html);
    let table_selector = Selector::parse("table")
        .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to parse table selector: {}", e)))?;
    let row_selector = Selector::parse("tr")
        .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to parse row selector: {}", e)))?;
    let header_selector = Selector::parse("th")
        .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to parse header selector: {}", e)))?;
    let cell_selector = Selector::parse("td")
        .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to parse cell selector: {}", e)))?;

    let mut tables = Vec::new();

    for (table_index, table_elem) in document.select(&table_selector).enumerate() {
        let mut cells: Vec<Vec<String>> = Vec::new();

        for row in table_elem.select(&row_selector) {
            let mut row_cells = Vec::new();

            // Try headers first (th elements)
            let headers: Vec<_> = row.select(&header_selector).collect();
            if !headers.is_empty() {
                for header in headers {
                    let text = header
                        .text()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");
                    row_cells.push(text);
                }
            } else {
                // Use data cells (td elements)
                for cell in row.select(&cell_selector) {
                    let text = cell
                        .text()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");
                    row_cells.push(text);
                }
            }

            if !row_cells.is_empty() {
                cells.push(row_cells);
            }
        }

        // Only create a table if it has content
        if !cells.is_empty() {
            let markdown = cells_to_markdown(&cells);
            tables.push(Table {
                cells,
                markdown,
                page_number: table_index + 1, // 1-indexed
            });
        }
    }

    Ok(tables)
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
        let html = std::str::from_utf8(content)
            .map(|s| s.to_string())
            .unwrap_or_else(|_| String::from_utf8_lossy(content).to_string());

        // Extract tables from HTML
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
            tables,
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }

    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, path, config),
        fields(
            extractor.name = self.name(),
        )
    ))]
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

        // Check markdown format
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
        // Whitespace is normalized during text extraction
        assert_eq!(table.cells[0][0], "Header Bold");
        assert_eq!(table.cells[1][0], "Data with emphasis");
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
