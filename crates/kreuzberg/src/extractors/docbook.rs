//! DocBook document extractor supporting both 4.x and 5.x formats.
//!
//! This extractor handles DocBook XML documents in both traditional (4.x, no namespace)
//! and modern (5.x, with http://docbook.org/ns/docbook namespace) formats.
//!
//! Single-pass architecture that extracts in one document traversal:
//! - Document metadata (title, author, date, abstract)
//! - Section hierarchy and content
//! - Paragraphs and text content
//! - Lists (itemizedlist, orderedlist)
//! - Code blocks (programlisting, screen)
//! - Blockquotes
//! - Figures and mediaobjects
//! - Footnotes
//! - Tables
//! - Cross-references and links

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::cells_to_markdown;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata, Table};
use async_trait::async_trait;
use quick_xml::Reader;
use quick_xml::events::Event;
use std::path::Path;

/// Strip namespace prefix from XML tag names.
/// Converts "{http://docbook.org/ns/docbook}title" to "title"
/// and leaves non-namespaced "title" unchanged.
fn strip_namespace(tag: &str) -> &str {
    if tag.starts_with('{') {
        if let Some(pos) = tag.find('}') {
            return &tag[pos + 1..];
        }
    }
    tag
}

/// State machine for tracking nested elements during extraction
#[derive(Debug, Clone, Copy)]
struct ParsingState {
    in_info: bool,
    in_table: bool,
    in_tgroup: bool,
    in_thead: bool,
    in_tbody: bool,
    in_row: bool,
    in_list: bool,
    in_list_item: bool,
}

/// DocBook document extractor.
///
/// Supports both DocBook 4.x (no namespace) and 5.x (with namespace) formats.
pub struct DocbookExtractor;

impl Default for DocbookExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl DocbookExtractor {
    pub fn new() -> Self {
        Self
    }
}

/// Single-pass DocBook parser that extracts all content in one document traversal.
/// Returns: (content, title, author, date, tables)
fn parse_docbook_single_pass(content: &str) -> Result<(String, String, Option<String>, Option<String>, Vec<Table>)> {
    let mut reader = Reader::from_str(content);
    let mut output = String::new();
    let mut title = String::new();
    let mut author = Option::None;
    let mut date = Option::None;
    let mut tables = Vec::new();
    let mut table_index = 0;

    let mut state = ParsingState {
        in_info: false,
        in_table: false,
        in_tgroup: false,
        in_thead: false,
        in_tbody: false,
        in_row: false,
        in_list: false,
        in_list_item: false,
    };

    let mut title_extracted = false;
    let mut current_table: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut list_type = ""; // "ordered" or "itemized"

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let tag = strip_namespace(&tag);

                match tag {
                    // Metadata extraction
                    "info" | "articleinfo" | "bookinfo" | "chapterinfo" => {
                        state.in_info = true;
                    }
                    "title" if !title_extracted && state.in_info => {
                        title = extract_element_text(&mut reader)?;
                        title_extracted = true;
                    }
                    "title" if !title_extracted => {
                        title = extract_element_text(&mut reader)?;
                        title_extracted = true;
                    }
                    "title" if title_extracted => {
                        let section_title = extract_element_text(&mut reader)?;
                        if !section_title.is_empty() {
                            output.push_str("## ");
                            output.push_str(&section_title);
                            output.push_str("\n\n");
                        }
                    }
                    "author" | "personname" if state.in_info && author.is_none() => {
                        author = Some(extract_element_text(&mut reader)?);
                    }
                    "date" if state.in_info && date.is_none() => {
                        let date_text = extract_element_text(&mut reader)?;
                        if !date_text.is_empty() {
                            date = Some(date_text);
                        }
                    }

                    // Paragraph content
                    "para" => {
                        let para_text = extract_element_text(&mut reader)?;
                        if !para_text.is_empty() {
                            output.push_str(&para_text);
                            output.push_str("\n\n");
                        }
                    }

                    // Code blocks
                    "programlisting" | "screen" => {
                        let code_text = extract_element_text(&mut reader)?;
                        if !code_text.is_empty() {
                            output.push_str("```\n");
                            output.push_str(&code_text);
                            output.push_str("\n```\n\n");
                        }
                    }

                    // Lists
                    "itemizedlist" => {
                        state.in_list = true;
                        list_type = "itemized";
                    }
                    "orderedlist" => {
                        state.in_list = true;
                        list_type = "ordered";
                    }
                    "listitem" if state.in_list => {
                        state.in_list_item = true;
                        let prefix = if list_type == "ordered" { "1. " } else { "- " };
                        output.push_str(prefix);
                        let item_text = extract_element_text(&mut reader)?;
                        if !item_text.is_empty() {
                            output.push_str(&item_text);
                        }
                        output.push('\n');
                        state.in_list_item = false;
                    }

                    // Blockquotes
                    "blockquote" => {
                        output.push_str("> ");
                        let quote_text = extract_element_text(&mut reader)?;
                        if !quote_text.is_empty() {
                            output.push_str(&quote_text);
                        }
                        output.push_str("\n\n");
                    }

                    // Figures
                    "figure" => {
                        let figure_text = extract_element_text(&mut reader)?;
                        if !figure_text.is_empty() {
                            output.push_str("**Figure:** ");
                            output.push_str(&figure_text);
                            output.push_str("\n\n");
                        }
                    }

                    // Footnotes
                    "footnote" => {
                        output.push('[');
                        let footnote_text = extract_element_text(&mut reader)?;
                        if !footnote_text.is_empty() {
                            output.push_str(&footnote_text);
                        }
                        output.push_str("]");
                    }

                    // Tables - single-pass table extraction
                    "table" | "informaltable" => {
                        state.in_table = true;
                        current_table.clear();
                    }
                    "tgroup" if state.in_table => {
                        state.in_tgroup = true;
                    }
                    "thead" if state.in_tgroup => {
                        state.in_thead = true;
                    }
                    "tbody" if state.in_tgroup => {
                        state.in_tbody = true;
                    }
                    "row" if (state.in_thead || state.in_tbody) && state.in_tgroup => {
                        state.in_row = true;
                        current_row.clear();
                    }
                    "entry" if state.in_row => {
                        let entry_text = extract_element_text(&mut reader)?;
                        current_row.push(entry_text);
                    }

                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let tag = strip_namespace(&tag);

                match tag {
                    "info" | "articleinfo" | "bookinfo" | "chapterinfo" => {
                        state.in_info = false;
                    }
                    "itemizedlist" | "orderedlist" if state.in_list => {
                        output.push('\n');
                        state.in_list = false;
                    }
                    "table" | "informaltable" if state.in_table => {
                        if !current_table.is_empty() {
                            let markdown = cells_to_markdown(&current_table);
                            tables.push(Table {
                                cells: current_table.clone(),
                                markdown,
                                page_number: table_index + 1,
                            });
                            table_index += 1;
                            current_table.clear();
                        }
                        state.in_table = false;
                    }
                    "tgroup" if state.in_tgroup => {
                        state.in_tgroup = false;
                    }
                    "thead" if state.in_thead => {
                        state.in_thead = false;
                    }
                    "tbody" if state.in_tbody => {
                        state.in_tbody = false;
                    }
                    "row" if state.in_row => {
                        if !current_row.is_empty() {
                            current_table.push(current_row.clone());
                            current_row.clear();
                        }
                        state.in_row = false;
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(crate::error::KreuzbergError::parsing(format!(
                    "XML parsing error: {}",
                    e
                )));
            }
            _ => {}
        }
    }

    // Prepend title to output
    let mut final_output = output;
    if !title.is_empty() {
        final_output = format!("{}\n\n{}", title, final_output);
    }

    Ok((final_output.trim().to_string(), title, author, date, tables))
}

/// Extract text content from a DocBook element and its children.
/// Used for extracting nested content within elements.
fn extract_element_text(reader: &mut Reader<&[u8]>) -> Result<String> {
    let mut text = String::new();
    let mut depth = 0;

    loop {
        match reader.read_event() {
            Ok(Event::Start(_)) => {
                depth += 1;
            }
            Ok(Event::End(_)) => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            Ok(Event::Text(t)) => {
                let decoded = String::from_utf8_lossy(t.as_ref()).to_string();
                if !decoded.trim().is_empty() {
                    if !text.is_empty() && !text.ends_with(' ') && !text.ends_with('\n') {
                        text.push(' ');
                    }
                    text.push_str(decoded.trim());
                }
            }
            Ok(Event::CData(t)) => {
                let decoded = std::str::from_utf8(t.as_ref()).unwrap_or("").to_string();
                if !decoded.trim().is_empty() {
                    if !text.is_empty() {
                        text.push(' ');
                    }
                    text.push_str(decoded.trim());
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(crate::error::KreuzbergError::parsing(format!(
                    "XML parsing error: {}",
                    e
                )));
            }
            _ => {}
        }
    }

    Ok(text.trim().to_string())
}

impl Plugin for DocbookExtractor {
    fn name(&self) -> &str {
        "docbook-extractor"
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
impl DocumentExtractor for DocbookExtractor {
    #[cfg_attr(
        feature = "otel",
        tracing::instrument(
            skip(self, content, config),
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
        let docbook_content = std::str::from_utf8(content)
            .map(|s| s.to_string())
            .unwrap_or_else(|_| String::from_utf8_lossy(content).to_string());

        // Single-pass extraction: metadata, content, and tables in one traversal
        let (extracted_content, title, author, date, tables) = parse_docbook_single_pass(&docbook_content)?;

        // Build metadata
        let mut metadata = Metadata::default();
        let mut subject_parts = Vec::new();

        if !title.is_empty() {
            subject_parts.push(format!("Title: {}", title));
        }
        if let Some(author) = &author {
            subject_parts.push(format!("Author: {}", author));
        }

        if !subject_parts.is_empty() {
            metadata.subject = Some(subject_parts.join("; "));
        }

        if let Some(date_val) = date {
            metadata.date = Some(date_val);
        }

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

    #[cfg_attr(
        feature = "otel",
        tracing::instrument(
            skip(self, path, config),
            fields(
                extractor.name = self.name(),
            )
        )
    )]
    async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
        let bytes = tokio::fs::read(path).await?;
        self.extract_bytes(&bytes, mime_type, config).await
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/docbook+xml", "text/docbook"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docbook_extractor_plugin_interface() {
        let extractor = DocbookExtractor::new();
        assert_eq!(extractor.name(), "docbook-extractor");
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_docbook_extractor_supported_mime_types() {
        let extractor = DocbookExtractor::new();
        let mime_types = extractor.supported_mime_types();
        assert_eq!(mime_types.len(), 2);
        assert!(mime_types.contains(&"application/docbook+xml"));
        assert!(mime_types.contains(&"text/docbook"));
    }

    #[test]
    fn test_docbook_extractor_priority() {
        let extractor = DocbookExtractor::new();
        assert_eq!(extractor.priority(), 50);
    }

    #[test]
    fn test_parse_simple_docbook() {
        let docbook = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE article PUBLIC "-//OASIS//DTD DocBook XML V4.4//EN"
"http://www.oasis-open.org/docbook/xml/4.4/docbookx.dtd">
<article>
  <title>Test Article</title>
  <para>Test content.</para>
</article>"#;

        let (content, title, _, _) = parse_docbook_content(docbook).expect("Parse failed");
        assert_eq!(title, "Test Article");
        assert!(content.contains("Test content"));
    }

    #[test]
    fn test_extract_docbook_tables_basic() {
        let docbook = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <table>
    <tgroup cols="2">
      <thead>
        <row>
          <entry>Col1</entry>
          <entry>Col2</entry>
        </row>
      </thead>
      <tbody>
        <row>
          <entry>Data1</entry>
          <entry>Data2</entry>
        </row>
      </tbody>
    </tgroup>
  </table>
</article>"#;

        let tables = extract_docbook_tables(docbook).expect("Table extraction failed");
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].cells.len(), 2);
        assert_eq!(tables[0].cells[0], vec!["Col1", "Col2"]);
    }
}
