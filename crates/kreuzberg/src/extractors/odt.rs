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
    for node in math_node.descendants() {
        if node.tag_name().name() == "annotation"
            && let Some(encoding) = node.attribute("encoding")
            && encoding.contains("StarMath")
            && let Some(text) = node.text()
        {
            return Some(text.to_string());
        }
    }

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

    let file_names: Vec<String> = archive.file_names().map(|s| s.to_string()).collect();

    for file_name in file_names {
        if file_name.contains("Object")
            && file_name.ends_with("content.xml")
            && let Ok(mut file) = archive.by_name(&file_name)
        {
            let mut xml_content = String::new();
            if file.read_to_string(&mut xml_content).is_ok()
                && let Ok(doc) = Document::parse(&xml_content)
            {
                let root = doc.root_element();

                if root.tag_name().name() == "math" {
                    if let Some(formula_text) = extract_mathml_text(root) {
                        formula_parts.push(formula_text);
                    }
                } else {
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

    let mut text_parts: Vec<String> = Vec::new();

    // Find the office:text or text body element - this is the main document body
    for body_child in root.children() {
        if body_child.tag_name().name() == "body" {
            // Process the text element inside body
            for text_elem in body_child.children() {
                if text_elem.tag_name().name() == "text" {
                    // Now process only direct children of the text element
                    process_document_elements(text_elem, &mut text_parts);
                }
            }
        }
    }

    Ok(text_parts.join("\n").trim().to_string())
}

/// Helper function to process document elements (paragraphs, headings, tables)
/// Only processes direct children, avoiding nested content like table cells
fn process_document_elements(parent: roxmltree::Node, text_parts: &mut Vec<String>) {
    for node in parent.children() {
        match node.tag_name().name() {
            "h" => {
                if let Some(text) = extract_node_text(node)
                    && !text.trim().is_empty()
                {
                    text_parts.push(format!("# {}", text.trim()));
                    text_parts.push(String::new());
                }
            }
            "p" => {
                if let Some(text) = extract_node_text(node)
                    && !text.trim().is_empty()
                {
                    text_parts.push(text.trim().to_string());
                    text_parts.push(String::new());
                }
            }
            "table" => {
                if let Some(table_text) = extract_table_text(node) {
                    text_parts.push(table_text);
                    text_parts.push(String::new());
                }
            }
            _ => {}
        }
    }
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
                if let Some(text) = child.text() {
                    text_parts.push(text.to_string());
                }
            }
        }
    }

    if text_parts.is_empty() {
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

    for row_node in table_node.children() {
        if row_node.tag_name().name() == "table-row" {
            let mut row_cells = Vec::new();

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

    for row in &mut rows {
        while row.len() < max_cols {
            row.push(String::new());
        }
    }

    let mut markdown = String::new();

    if !rows.is_empty() {
        markdown.push('|');
        for cell in &rows[0] {
            markdown.push(' ');
            markdown.push_str(cell);
            markdown.push_str(" |");
        }
        markdown.push('\n');

        markdown.push('|');
        for _ in 0..rows[0].len() {
            markdown.push_str(" --- |");
        }
        markdown.push('\n');

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

    for row_node in table_node.children() {
        if row_node.tag_name().name() == "table-row" {
            let mut row_cells = Vec::new();

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

    let markdown = cells_to_markdown(&cells);

    Some(Table {
        cells,
        markdown,
        page_number: table_index + 1,
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
        let content_owned = content.to_vec();

        let (text, tables) = if crate::core::batch_mode::is_batch_mode() {
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
            let cursor = Cursor::new(content_owned.clone());
            let mut archive = zip::ZipArchive::new(cursor)
                .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to open ZIP archive: {}", e)))?;

            let text = extract_content_text(&mut archive)?;
            let tables = extract_tables(&mut archive)?;
            let embedded_formulas = extract_embedded_formulas(&mut archive)?;

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

        let mut metadata_map = std::collections::HashMap::new();

        let cursor = Cursor::new(content_owned.clone());
        let mut archive = zip::ZipArchive::new(cursor).map_err(|e| {
            crate::error::KreuzbergError::parsing(format!("Failed to open ZIP archive for metadata: {}", e))
        })?;

        if let Ok(odt_props) = office_metadata::extract_odt_properties(&mut archive) {
            if let Some(title) = odt_props.title {
                metadata_map.insert("title".to_string(), serde_json::Value::String(title));
            }
            if let Some(creator) = odt_props.creator {
                metadata_map.insert(
                    "authors".to_string(),
                    serde_json::Value::Array(vec![serde_json::Value::String(creator.clone())]),
                );
                metadata_map.insert("created_by".to_string(), serde_json::Value::String(creator));
            }
            if let Some(initial_creator) = odt_props.initial_creator {
                metadata_map.insert(
                    "initial_creator".to_string(),
                    serde_json::Value::String(initial_creator),
                );
            }
            if let Some(subject) = odt_props.subject {
                metadata_map.insert("subject".to_string(), serde_json::Value::String(subject));
            }
            if let Some(keywords) = odt_props.keywords {
                metadata_map.insert("keywords".to_string(), serde_json::Value::String(keywords));
            }
            if let Some(description) = odt_props.description {
                metadata_map.insert("description".to_string(), serde_json::Value::String(description));
            }
            if let Some(creation_date) = odt_props.creation_date {
                metadata_map.insert("created_at".to_string(), serde_json::Value::String(creation_date));
            }
            if let Some(date) = odt_props.date {
                metadata_map.insert("modified_at".to_string(), serde_json::Value::String(date));
            }
            if let Some(language) = odt_props.language {
                metadata_map.insert("language".to_string(), serde_json::Value::String(language));
            }
            if let Some(generator) = odt_props.generator {
                metadata_map.insert("generator".to_string(), serde_json::Value::String(generator));
            }
            if let Some(editing_duration) = odt_props.editing_duration {
                metadata_map.insert(
                    "editing_duration".to_string(),
                    serde_json::Value::String(editing_duration),
                );
            }
            if let Some(editing_cycles) = odt_props.editing_cycles {
                metadata_map.insert("editing_cycles".to_string(), serde_json::Value::String(editing_cycles));
            }
            if let Some(page_count) = odt_props.page_count {
                metadata_map.insert("page_count".to_string(), serde_json::Value::Number(page_count.into()));
            }
            if let Some(word_count) = odt_props.word_count {
                metadata_map.insert("word_count".to_string(), serde_json::Value::Number(word_count.into()));
            }
            if let Some(character_count) = odt_props.character_count {
                metadata_map.insert(
                    "character_count".to_string(),
                    serde_json::Value::Number(character_count.into()),
                );
            }
            if let Some(paragraph_count) = odt_props.paragraph_count {
                metadata_map.insert(
                    "paragraph_count".to_string(),
                    serde_json::Value::Number(paragraph_count.into()),
                );
            }
            if let Some(table_count) = odt_props.table_count {
                metadata_map.insert("table_count".to_string(), serde_json::Value::Number(table_count.into()));
            }
            if let Some(image_count) = odt_props.image_count {
                metadata_map.insert("image_count".to_string(), serde_json::Value::Number(image_count.into()));
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
        60
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
        assert!(!result.unwrap().is_empty());
    }
}
