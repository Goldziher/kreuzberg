//! DocBook document extractor supporting both 4.x and 5.x formats.
//!
//! This extractor handles DocBook XML documents in both traditional (4.x, no namespace)
//! and modern (5.x, with http://docbook.org/ns/docbook namespace) formats.
//!
//! It extracts:
//! - Document metadata (title, author, date, abstract)
//! - Section hierarchy and content
//! - Paragraphs and text content
//! - Code blocks (programlisting)
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

/// Extract text content from a DocBook element and its children.
/// Handles both namespaced (DocBook 5) and non-namespaced (DocBook 4) content.
fn extract_text_content(reader: &mut Reader<&[u8]>) -> Result<String> {
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

/// Extract tables from DocBook content.
fn extract_docbook_tables(content: &str) -> Result<Vec<Table>> {
    let mut reader = Reader::from_str(content);
    let mut tables = Vec::new();
    let mut in_table = false;
    let mut in_tgroup = false;
    let mut in_thead = false;
    let mut in_tbody = false;
    let mut in_row = false;
    let mut current_table: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut table_index = 0;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let tag = strip_namespace(&tag);

                match tag {
                    "table" | "informaltable" => {
                        in_table = true;
                        current_table.clear();
                    }
                    "tgroup" if in_table => {
                        in_tgroup = true;
                    }
                    "thead" if in_tgroup => {
                        in_thead = true;
                    }
                    "tbody" if in_tgroup => {
                        in_tbody = true;
                    }
                    "row" if (in_thead || in_tbody) && in_tgroup => {
                        in_row = true;
                        current_row.clear();
                    }
                    "entry" if in_row => {
                        // Extract entry content
                        let mut entry_text = String::new();
                        let mut entry_depth = 0;

                        loop {
                            match reader.read_event() {
                                Ok(Event::Start(_)) => {
                                    entry_depth += 1;
                                }
                                Ok(Event::End(e)) => {
                                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                                    let tag = strip_namespace(&tag);
                                    if tag == "entry" && entry_depth == 0 {
                                        break;
                                    }
                                    if entry_depth > 0 {
                                        entry_depth -= 1;
                                    }
                                }
                                Ok(Event::Text(t)) => {
                                    let decoded = String::from_utf8_lossy(t.as_ref()).to_string();
                                    if !decoded.trim().is_empty() {
                                        if !entry_text.is_empty() {
                                            entry_text.push(' ');
                                        }
                                        entry_text.push_str(decoded.trim());
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

                        current_row.push(entry_text);
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let tag = strip_namespace(&tag);

                match tag {
                    "table" | "informaltable" if in_table => {
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
                        in_table = false;
                    }
                    "tgroup" if in_tgroup => {
                        in_tgroup = false;
                    }
                    "thead" if in_thead => {
                        in_thead = false;
                    }
                    "tbody" if in_tbody => {
                        in_tbody = false;
                    }
                    "row" if in_row => {
                        if !current_row.is_empty() {
                            current_table.push(current_row.clone());
                            current_row.clear();
                        }
                        in_row = false;
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

    Ok(tables)
}

/// Parse DocBook content and extract structured text and metadata.
fn parse_docbook_content(content: &str) -> Result<(String, String, Option<String>, Option<String>)> {
    let mut reader = Reader::from_str(content);
    let mut output = String::new();
    let mut title = String::new();
    let mut author = Option::None;
    let mut date = Option::None;
    let mut in_info = false;
    let mut title_extracted = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let tag = strip_namespace(&tag);

                match tag {
                    "info" | "articleinfo" | "bookinfo" | "chapterinfo" => {
                        in_info = true;
                    }
                    "title" if !title_extracted && in_info => {
                        let title_text = extract_text_content(&mut reader)?;
                        if !title_text.is_empty() && title.is_empty() {
                            title = title_text.clone();
                            title_extracted = true;
                        }
                    }
                    "title" if !title_extracted => {
                        // First title outside info block (e.g., document title)
                        let title_text = extract_text_content(&mut reader)?;
                        if !title_text.is_empty() && title.is_empty() {
                            title = title_text.clone();
                            title_extracted = true;
                        }
                    }
                    "title" => {
                        // Titles of sections, chapters, etc.
                        let section_title = extract_text_content(&mut reader)?;
                        if !section_title.is_empty() && title_extracted {
                            // Add section title to output (only if we already have a document title)
                            output.push_str(&section_title);
                            output.push_str("\n\n");
                        }
                    }
                    "author" | "personname" if in_info => {
                        if author.is_none() {
                            let author_text = extract_text_content(&mut reader)?;
                            if !author_text.is_empty() {
                                author = Some(author_text);
                            }
                        }
                    }
                    "date" if in_info => {
                        let date_text = extract_text_content(&mut reader)?;
                        if !date_text.is_empty() && date.is_none() {
                            date = Some(date_text);
                        }
                    }
                    "para" => {
                        let para_text = extract_text_content(&mut reader)?;
                        if !para_text.is_empty() {
                            output.push_str(&para_text);
                            output.push_str("\n\n");
                        }
                    }
                    "programlisting" | "screen" => {
                        let code_text = extract_text_content(&mut reader)?;
                        if !code_text.is_empty() {
                            output.push_str("```\n");
                            output.push_str(&code_text);
                            output.push_str("\n```\n\n");
                        }
                    }
                    "section" | "sect1" | "sect2" | "sect3" | "sect4" | "sect5" | "simplesect" | "chapter"
                    | "article" | "book" => {
                        // These are structural elements, their titles will be handled below
                    }
                    "emphasis" | "phrase" | "ulink" | "link" | "xref" => {
                        // These are inline elements, content will be handled by text extraction
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let tag = strip_namespace(&tag);

                if matches!(tag, "info" | "articleinfo" | "bookinfo" | "chapterinfo") {
                    in_info = false;
                }
            }
            Ok(Event::Text(_t)) => {
                // Skip loose text nodes when not in paragraph/code
                // They're handled by element-specific extraction
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

    // Prepend title to output if it was extracted
    let mut final_output = output;
    if !title.is_empty() {
        final_output = format!("{}\n\n{}", title, final_output);
    }

    Ok((final_output.trim().to_string(), title, author, date))
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

        // Parse DocBook content
        let (extracted_content, title, author, date) = parse_docbook_content(&docbook_content)?;

        // Extract tables
        let tables = extract_docbook_tables(&docbook_content)?;

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
