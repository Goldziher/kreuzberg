//! FictionBook (FB2) document extractor supporting FictionBook 2.0 format.
//!
//! This extractor handles FictionBook XML documents (FB2), an XML-based e-book format
//! popular in Russian-speaking countries.
//!
//! It extracts:
//! - Document metadata (genre, language)
//! - Section hierarchy and content
//! - Paragraphs and text content with inline formatting
//! - Inline markup: emphasis, strong, strikethrough, subscript, superscript, code
//! - Blockquotes and notes

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata};
use async_trait::async_trait;
use quick_xml::Reader;
use quick_xml::events::Event;

/// FictionBook document extractor.
///
/// Supports FictionBook 2.0 format with proper section hierarchy and inline formatting.
pub struct FictionBookExtractor;

impl Default for FictionBookExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl FictionBookExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Extract paragraph content with markdown formatting preservation.
    /// Handles inline formatting tags like emphasis (*), strong (**), strikethrough (~~), etc.
    fn extract_paragraph_content(reader: &mut Reader<&[u8]>) -> Result<String> {
        let mut text = String::new();
        let mut para_depth = 0;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    match tag.as_str() {
                        "emphasis" => {
                            text.push('*');
                        }
                        "strong" => {
                            text.push_str("**");
                        }
                        "strikethrough" => {
                            text.push_str("~~");
                        }
                        "code" => {
                            text.push('`');
                        }
                        "sub" => {
                            text.push('~');
                        }
                        "sup" => {
                            text.push('^');
                        }
                        "a" | "empty-line" => {}
                        _ => {}
                    }
                    para_depth += 1;
                }
                Ok(Event::End(e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    match tag.as_str() {
                        "p" if para_depth == 1 => {
                            break;
                        }
                        "emphasis" => {
                            text.push('*');
                        }
                        "strong" => {
                            text.push_str("**");
                        }
                        "strikethrough" => {
                            text.push_str("~~");
                        }
                        "code" => {
                            text.push('`');
                        }
                        "sub" => {
                            text.push('~');
                        }
                        "sup" => {
                            text.push('^');
                        }
                        _ => {}
                    }
                    if para_depth > 0 {
                        para_depth -= 1;
                    }
                }
                Ok(Event::Text(t)) => {
                    let decoded = String::from_utf8_lossy(t.as_ref()).to_string();
                    let trimmed = decoded.trim();
                    if !trimmed.is_empty() {
                        if !text.is_empty()
                            && !text.ends_with(' ')
                            && !text.ends_with('*')
                            && !text.ends_with('`')
                            && !text.ends_with('~')
                            && !text.ends_with('^')
                        {
                            text.push(' ');
                        }
                        text.push_str(trimmed);
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

    /// Extract text content from a FictionBook element and its children.
    fn extract_text_content(reader: &mut Reader<&[u8]>) -> Result<String> {
        let mut text = String::new();
        let mut depth = 0;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    match tag.as_str() {
                        "emphasis" | "strong" | "strikethrough" | "code" | "sub" | "sup" => {}
                        "empty-line" => {
                            text.push('\n');
                        }
                        _ => {}
                    }
                    depth += 1;
                }
                Ok(Event::End(e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                    if (tag == "p" || tag == "cite" || tag == "section") && !text.is_empty() && !text.ends_with('\n') {
                        text.push('\n');
                    }
                }
                Ok(Event::Text(t)) => {
                    let decoded = String::from_utf8_lossy(t.as_ref()).to_string();
                    let trimmed = decoded.trim();
                    if !trimmed.is_empty() {
                        if !text.is_empty() && !text.ends_with(' ') && !text.ends_with('\n') {
                            text.push(' ');
                        }
                        text.push_str(trimmed);
                        text.push(' ');
                    }
                }
                Ok(Event::CData(t)) => {
                    let decoded = String::from_utf8_lossy(t.as_ref()).to_string();
                    if !decoded.trim().is_empty() {
                        if !text.is_empty() && !text.ends_with('\n') {
                            text.push('\n');
                        }
                        text.push_str(&decoded);
                        text.push('\n');
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

        let text = text
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(text)
    }

    /// Extract metadata from FictionBook document.
    fn extract_metadata(data: &[u8]) -> Result<Metadata> {
        let mut reader = Reader::from_reader(data);
        let mut metadata = Metadata::default();
        let mut in_title_info = false;
        let mut in_description = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    match tag.as_str() {
                        "description" => {
                            in_description = true;
                        }
                        "title-info" if in_description => {
                            in_title_info = true;
                        }
                        "genre" if in_title_info => {
                            if let Ok(Event::Text(t)) = reader.read_event() {
                                let genre = String::from_utf8_lossy(t.as_ref()).to_string();
                                if !genre.trim().is_empty() && genre.trim() != "unrecognised" {
                                    metadata.subject = Some(genre.trim().to_string());
                                }
                            }
                        }
                        "date" if in_title_info => {
                            if let Ok(Event::Text(t)) = reader.read_event() {
                                let date = String::from_utf8_lossy(t.as_ref()).to_string();
                                if !date.trim().is_empty() {
                                    metadata.date = Some(date.trim().to_string());
                                }
                            }
                        }
                        "lang" if in_title_info => {
                            if let Ok(Event::Text(t)) = reader.read_event() {
                                let lang = String::from_utf8_lossy(t.as_ref()).to_string();
                                if !lang.trim().is_empty() {
                                    metadata.language = Some(lang.trim().to_string());
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if tag == "title-info" {
                        in_title_info = false;
                    } else if tag == "description" {
                        in_description = false;
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        Ok(metadata)
    }

    /// Extract content from FictionBook document body sections.
    fn extract_body_content(data: &[u8]) -> Result<String> {
        let mut reader = Reader::from_reader(data);
        let mut content = String::new();
        let mut in_body = false;
        let mut skip_notes_body = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    if tag == "body" {
                        for a in e.attributes().flatten() {
                            let attr_name = String::from_utf8_lossy(a.key.as_ref()).to_string();
                            if attr_name == "name" {
                                let val = String::from_utf8_lossy(a.value.as_ref());
                                if val == "notes" {
                                    skip_notes_body = true;
                                    break;
                                }
                            }
                        }

                        if !skip_notes_body {
                            in_body = true;
                        }
                    } else if tag == "section" && in_body {
                        match Self::extract_section_content(&mut reader) {
                            Ok(section_content) if !section_content.is_empty() => {
                                content.push_str(&section_content);
                                content.push('\n');
                            }
                            _ => {}
                        }
                    } else if tag == "p" && in_body && !skip_notes_body {
                        match Self::extract_paragraph_content(&mut reader) {
                            Ok(para) if !para.is_empty() => {
                                content.push_str(&para);
                                content.push('\n');
                            }
                            _ => {}
                        }
                    } else if tag == "title" && in_body {
                        match Self::extract_text_content(&mut reader) {
                            Ok(title_content) if !title_content.is_empty() => {
                                content.push_str(&format!("# {}\n", title_content));
                            }
                            _ => {}
                        }
                    }
                }
                Ok(Event::End(e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if tag == "body" {
                        if skip_notes_body {
                            skip_notes_body = false;
                        } else {
                            in_body = false;
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        Ok(content.trim().to_string())
    }

    /// Extract content from a section with proper hierarchy.
    fn extract_section_content(reader: &mut Reader<&[u8]>) -> Result<String> {
        let mut content = String::new();
        let mut section_depth = 1;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    match tag.as_str() {
                        "section" => {
                            section_depth += 1;
                        }
                        "title" => match Self::extract_text_content(reader) {
                            Ok(title_text) if !title_text.is_empty() => {
                                let heading_level = std::cmp::min(section_depth + 1, 6);
                                let heading = "#".repeat(heading_level);
                                content.push_str(&format!("{} {}\n", heading, title_text));
                            }
                            _ => {}
                        },
                        "p" => match Self::extract_paragraph_content(reader) {
                            Ok(para) if !para.is_empty() => {
                                content.push_str(&para);
                                content.push('\n');
                            }
                            _ => {}
                        },
                        "cite" => match Self::extract_text_content(reader) {
                            Ok(cite_content) if !cite_content.is_empty() => {
                                content.push_str("> ");
                                content.push_str(&cite_content);
                                content.push('\n');
                            }
                            _ => {}
                        },
                        "empty-line" => {
                            content.push('\n');
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if tag == "section" {
                        section_depth -= 1;
                        if section_depth == 0 {
                            break;
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        Ok(content.trim().to_string())
    }
}

impl Plugin for FictionBookExtractor {
    fn name(&self) -> &str {
        "fictionbook-extractor"
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
        "Extracts content and metadata from FictionBook documents (FB2 format)"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[async_trait]
impl DocumentExtractor for FictionBookExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let metadata = Self::extract_metadata(content)?;

        let extracted_content = Self::extract_body_content(content)?;

        Ok(ExtractionResult {
            content: extracted_content,
            mime_type: mime_type.to_string(),
            metadata,
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[
            "application/x-fictionbook+xml",
            "text/x-fictionbook",
            "application/x-fictionbook",
        ]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(all(test, feature = "office"))]
mod tests {
    use super::*;

    #[test]
    fn test_fictionbook_extractor_plugin_interface() {
        let extractor = FictionBookExtractor::new();
        assert_eq!(extractor.name(), "fictionbook-extractor");
        assert_eq!(extractor.priority(), 50);
        assert!(!extractor.supported_mime_types().is_empty());
    }

    #[test]
    fn test_fictionbook_extractor_default() {
        let extractor = FictionBookExtractor::default();
        assert_eq!(extractor.name(), "fictionbook-extractor");
    }

    #[test]
    fn test_fictionbook_extractor_supported_mime_types() {
        let extractor = FictionBookExtractor::new();
        let supported = extractor.supported_mime_types();
        assert!(supported.contains(&"application/x-fictionbook+xml"));
        assert!(supported.contains(&"text/x-fictionbook"));
    }

    #[tokio::test]
    async fn test_fictionbook_extractor_initialize_shutdown() {
        let extractor = FictionBookExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }
}
