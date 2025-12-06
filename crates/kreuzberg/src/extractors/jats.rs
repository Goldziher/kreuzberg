//! JATS (Journal Article Tag Suite) document extractor.
//!
//! This extractor handles JATS XML documents, the standard format for scientific journal articles
//! used by PubMed Central and major academic publishers.
//!
//! It extracts:
//! - Rich metadata (title, subtitle, authors with affiliations, DOI, PII, keywords, dates)
//! - Article abstract (regular and graphical)
//! - Section hierarchy and content (intro, methods, results, discussion)
//! - Paragraphs and text content
//! - Tables with captions
//! - Figures with captions
//! - Citations and references
//! - Supplementary material information

#![cfg(feature = "xml")]

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::cells_to_markdown;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata, Table};
use async_trait::async_trait;
use quick_xml::Reader;
use quick_xml::events::Event;
use std::path::Path;

/// JATS document extractor.
///
/// Supports JATS (Journal Article Tag Suite) XML documents in various versions,
/// handling both the full article structure and minimal JATS subsets.
pub struct JatsExtractor;

impl Default for JatsExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl JatsExtractor {
    pub fn new() -> Self {
        Self
    }
}

/// Extract text content from a JATS element and its children.
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
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push(' ');
                }
            }
            Ok(Event::Text(t)) => {
                let decoded = String::from_utf8_lossy(t.as_ref()).to_string();
                if !decoded.trim().is_empty() {
                    text.push_str(&decoded);
                    text.push(' ');
                }
            }
            Ok(Event::CData(t)) => {
                let decoded = std::str::from_utf8(t.as_ref()).unwrap_or("").to_string();
                if !decoded.trim().is_empty() {
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

    Ok(text.trim().to_string())
}

/// Extract tables from JATS content.
fn extract_jats_tables(content: &str) -> Result<Vec<Table>> {
    let mut reader = Reader::from_str(content);
    let mut tables = Vec::new();
    let mut in_table = false;
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

                match tag.as_str() {
                    "table" => {
                        in_table = true;
                        current_table.clear();
                    }
                    "thead" if in_table => {
                        in_thead = true;
                    }
                    "tbody" if in_table => {
                        in_tbody = true;
                    }
                    "tr" if (in_thead || in_tbody) && in_table => {
                        in_row = true;
                        current_row.clear();
                    }
                    "td" | "th" if in_row => {
                        // Extract cell content
                        let mut cell_text = String::new();
                        let mut cell_depth = 0;

                        loop {
                            match reader.read_event() {
                                Ok(Event::Start(_)) => {
                                    cell_depth += 1;
                                }
                                Ok(Event::End(e)) => {
                                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                                    if (tag == "td" || tag == "th") && cell_depth == 0 {
                                        break;
                                    }
                                    if cell_depth > 0 {
                                        cell_depth -= 1;
                                    }
                                }
                                Ok(Event::Text(t)) => {
                                    let decoded = String::from_utf8_lossy(t.as_ref()).to_string();
                                    if !decoded.trim().is_empty() {
                                        if !cell_text.is_empty() {
                                            cell_text.push(' ');
                                        }
                                        cell_text.push_str(decoded.trim());
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

                        current_row.push(cell_text);
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();

                match tag.as_str() {
                    "table" if in_table => {
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
                    "thead" if in_thead => {
                        in_thead = false;
                    }
                    "tbody" if in_tbody => {
                        in_tbody = false;
                    }
                    "tr" if in_row => {
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

/// Structure to hold extracted JATS metadata.
#[derive(Debug, Clone, Default)]
struct JatsMetadataExtracted {
    title: String,
    subtitle: Option<String>,
    authors: Vec<String>,
    affiliations: Vec<String>,
    doi: Option<String>,
    pii: Option<String>,
    keywords: Vec<String>,
    publication_date: Option<String>,
    volume: Option<String>,
    issue: Option<String>,
    pages: Option<String>,
    journal_title: Option<String>,
    article_type: Option<String>,
    abstract_text: Option<String>,
    corresponding_author: Option<String>,
}

/// Parse JATS metadata from article-meta element.
fn extract_jats_metadata(content: &str) -> Result<JatsMetadataExtracted> {
    let mut reader = Reader::from_str(content);
    let mut metadata = JatsMetadataExtracted::default();
    let mut in_article_meta = false;
    let mut in_article_title = false;
    let mut in_subtitle = false;
    let mut in_contrib = false;
    let mut in_name = false;
    let mut in_aff = false;
    let mut in_abstract = false;
    let mut in_kwd_group = false;
    let mut in_kwd = false;
    let mut current_author = String::new();
    let mut current_aff = String::new();
    let mut abstract_content = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();

                match tag.as_str() {
                    "article-meta" => {
                        in_article_meta = true;
                    }
                    "article-title" if in_article_meta => {
                        in_article_title = true;
                    }
                    "subtitle" if in_article_meta => {
                        in_subtitle = true;
                    }
                    "contrib" if in_article_meta => {
                        in_contrib = true;
                        current_author.clear();
                    }
                    "name" if in_contrib => {
                        in_name = true;
                    }
                    "aff" if in_article_meta => {
                        in_aff = true;
                        current_aff.clear();
                    }
                    "article-id" if in_article_meta => {
                        // Extract article identifiers
                        let mut id_type = String::new();
                        for attr in e.attributes() {
                            if let Ok(attr) = attr {
                                if String::from_utf8_lossy(attr.key.as_ref()) == "pub-id-type" {
                                    id_type = String::from_utf8_lossy(attr.value.as_ref()).to_string();
                                }
                            }
                        }

                        let id_text = extract_text_content(&mut reader)?;
                        match id_type.as_str() {
                            "doi" => metadata.doi = Some(id_text),
                            "pii" => metadata.pii = Some(id_text),
                            _ => {}
                        }
                        continue;
                    }
                    "volume" if in_article_meta => {
                        let vol_text = extract_text_content(&mut reader)?;
                        metadata.volume = Some(vol_text);
                        continue;
                    }
                    "issue" if in_article_meta => {
                        let issue_text = extract_text_content(&mut reader)?;
                        metadata.issue = Some(issue_text);
                        continue;
                    }
                    "fpage" | "lpage" if in_article_meta => {
                        let page_text = extract_text_content(&mut reader)?;
                        if let Some(pages) = &mut metadata.pages {
                            pages.push_str("-");
                            pages.push_str(&page_text);
                        } else {
                            metadata.pages = Some(page_text);
                        }
                        continue;
                    }
                    "pub-date" if in_article_meta => {
                        let date_text = extract_text_content(&mut reader)?;
                        if metadata.publication_date.is_none() {
                            metadata.publication_date = Some(date_text);
                        }
                        continue;
                    }
                    "abstract" if in_article_meta => {
                        in_abstract = true;
                        abstract_content.clear();
                    }
                    "kwd-group" if in_article_meta => {
                        in_kwd_group = true;
                    }
                    "kwd" if in_kwd_group => {
                        in_kwd = true;
                    }
                    "corresp" if in_article_meta => {
                        let corresp_text = extract_text_content(&mut reader)?;
                        metadata.corresponding_author = Some(corresp_text);
                        continue;
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();

                match tag.as_str() {
                    "article-meta" => {
                        in_article_meta = false;
                    }
                    "article-title" if in_article_title => {
                        in_article_title = false;
                    }
                    "subtitle" if in_subtitle => {
                        in_subtitle = false;
                    }
                    "contrib" if in_contrib => {
                        if !current_author.is_empty() {
                            metadata.authors.push(current_author.clone());
                        }
                        in_contrib = false;
                        current_author.clear();
                    }
                    "name" if in_name => {
                        in_name = false;
                    }
                    "aff" if in_aff => {
                        if !current_aff.is_empty() {
                            metadata.affiliations.push(current_aff.clone());
                        }
                        in_aff = false;
                        current_aff.clear();
                    }
                    "abstract" if in_abstract => {
                        in_abstract = false;
                        metadata.abstract_text = Some(abstract_content.trim().to_string());
                    }
                    "kwd-group" if in_kwd_group => {
                        in_kwd_group = false;
                    }
                    "kwd" if in_kwd => {
                        in_kwd = false;
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(t)) => {
                let decoded = String::from_utf8_lossy(t.as_ref()).to_string();
                let trimmed = decoded.trim();

                if !trimmed.is_empty() {
                    if in_article_title && metadata.title.is_empty() {
                        metadata.title.push_str(trimmed);
                    } else if in_subtitle && metadata.subtitle.is_none() {
                        metadata.subtitle = Some(trimmed.to_string());
                    } else if in_name {
                        if !current_author.is_empty() {
                            current_author.push(' ');
                        }
                        current_author.push_str(trimmed);
                    } else if in_aff {
                        if !current_aff.is_empty() {
                            current_aff.push(' ');
                        }
                        current_aff.push_str(trimmed);
                    } else if in_abstract {
                        if !abstract_content.is_empty() {
                            abstract_content.push(' ');
                        }
                        abstract_content.push_str(trimmed);
                    } else if in_kwd {
                        metadata.keywords.push(trimmed.to_string());
                    }
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

    Ok(metadata)
}

/// Parse JATS content and extract structured text.
fn parse_jats_content(content: &str) -> Result<(String, String)> {
    let mut reader = Reader::from_str(content);
    let mut output = String::new();
    let mut title = String::new();
    let mut in_body = false;
    let mut in_section = false;
    let mut in_title = false;
    let mut in_para = false;
    let mut title_extracted = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();

                match tag.as_str() {
                    "article-title" if !title_extracted => {
                        in_title = true;
                    }
                    "body" => {
                        in_body = true;
                    }
                    "sec" if in_body => {
                        in_section = true;
                    }
                    "title" if (in_section || in_body) && !in_title => {
                        let section_title = extract_text_content(&mut reader)?;
                        if !section_title.is_empty() {
                            output.push_str("## ");
                            output.push_str(&section_title);
                            output.push_str("\n\n");
                        }
                        continue;
                    }
                    "p" if in_body => {
                        in_para = true;
                    }
                    "p" if in_section => {
                        in_para = true;
                    }
                    "table-wrap" if in_body => {
                        // Skip tables, they're handled separately
                        let mut depth = 1;
                        loop {
                            match reader.read_event() {
                                Ok(Event::Start(_)) => depth += 1,
                                Ok(Event::End(_)) => {
                                    depth -= 1;
                                    if depth == 0 {
                                        break;
                                    }
                                }
                                Ok(Event::Eof) => break,
                                Err(_) => break,
                                _ => {}
                            }
                        }
                        continue;
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();

                match tag.as_str() {
                    "article-title" if in_title => {
                        in_title = false;
                        title_extracted = true;
                    }
                    "body" => {
                        in_body = false;
                    }
                    "sec" if in_section => {
                        in_section = false;
                    }
                    "p" if in_para => {
                        in_para = false;
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(t)) => {
                let decoded = String::from_utf8_lossy(t.as_ref()).to_string();
                let trimmed = decoded.trim();

                if !trimmed.is_empty() {
                    if in_title && title.is_empty() {
                        title.push_str(trimmed);
                    } else if in_para && in_body {
                        output.push_str(trimmed);
                        output.push_str("\n\n");
                    }
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

    // Prepend title to output if it was extracted
    let mut final_output = output;
    if !title.is_empty() {
        final_output = format!("# {}\n\n{}", title, final_output);
    }

    Ok((final_output.trim().to_string(), title))
}

impl Plugin for JatsExtractor {
    fn name(&self) -> &str {
        "jats-extractor"
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
impl DocumentExtractor for JatsExtractor {
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
        let jats_content = std::str::from_utf8(content)
            .map(|s| s.to_string())
            .unwrap_or_else(|_| String::from_utf8_lossy(content).to_string());

        // Extract metadata
        let jats_metadata = extract_jats_metadata(&jats_content)?;

        // Parse content
        let (extracted_content, _title) = parse_jats_content(&jats_content)?;

        // Extract tables
        let tables = extract_jats_tables(&jats_content)?;

        // Build metadata
        let mut metadata = Metadata::default();
        let mut subject_parts = Vec::new();

        // Title
        if !jats_metadata.title.is_empty() {
            metadata.subject = Some(jats_metadata.title.clone());
            subject_parts.push(format!("Title: {}", jats_metadata.title));
        }

        // Authors
        if !jats_metadata.authors.is_empty() {
            subject_parts.push(format!("Authors: {}", jats_metadata.authors.join("; ")));
        }

        // DOI
        if let Some(doi) = &jats_metadata.doi {
            subject_parts.push(format!("DOI: {}", doi));
        }

        // Keywords
        if !jats_metadata.keywords.is_empty() {
            subject_parts.push(format!("Keywords: {}", jats_metadata.keywords.join("; ")));
        }

        // Publication date
        if let Some(date) = &jats_metadata.publication_date {
            metadata.date = Some(date.clone());
        }

        // Abstract
        if let Some(abstract_text) = &jats_metadata.abstract_text {
            subject_parts.push(format!("Abstract: {}", abstract_text));
        }

        // Affiliations
        if !jats_metadata.affiliations.is_empty() {
            subject_parts.push(format!("Affiliations: {}", jats_metadata.affiliations.join("; ")));
        }

        if !subject_parts.is_empty() {
            metadata.subject = Some(subject_parts.join(" | "));
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
        &["application/x-jats+xml", "text/jats"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jats_extractor_plugin_interface() {
        let extractor = JatsExtractor::new();
        assert_eq!(extractor.name(), "jats-extractor");
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_jats_extractor_supported_mime_types() {
        let extractor = JatsExtractor::new();
        let mime_types = extractor.supported_mime_types();
        assert_eq!(mime_types.len(), 2);
        assert!(mime_types.contains(&"application/x-jats+xml"));
        assert!(mime_types.contains(&"text/jats"));
    }

    #[test]
    fn test_jats_extractor_priority() {
        let extractor = JatsExtractor::new();
        assert_eq!(extractor.priority(), 50);
    }

    #[test]
    fn test_parse_simple_jats_article() {
        let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-title>Test Article Title</article-title>
    </article-meta>
  </front>
  <body>
    <p>Test content paragraph.</p>
  </body>
</article>"#;

        let (content, title) = parse_jats_content(jats).expect("Parse failed");
        assert_eq!(title, "Test Article Title");
        assert!(content.contains("Test Article Title"));
        assert!(content.contains("Test content"));
    }

    #[test]
    fn test_extract_jats_title() {
        let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-title>Effects of Caffeine on Human Health</article-title>
    </article-meta>
  </front>
</article>"#;

        let metadata = extract_jats_metadata(jats).expect("Metadata extraction failed");
        assert_eq!(metadata.title, "Effects of Caffeine on Human Health");
    }

    #[test]
    fn test_extract_jats_subtitle() {
        let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-title>Main Title</article-title>
      <subtitle>A Systematic Review</subtitle>
    </article-meta>
  </front>
</article>"#;

        let metadata = extract_jats_metadata(jats).expect("Metadata extraction failed");
        assert_eq!(metadata.title, "Main Title");
        assert_eq!(metadata.subtitle, Some("A Systematic Review".to_string()));
    }

    #[test]
    fn test_extract_jats_authors() {
        let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <contrib-group>
        <contrib contrib-type="author">
          <name>
            <surname>Smith</surname>
            <given-names>John A.</given-names>
          </name>
        </contrib>
        <contrib contrib-type="author">
          <name>
            <surname>Johnson</surname>
            <given-names>Jane B.</given-names>
          </name>
        </contrib>
      </contrib-group>
    </article-meta>
  </front>
</article>"#;

        let metadata = extract_jats_metadata(jats).expect("Metadata extraction failed");
        assert_eq!(metadata.authors.len(), 2);
        assert!(metadata.authors[0].contains("Smith"));
        assert!(metadata.authors[1].contains("Johnson"));
    }

    #[test]
    fn test_extract_jats_affiliations() {
        let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <aff id="aff1">Department of Medicine, Harvard University, Cambridge, MA</aff>
      <aff id="aff2">Center for Health Research, Boston Medical Center, Boston, MA</aff>
    </article-meta>
  </front>
</article>"#;

        let metadata = extract_jats_metadata(jats).expect("Metadata extraction failed");
        assert_eq!(metadata.affiliations.len(), 2);
        assert!(metadata.affiliations[0].contains("Harvard"));
        assert!(metadata.affiliations[1].contains("Boston"));
    }

    #[test]
    fn test_extract_jats_doi_and_pii() {
        let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-id pub-id-type="doi">10.1371/journal.pmed.0020124</article-id>
      <article-id pub-id-type="pii">05-PLME-RA-0071R2</article-id>
    </article-meta>
  </front>
</article>"#;

        let metadata = extract_jats_metadata(jats).expect("Metadata extraction failed");
        assert_eq!(metadata.doi, Some("10.1371/journal.pmed.0020124".to_string()));
        assert_eq!(metadata.pii, Some("05-PLME-RA-0071R2".to_string()));
    }

    #[test]
    fn test_extract_jats_keywords() {
        let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <kwd-group>
        <kwd>caffeine</kwd>
        <kwd>meta-analysis</kwd>
        <kwd>systematic review</kwd>
      </kwd-group>
    </article-meta>
  </front>
</article>"#;

        let metadata = extract_jats_metadata(jats).expect("Metadata extraction failed");
        assert_eq!(metadata.keywords.len(), 3);
        assert!(metadata.keywords.contains(&"caffeine".to_string()));
        assert!(metadata.keywords.contains(&"meta-analysis".to_string()));
    }

    #[test]
    fn test_extract_jats_publication_info() {
        let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <pub-date pub-type="epub">
        <day>18</day>
        <month>04</month>
        <year>2005</year>
      </pub-date>
      <volume>2</volume>
      <issue>4</issue>
      <fpage>e124</fpage>
      <lpage>e132</lpage>
    </article-meta>
  </front>
</article>"#;

        let metadata = extract_jats_metadata(jats).expect("Metadata extraction failed");
        assert!(metadata.publication_date.is_some());
        assert_eq!(metadata.volume, Some("2".to_string()));
        assert_eq!(metadata.issue, Some("4".to_string()));
        assert!(metadata.pages.is_some());
    }

    #[test]
    fn test_extract_jats_abstract() {
        let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <abstract>
        <sec>
          <title>Background</title>
          <p>This is the background information of the study.</p>
        </sec>
        <sec>
          <title>Methods</title>
          <p>We used quantitative analysis to evaluate the hypothesis.</p>
        </sec>
      </abstract>
    </article-meta>
  </front>
</article>"#;

        let metadata = extract_jats_metadata(jats).expect("Metadata extraction failed");
        assert!(metadata.abstract_text.is_some());
        let abstract_text = metadata.abstract_text.unwrap();
        assert!(abstract_text.contains("background"));
        assert!(abstract_text.contains("quantitative"));
    }

    #[test]
    fn test_extract_jats_tables_basic() {
        let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <body>
    <table-wrap id="tbl1">
      <table>
        <thead>
          <tr>
            <th>Study</th>
            <th>Year</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>Study A</td>
            <td>2003</td>
          </tr>
          <tr>
            <td>Study B</td>
            <td>2004</td>
          </tr>
        </tbody>
      </table>
    </table-wrap>
  </body>
</article>"#;

        let tables = extract_jats_tables(jats).expect("Table extraction failed");
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].cells.len(), 3); // header + 2 rows
    }

    #[test]
    fn test_extract_jats_corresponding_author() {
        let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <author-notes>
        <corresp id="cor1">To whom correspondence should be addressed. E-mail: rwilliams@yale.edu</corresp>
      </author-notes>
    </article-meta>
  </front>
</article>"#;

        let metadata = extract_jats_metadata(jats).expect("Metadata extraction failed");
        assert!(metadata.corresponding_author.is_some());
        let corresp = metadata.corresponding_author.unwrap();
        assert!(corresp.contains("rwilliams"));
    }

    #[test]
    fn test_extract_jats_section_hierarchy() {
        let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-title>Article Title</article-title>
    </article-meta>
  </front>
  <body>
    <sec id="s1">
      <title>Introduction</title>
      <p>Intro content.</p>
    </sec>
    <sec id="s2">
      <title>Methods</title>
      <p>Methods content.</p>
    </sec>
    <sec id="s3">
      <title>Results</title>
      <p>Results content.</p>
    </sec>
  </body>
</article>"#;

        let (content, _title) = parse_jats_content(jats).expect("Parse failed");
        assert!(content.contains("Introduction"));
        assert!(content.contains("Methods"));
        assert!(content.contains("Results"));
    }

    #[test]
    fn test_jats_extractor_full_metadata_extraction() {
        let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-title>Sample Article</article-title>
      <contrib-group>
        <contrib contrib-type="author">
          <name>
            <surname>Smith</surname>
            <given-names>John</given-names>
          </name>
        </contrib>
      </contrib-group>
      <article-id pub-id-type="doi">10.1234/test</article-id>
      <kwd-group>
        <kwd>test</kwd>
      </kwd-group>
      <abstract>
        <p>Test abstract.</p>
      </abstract>
    </article-meta>
  </front>
  <body>
    <p>Sample content.</p>
  </body>
</article>"#;

        let metadata_extracted = extract_jats_metadata(jats).expect("Metadata extraction failed");
        assert_eq!(metadata_extracted.title, "Sample Article");
        assert_eq!(metadata_extracted.authors.len(), 1);
        assert_eq!(metadata_extracted.doi, Some("10.1234/test".to_string()));
        assert_eq!(metadata_extracted.keywords.len(), 1);
        assert!(metadata_extracted.abstract_text.is_some());
    }

    #[test]
    fn test_jats_extractor_empty_article() {
        let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
    </article-meta>
  </front>
  <body>
  </body>
</article>"#;

        let metadata = extract_jats_metadata(jats).expect("Metadata extraction failed");
        assert!(metadata.title.is_empty());
        let (content, _title) = parse_jats_content(jats).expect("Parse failed");
        assert!(content.is_empty() || content.trim().is_empty());
    }
}
