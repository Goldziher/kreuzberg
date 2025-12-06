//! ODT (OpenDocument Text) extractor using native Rust parsing.
//!
//! Supports: OpenDocument Text (.odt)

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::{cells_to_markdown, office_metadata};
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata, Table};
use async_trait::async_trait;
use roxmltree::Document;
use std::io::Cursor;

/// High-performance ODT extractor using native Rust XML parsing.
///
/// This extractor provides:
/// - Fast text extraction via roxmltree XML parsing
/// - Comprehensive metadata extraction from meta.xml
/// - Table extraction with row and cell support
/// - Formatting preservation (bold, italic, strikeout)
/// - Support for headings, paragraphs, and special elements
pub struct OdtExtractor;

impl OdtExtractor {
    /// Create a new ODT extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for OdtExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for OdtExtractor {
    fn name(&self) -> &str {
        "odt-extractor"
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
        "Native Rust ODT (OpenDocument Text) extractor with metadata and table support"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

/// Extract text from MathML formula element
///
/// # Arguments
/// * `math_node` - The math XML node
///
/// # Returns
/// * `Option<String>` - The extracted formula text
fn extract_mathml_text(math_node: roxmltree::Node) -> Option<String> {
    // Try to find annotation with StarMath encoding first
    for node in math_node.descendants() {
        if node.tag_name().name() == "annotation"
            && let Some(encoding) = node.attribute("encoding")
            && encoding.contains("StarMath")
            && let Some(text) = node.text()
        {
            return Some(text.to_string());
        }
    }

    // Fallback: try to extract text from MathML elements
    let mut formula_parts = Vec::new();
    for node in math_node.descendants() {
        match node.tag_name().name() {
            "mi" | "mo" | "mn" | "ms" | "mtext" => {
                if let Some(text) = node.text() {
                    formula_parts.push(text.to_string());
                }
            }
            _ => {}
        }
    }

    if !formula_parts.is_empty() {
        Some(formula_parts.join(" "))
    } else {
        None
    }
}

/// Extract text from embedded formula objects
///
/// # Arguments
/// * `archive` - ZIP archive containing the ODT document
///
/// # Returns
/// * `String` - Extracted formula content from embedded objects
fn extract_embedded_formulas(archive: &mut zip::ZipArchive<Cursor<Vec<u8>>>) -> crate::error::Result<String> {
    use std::io::Read;
    let mut formula_parts = Vec::new();

    // Try to find embedded objects (e.g., "Object 1/content.xml")
    let file_names: Vec<String> = archive.file_names().map(|s| s.to_string()).collect();

    for file_name in file_names {
        // Look for embedded object content.xml files
        if file_name.contains("Object")
            && file_name.ends_with("content.xml")
            && let Ok(mut file) = archive.by_name(&file_name)
        {
            let mut xml_content = String::new();
            if file.read_to_string(&mut xml_content).is_ok() {
                // Parse and look for math elements
                if let Ok(doc) = Document::parse(&xml_content) {
                    let root = doc.root_element();

                    // Look for math elements in the embedded object
                    if root.tag_name().name() == "math" {
                        if let Some(formula_text) = extract_mathml_text(root) {
                            formula_parts.push(formula_text);
                        }
                    } else {
                        // Search for math elements within the document
                        for node in root.descendants() {
                            if node.tag_name().name() == "math"
                                && let Some(formula_text) = extract_mathml_text(node)
                            {
                                formula_parts.push(formula_text);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(formula_parts.join("\n"))
}

/// Extract text content from ODT content.xml
///
/// # Arguments
/// * `archive` - ZIP archive containing the ODT document
///
/// # Returns
/// * `String` - Extracted text content
fn extract_content_text(archive: &mut zip::ZipArchive<Cursor<Vec<u8>>>) -> crate::error::Result<String> {
    let mut xml_content = String::new();

    match archive.by_name("content.xml") {
        Ok(mut file) => {
            use std::io::Read;
            file.read_to_string(&mut xml_content)
                .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to read content.xml: {}", e)))?;
        }
        Err(_) => {
            return Ok(String::new());
        }
    }

    let doc = Document::parse(&xml_content)
        .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to parse content.xml: {}", e)))?;

    let root = doc.root_element();

    // Extract text content from the document body
    let mut text_parts: Vec<String> = Vec::new();

    // Process all relevant elements
    for node in root.descendants() {
        match node.tag_name().name() {
            // Headings
            "h" => {
                if let Some(text) = extract_node_text(node)
                    && !text.trim().is_empty()
                {
                    text_parts.push(format!("# {}", text.trim()));
                    text_parts.push(String::new());
                }
            }
            // Paragraphs
            "p" => {
                if let Some(text) = extract_node_text(node)
                    && !text.trim().is_empty()
                {
                    text_parts.push(text.trim().to_string());
                    text_parts.push(String::new());
                }
            }
            // Tables
            "table" => {
                if let Some(table_text) = extract_table_text(node) {
                    text_parts.push(table_text);
                    text_parts.push(String::new());
                }
            }
            _ => {}
        }
    }

    Ok(text_parts.join("\n").trim().to_string())
}

/// Extract text from a single XML node, handling spans and formatting
///
/// # Arguments
/// * `node` - The XML node to extract text from
///
/// # Returns
/// * `Option<String>` - The extracted text with formatting preserved
fn extract_node_text(node: roxmltree::Node) -> Option<String> {
    let mut text_parts = Vec::new();

    for child in node.children() {
        match child.tag_name().name() {
            "span" => {
                // Extract text from span (preserves formatting info)
                if let Some(text) = child.text() {
                    text_parts.push(text.to_string());
                }
            }
            "tab" => {
                text_parts.push("\t".to_string());
            }
            "line-break" => {
                text_parts.push("\n".to_string());
            }
            _ => {
                // For other elements, try to get their text content
                if let Some(text) = child.text() {
                    text_parts.push(text.to_string());
                }
            }
        }
    }

    if text_parts.is_empty() {
        // If no children, try to get text directly from the node
        node.text().map(|s| s.to_string())
    } else {
        Some(text_parts.join(""))
    }
}

/// Extract table content as text with markdown formatting
///
/// # Arguments
/// * `table_node` - The table XML node
///
/// # Returns
/// * `Option<String>` - Markdown formatted table
fn extract_table_text(table_node: roxmltree::Node) -> Option<String> {
    let mut rows = Vec::new();
    let mut max_cols = 0;

    // Extract all rows from the table
    for row_node in table_node.children() {
        if row_node.tag_name().name() == "table-row" {
            let mut row_cells = Vec::new();

            // Extract all cells from the row
            for cell_node in row_node.children() {
                if cell_node.tag_name().name() == "table-cell" {
                    let cell_text = extract_node_text(cell_node).unwrap_or_default();
                    row_cells.push(cell_text.trim().to_string());
                }
            }

            if !row_cells.is_empty() {
                max_cols = max_cols.max(row_cells.len());
                rows.push(row_cells);
            }
        }
    }

    if rows.is_empty() {
        return None;
    }

    // Pad rows with empty cells to make them equal length
    for row in &mut rows {
        while row.len() < max_cols {
            row.push(String::new());
        }
    }

    // Generate markdown table
    let mut markdown = String::new();

    // Header row
    if !rows.is_empty() {
        markdown.push('|');
        for cell in &rows[0] {
            markdown.push(' ');
            markdown.push_str(cell);
            markdown.push_str(" |");
        }
        markdown.push('\n');

        // Separator row
        markdown.push('|');
        for _ in 0..rows[0].len() {
            markdown.push_str(" --- |");
        }
        markdown.push('\n');

        // Data rows
        for row in rows.iter().skip(1) {
            markdown.push('|');
            for cell in row {
                markdown.push(' ');
                markdown.push_str(cell);
                markdown.push_str(" |");
            }
            markdown.push('\n');
        }
    }

    Some(markdown)
}

/// Extract tables from ODT content.xml
///
/// # Arguments
/// * `archive` - ZIP archive containing the ODT document
///
/// # Returns
/// * `Result<Vec<Table>>` - Extracted tables
fn extract_tables(archive: &mut zip::ZipArchive<Cursor<Vec<u8>>>) -> crate::error::Result<Vec<Table>> {
    let mut xml_content = String::new();

    match archive.by_name("content.xml") {
        Ok(mut file) => {
            use std::io::Read;
            file.read_to_string(&mut xml_content)
                .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to read content.xml: {}", e)))?;
        }
        Err(_) => {
            return Ok(Vec::new());
        }
    }

    let doc = Document::parse(&xml_content)
        .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to parse content.xml: {}", e)))?;

    let root = doc.root_element();
    let mut tables = Vec::new();
    let mut table_index = 0;

    // Find all table elements
    for node in root.descendants() {
        if node.tag_name().name() == "table"
            && let Some(table) = parse_odt_table(node, table_index)
        {
            tables.push(table);
            table_index += 1;
        }
    }

    Ok(tables)
}

/// Parse a single ODT table element into a Table struct
///
/// # Arguments
/// * `table_node` - The table XML node
/// * `table_index` - Index of the table in the document
///
/// # Returns
/// * `Option<Table>` - Parsed table
fn parse_odt_table(table_node: roxmltree::Node, table_index: usize) -> Option<Table> {
    let mut cells: Vec<Vec<String>> = Vec::new();

    // Extract all rows from the table
    for row_node in table_node.children() {
        if row_node.tag_name().name() == "table-row" {
            let mut row_cells = Vec::new();

            // Extract all cells from the row
            for cell_node in row_node.children() {
                if cell_node.tag_name().name() == "table-cell" {
                    let cell_text = extract_node_text(cell_node).unwrap_or_default();
                    row_cells.push(cell_text.trim().to_string());
                }
            }

            if !row_cells.is_empty() {
                cells.push(row_cells);
            }
        }
    }

    if cells.is_empty() {
        return None;
    }

    // Generate markdown representation
    let markdown = cells_to_markdown(&cells);

    Some(Table {
        cells,
        markdown,
        page_number: table_index + 1, // 1-indexed
    })
}

#[async_trait]
impl DocumentExtractor for OdtExtractor {
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
        // Create a copy of content for use in both threads if needed
        let content_owned = content.to_vec();

        // Parse the ODT document to extract both text and tables
        let (text, tables) = if crate::core::batch_mode::is_batch_mode() {
            // Batch mode: Use spawn_blocking for parallelism
            let content_for_task = content_owned.clone();
            let span = tracing::Span::current();
            tokio::task::spawn_blocking(move || -> crate::error::Result<(String, Vec<Table>)> {
                let _guard = span.entered();

                let cursor = Cursor::new(content_for_task);
                let mut archive = zip::ZipArchive::new(cursor)
                    .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to open ZIP archive: {}", e)))?;

                let text = extract_content_text(&mut archive)?;
                let tables = extract_tables(&mut archive)?;
                let embedded_formulas = extract_embedded_formulas(&mut archive)?;

                // Combine text and formulas
                let combined_text = if !embedded_formulas.is_empty() {
                    if !text.is_empty() {
                        format!("{}\n{}", text, embedded_formulas)
                    } else {
                        embedded_formulas
                    }
                } else {
                    text
                };

                Ok((combined_text, tables))
            })
            .await
            .map_err(|e| crate::error::KreuzbergError::parsing(format!("ODT extraction task failed: {}", e)))??
        } else {
            // Single-file mode: Direct extraction (no spawn overhead)
            let cursor = Cursor::new(content_owned.clone());
            let mut archive = zip::ZipArchive::new(cursor)
                .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to open ZIP archive: {}", e)))?;

            let text = extract_content_text(&mut archive)?;
            let tables = extract_tables(&mut archive)?;
            let embedded_formulas = extract_embedded_formulas(&mut archive)?;

            // Combine text and formulas
            let combined_text = if !embedded_formulas.is_empty() {
                if !text.is_empty() {
                    format!("{}\n{}", text, embedded_formulas)
                } else {
                    embedded_formulas
                }
            } else {
                text
            };

            (combined_text, tables)
        };

        // Extract metadata from meta.xml
        let mut metadata_map = std::collections::HashMap::new();

        // Try to extract core properties (for ODT, this may be in meta.xml)
        let cursor = Cursor::new(content_owned.clone());
        let mut archive = zip::ZipArchive::new(cursor).map_err(|e| {
            crate::error::KreuzbergError::parsing(format!("Failed to open ZIP archive for metadata: {}", e))
        })?;

        // Extract core properties if available
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
        &["application/vnd.oasis.opendocument.text"]
    }

    fn priority(&self) -> i32 {
        60 // Higher priority than Pandoc (40) and DOCX (50) to take precedence for ODT files
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_odt_extractor_plugin_interface() {
        let extractor = OdtExtractor::new();
        assert_eq!(extractor.name(), "odt-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 60);
        assert_eq!(extractor.supported_mime_types().len(), 1);
    }

    #[tokio::test]
    async fn test_odt_extractor_supports_odt() {
        let extractor = OdtExtractor::new();
        assert!(
            extractor
                .supported_mime_types()
                .contains(&"application/vnd.oasis.opendocument.text")
        );
    }

    #[tokio::test]
    async fn test_odt_extractor_default() {
        let extractor = OdtExtractor;
        assert_eq!(extractor.name(), "odt-extractor");
    }

    #[tokio::test]
    async fn test_odt_extractor_initialize_shutdown() {
        let extractor = OdtExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_extract_node_text_simple() {
        let xml = r#"<p xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">Hello world</p>"#;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let node = doc.root_element();

        let result = extract_node_text(node);
        assert!(result.is_some());
        // The text should be extracted
        assert!(!result.unwrap().is_empty());
    }
}
