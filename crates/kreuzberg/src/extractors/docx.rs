//! DOCX extractor using docx-lite for high-performance text extraction.
//!
//! Supports: Microsoft Word (.docx)

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::office_metadata;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata, Table};
use async_trait::async_trait;
use std::io::Cursor;

/// High-performance DOCX extractor using docx-lite.
///
/// This extractor provides:
/// - Fast text extraction via streaming XML parsing (~160 MB/s average)
/// - Comprehensive metadata extraction (core.xml, app.xml, custom.xml)
/// - ~400x faster than Pandoc subprocess approach
pub struct DocxExtractor;

impl DocxExtractor {
    /// Create a new DOCX extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for DocxExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for DocxExtractor {
    fn name(&self) -> &str {
        "docx-extractor"
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
        "High-performance DOCX text extraction using docx-lite with metadata support"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

/// Convert docx-lite table to Kreuzberg Table struct with markdown representation.
///
/// # Arguments
/// * `docx_table` - The table from docx-lite library
/// * `table_index` - Index of the table in the document (used as page_number)
///
/// # Returns
/// * `Table` - Converted table with cells and markdown representation
fn convert_docx_table_to_table(docx_table: &docx_lite::Table, table_index: usize) -> Table {
    // Extract cells as 2D vector
    let cells: Vec<Vec<String>> = docx_table
        .rows
        .iter()
        .map(|row| {
            row.cells
                .iter()
                .map(|cell| {
                    // Extract text from all paragraphs in the cell
                    cell.paragraphs
                        .iter()
                        .map(|para| para.to_text())
                        .collect::<Vec<_>>()
                        .join(" ")
                        .trim()
                        .to_string()
                })
                .collect()
        })
        .collect();

    // Generate markdown representation
    let markdown = cells_to_markdown(&cells);

    Table {
        cells,
        markdown,
        page_number: table_index + 1, // 1-indexed
    }
}

/// Convert 2D cell data to markdown table format.
///
/// # Arguments
/// * `cells` - 2D vector of cell strings (rows × columns)
///
/// # Returns
/// * `String` - Markdown formatted table
fn cells_to_markdown(cells: &[Vec<String>]) -> String {
    if cells.is_empty() {
        return String::new();
    }

    let mut markdown = String::new();

    // Determine number of columns from first row
    let num_cols = cells.first().map(|r| r.len()).unwrap_or(0);
    if num_cols == 0 {
        return String::new();
    }

    // Header row (first row)
    if let Some(header) = cells.first() {
        markdown.push_str("| ");
        for cell in header {
            // Escape pipe characters in cell content
            let escaped = cell.replace('|', "\\|");
            markdown.push_str(&escaped);
            markdown.push_str(" | ");
        }
        markdown.push('\n');

        // Separator row
        markdown.push('|');
        for _ in 0..num_cols {
            markdown.push_str("------|");
        }
        markdown.push('\n');
    }

    // Data rows (skip first row as it's the header)
    for row in cells.iter().skip(1) {
        markdown.push_str("| ");
        for (idx, cell) in row.iter().enumerate() {
            if idx >= num_cols {
                break; // Handle irregular tables
            }
            // Escape pipe characters in cell content
            let escaped = cell.replace('|', "\\|");
            markdown.push_str(&escaped);
            markdown.push_str(" | ");
        }
        // Pad with empty cells if row is shorter than expected
        for _ in row.len()..num_cols {
            markdown.push_str(" | ");
        }
        markdown.push('\n');
    }

    markdown
}

#[async_trait]
impl DocumentExtractor for DocxExtractor {
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
        // Parse the DOCX document to extract both text and tables
        let (text, tables) = if crate::core::batch_mode::is_batch_mode() {
            // Batch mode: Use spawn_blocking for parallelism
            let content_owned = content.to_vec();
            tokio::task::spawn_blocking(move || -> crate::error::Result<(String, Vec<Table>)> {
                // Parse document structure
                let cursor = Cursor::new(&content_owned);
                let doc = docx_lite::parse_document(cursor)
                    .map_err(|e| crate::error::KreuzbergError::parsing(format!("DOCX parsing failed: {}", e)))?;

                // Extract text
                let text = doc.extract_text();

                // Extract tables
                let tables: Vec<Table> = doc
                    .tables
                    .iter()
                    .enumerate()
                    .map(|(idx, table)| convert_docx_table_to_table(table, idx))
                    .collect();

                Ok((text, tables))
            })
            .await
            .map_err(|e| crate::error::KreuzbergError::parsing(format!("DOCX extraction task failed: {}", e)))??
        } else {
            // Single-file mode: Direct extraction (no spawn overhead)
            let cursor = Cursor::new(content);
            let doc = docx_lite::parse_document(cursor)
                .map_err(|e| crate::error::KreuzbergError::parsing(format!("DOCX parsing failed: {}", e)))?;

            // Extract text
            let text = doc.extract_text();

            // Extract tables
            let tables: Vec<Table> = doc
                .tables
                .iter()
                .enumerate()
                .map(|(idx, table)| convert_docx_table_to_table(table, idx))
                .collect();

            (text, tables)
        };

        // Extract metadata using existing office_metadata module
        let mut archive = if crate::core::batch_mode::is_batch_mode() {
            // Batch mode: Use spawn_blocking for parallelism
            let content_owned = content.to_vec();
            tokio::task::spawn_blocking(move || -> crate::error::Result<_> {
                let cursor = Cursor::new(content_owned);
                zip::ZipArchive::new(cursor)
                    .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to open ZIP archive: {}", e)))
            })
            .await
            .map_err(|e| crate::error::KreuzbergError::parsing(format!("Task join error: {}", e)))??
        } else {
            // Single-file mode: Direct extraction (no spawn overhead)
            // Note: We still need to clone for ZipArchive type consistency with batch mode
            let content_owned = content.to_vec();
            let cursor = Cursor::new(content_owned);
            zip::ZipArchive::new(cursor)
                .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to open ZIP archive: {}", e)))?
        };

        let mut metadata_map = std::collections::HashMap::new();

        // Extract core properties (title, creator, dates, keywords, etc.)
        if let Ok(core) = office_metadata::extract_core_properties(&mut archive) {
            if let Some(title) = core.title {
                metadata_map.insert("title".to_string(), serde_json::Value::String(title));
            }
            if let Some(creator) = core.creator {
                metadata_map.insert(
                    "authors".to_string(),
                    serde_json::Value::Array(vec![serde_json::Value::String(creator.clone())]),
                );
                metadata_map.insert("created_by".to_string(), serde_json::Value::String(creator));
            }
            if let Some(subject) = core.subject {
                metadata_map.insert("subject".to_string(), serde_json::Value::String(subject));
            }
            if let Some(keywords) = core.keywords {
                metadata_map.insert("keywords".to_string(), serde_json::Value::String(keywords));
            }
            if let Some(description) = core.description {
                metadata_map.insert("description".to_string(), serde_json::Value::String(description));
            }
            if let Some(modified_by) = core.last_modified_by {
                metadata_map.insert("modified_by".to_string(), serde_json::Value::String(modified_by));
            }
            if let Some(created) = core.created {
                metadata_map.insert("created_at".to_string(), serde_json::Value::String(created));
            }
            if let Some(modified) = core.modified {
                metadata_map.insert("modified_at".to_string(), serde_json::Value::String(modified));
            }
            if let Some(revision) = core.revision {
                metadata_map.insert("revision".to_string(), serde_json::Value::String(revision));
            }
            if let Some(category) = core.category {
                metadata_map.insert("category".to_string(), serde_json::Value::String(category));
            }
            if let Some(content_status) = core.content_status {
                metadata_map.insert("content_status".to_string(), serde_json::Value::String(content_status));
            }
            if let Some(language) = core.language {
                metadata_map.insert("language".to_string(), serde_json::Value::String(language));
            }
        }

        // Extract app properties (page count, word count, etc.)
        if let Ok(app) = office_metadata::extract_docx_app_properties(&mut archive) {
            if let Some(pages) = app.pages {
                metadata_map.insert("page_count".to_string(), serde_json::Value::Number(pages.into()));
            }
            if let Some(words) = app.words {
                metadata_map.insert("word_count".to_string(), serde_json::Value::Number(words.into()));
            }
            if let Some(chars) = app.characters {
                metadata_map.insert("character_count".to_string(), serde_json::Value::Number(chars.into()));
            }
            if let Some(lines) = app.lines {
                metadata_map.insert("line_count".to_string(), serde_json::Value::Number(lines.into()));
            }
            if let Some(paragraphs) = app.paragraphs {
                metadata_map.insert(
                    "paragraph_count".to_string(),
                    serde_json::Value::Number(paragraphs.into()),
                );
            }
            if let Some(template) = app.template {
                metadata_map.insert("template".to_string(), serde_json::Value::String(template));
            }
            if let Some(company) = app.company {
                metadata_map.insert("organization".to_string(), serde_json::Value::String(company));
            }
            if let Some(time) = app.total_time {
                metadata_map.insert(
                    "total_editing_time_minutes".to_string(),
                    serde_json::Value::Number(time.into()),
                );
            }
            if let Some(application) = app.application {
                metadata_map.insert("application".to_string(), serde_json::Value::String(application));
            }
        }

        // Extract custom properties
        if let Ok(custom) = office_metadata::extract_custom_properties(&mut archive) {
            for (key, value) in custom {
                metadata_map.insert(format!("custom_{}", key), value);
            }
        }

        Ok(ExtractionResult {
            content: text,
            mime_type: mime_type.to_string(),
            metadata: Metadata {
                additional: metadata_map,
                ..Default::default()
            },
            tables,
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/vnd.openxmlformats-officedocument.wordprocessingml.document"]
    }

    fn priority(&self) -> i32 {
        50 // Higher priority than Pandoc (40) to take precedence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_docx_extractor_plugin_interface() {
        let extractor = DocxExtractor::new();
        assert_eq!(extractor.name(), "docx-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 50);
        assert_eq!(extractor.supported_mime_types().len(), 1);
    }

    #[tokio::test]
    async fn test_docx_extractor_supports_docx() {
        let extractor = DocxExtractor::new();
        assert!(
            extractor
                .supported_mime_types()
                .contains(&"application/vnd.openxmlformats-officedocument.wordprocessingml.document")
        );
    }

    #[tokio::test]
    async fn test_docx_extractor_default() {
        let extractor = DocxExtractor;
        assert_eq!(extractor.name(), "docx-extractor");
    }

    #[tokio::test]
    async fn test_docx_extractor_initialize_shutdown() {
        let extractor = DocxExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_cells_to_markdown_basic_table() {
        let cells = vec![
            vec!["Header1".to_string(), "Header2".to_string()],
            vec!["Row1Col1".to_string(), "Row1Col2".to_string()],
            vec!["Row2Col1".to_string(), "Row2Col2".to_string()],
        ];

        let markdown = cells_to_markdown(&cells);

        assert!(markdown.contains("| Header1 | Header2 |"));
        assert!(markdown.contains("|------|------|"));
        assert!(markdown.contains("| Row1Col1 | Row1Col2 |"));
        assert!(markdown.contains("| Row2Col1 | Row2Col2 |"));
    }

    #[test]
    fn test_cells_to_markdown_empty() {
        let cells: Vec<Vec<String>> = vec![];
        let markdown = cells_to_markdown(&cells);
        assert_eq!(markdown, "");
    }

    #[test]
    fn test_cells_to_markdown_escape_pipes() {
        let cells = vec![vec!["Header".to_string()], vec!["Cell with | pipe".to_string()]];

        let markdown = cells_to_markdown(&cells);
        assert!(markdown.contains("Cell with \\| pipe"));
    }

    #[test]
    fn test_cells_to_markdown_irregular_rows() {
        let cells = vec![
            vec!["H1".to_string(), "H2".to_string(), "H3".to_string()],
            vec!["R1C1".to_string(), "R1C2".to_string()], // Missing third column
            vec!["R2C1".to_string(), "R2C2".to_string(), "R2C3".to_string()],
        ];

        let markdown = cells_to_markdown(&cells);

        // Should have 3 columns in header
        assert!(markdown.contains("| H1 | H2 | H3 |"));
        // Should pad short rows
        assert!(markdown.contains("| R1C1 | R1C2 |  |"));
    }

    #[test]
    fn test_convert_docx_table_to_table() {
        use docx_lite::{Paragraph, Run, Table as DocxTable, TableCell, TableRow};

        // Create a simple docx-lite table
        let mut table = DocxTable::new();

        // Header row
        let mut header_row = TableRow::default();
        let mut cell1 = TableCell::default();
        let mut para1 = Paragraph::new();
        para1.add_run(Run::new("Name".to_string()));
        cell1.paragraphs.push(para1);
        header_row.cells.push(cell1);

        let mut cell2 = TableCell::default();
        let mut para2 = Paragraph::new();
        para2.add_run(Run::new("Age".to_string()));
        cell2.paragraphs.push(para2);
        header_row.cells.push(cell2);

        table.rows.push(header_row);

        // Data row
        let mut data_row = TableRow::default();
        let mut cell3 = TableCell::default();
        let mut para3 = Paragraph::new();
        para3.add_run(Run::new("Alice".to_string()));
        cell3.paragraphs.push(para3);
        data_row.cells.push(cell3);

        let mut cell4 = TableCell::default();
        let mut para4 = Paragraph::new();
        para4.add_run(Run::new("30".to_string()));
        cell4.paragraphs.push(para4);
        data_row.cells.push(cell4);

        table.rows.push(data_row);

        // Convert to Kreuzberg Table
        let result = convert_docx_table_to_table(&table, 0);

        assert_eq!(result.page_number, 1); // 0 + 1 = 1 (1-indexed)
        assert_eq!(result.cells.len(), 2); // 2 rows
        assert_eq!(result.cells[0], vec!["Name", "Age"]);
        assert_eq!(result.cells[1], vec!["Alice", "30"]);
        assert!(result.markdown.contains("| Name | Age |"));
        assert!(result.markdown.contains("| Alice | 30 |"));
    }
}
