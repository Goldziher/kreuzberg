//! Native EPUB extractor using permissive-licensed dependencies.
//!
//! This extractor provides native Rust-based EPUB extraction without GPL-licensed
//! dependencies, extracting:
//! - Metadata from OPF (Open Packaging Format) using Dublin Core standards
//! - Content from XHTML files in spine order
//! - Proper handling of EPUB2 and EPUB3 formats
//!
//! Uses only permissive-licensed crates:
//! - `zip` (MIT/Apache) - for reading EPUB container
//! - `roxmltree` (MIT) - for parsing XML
//! - `html-to-markdown-rs` (MIT) - for converting XHTML to plain text

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata};
use async_trait::async_trait;
use roxmltree;
use std::collections::BTreeMap;
use std::io::Cursor;
use zip::ZipArchive;

/// EPUB format extractor using permissive-licensed dependencies.
///
/// Extracts content and metadata from EPUB files (both EPUB2 and EPUB3)
/// using native Rust parsing without GPL-licensed dependencies.
pub struct EpubExtractor;

impl EpubExtractor {
    /// Create a new EPUB extractor.
    pub fn new() -> Self {
        Self
    }

    /// Extract text content from an EPUB document by reading in spine order
    fn extract_content(
        archive: &mut ZipArchive<Cursor<Vec<u8>>>,
        opf_path: &str,
        manifest_dir: &str,
    ) -> Result<String> {
        let opf_xml = Self::read_file_from_zip(archive, opf_path)?;
        let (_, spine_hrefs) = Self::parse_opf(&opf_xml)?;

        let mut content = String::new();

        for (index, href) in spine_hrefs.iter().enumerate() {
            let file_path = Self::resolve_path(manifest_dir, href);

            match Self::read_file_from_zip(archive, &file_path) {
                Ok(xhtml_content) => {
                    let text = Self::extract_text_from_xhtml(&xhtml_content);
                    if !text.is_empty() {
                        if index > 0 && !content.ends_with('\n') {
                            content.push('\n');
                        }
                        content.push_str(&text);
                        content.push('\n');
                    }
                }
                Err(_) => {
                    continue;
                }
            }
        }

        Ok(content.trim().to_string())
    }

    /// Extract text from XHTML content using html-to-markdown-rs
    fn extract_text_from_xhtml(xhtml: &str) -> String {
        match crate::extraction::html::convert_html_to_markdown(xhtml, None) {
            Ok(markdown) => {
                let text = Self::markdown_to_plain_text(&markdown);
                Self::remove_html_comments(&text)
            }
            Err(_) => Self::strip_html_tags(xhtml),
        }
    }

    /// Remove HTML comments from text
    fn remove_html_comments(text: &str) -> String {
        let mut result = String::new();
        let mut in_comment = false;
        let mut chars = text.chars().peekable();

        while let Some(ch) = chars.next() {
            if !in_comment && ch == '<' {
                if chars.peek() == Some(&'!') {
                    chars.next();
                    if chars.peek() == Some(&'-') {
                        chars.next();
                        if chars.peek() == Some(&'-') {
                            chars.next();
                            in_comment = true;
                            continue;
                        } else {
                            result.push('<');
                            result.push('!');
                            result.push('-');
                            continue;
                        }
                    } else {
                        result.push('<');
                        result.push('!');
                        continue;
                    }
                } else {
                    result.push(ch);
                }
            } else if in_comment {
                if ch == '-' && chars.peek() == Some(&'-') {
                    chars.next();
                    if chars.peek() == Some(&'>') {
                        chars.next();
                        in_comment = false;
                        result.push('\n');
                    }
                }
            } else {
                result.push(ch);
            }
        }

        result
    }

    /// Convert markdown output to plain text by removing markdown syntax
    fn markdown_to_plain_text(markdown: &str) -> String {
        let mut text = String::new();
        let mut in_code_block = false;

        for line in markdown.lines() {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push('\n');
                }
                continue;
            }

            if trimmed.starts_with("```") {
                in_code_block = !in_code_block;
                continue;
            }

            if in_code_block {
                text.push_str(trimmed);
                text.push('\n');
                continue;
            }

            let cleaned = if let Some(stripped) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
                stripped
            } else if let Some(stripped) = trimmed.strip_prefix(|c: char| c.is_ascii_digit()) {
                if let Some(rest) = stripped.strip_prefix(". ") {
                    rest
                } else {
                    trimmed
                }
            } else {
                trimmed
            };

            let cleaned = cleaned.trim_start_matches('#').trim();

            let cleaned = cleaned
                .replace("**", "")
                .replace("__", "")
                .replace("*", "")
                .replace("_", "");

            let cleaned = Self::remove_markdown_links(&cleaned);

            if !cleaned.is_empty() {
                text.push_str(&cleaned);
                text.push('\n');
            }
        }

        text.trim().to_string()
    }

    /// Remove markdown links [text](url) -> text
    fn remove_markdown_links(text: &str) -> String {
        let mut result = String::new();
        let mut chars = text.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '[' {
                let mut link_text = String::new();
                let mut depth = 1;

                while let Some(&next_ch) = chars.peek() {
                    chars.next();
                    if next_ch == '[' {
                        depth += 1;
                        link_text.push(next_ch);
                    } else if next_ch == ']' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                        link_text.push(next_ch);
                    } else {
                        link_text.push(next_ch);
                    }
                }

                if let Some(&'(') = chars.peek() {
                    chars.next();
                    let mut paren_depth = 1;
                    while let Some(&next_ch) = chars.peek() {
                        chars.next();
                        if next_ch == '(' {
                            paren_depth += 1;
                        } else if next_ch == ')' {
                            paren_depth -= 1;
                            if paren_depth == 0 {
                                break;
                            }
                        }
                    }
                }

                result.push_str(&link_text);
            } else {
                result.push(ch);
            }
        }

        result
    }

    /// Fallback: strip HTML tags without using specialized libraries
    fn strip_html_tags(html: &str) -> String {
        let mut text = String::new();
        let mut in_tag = false;
        let mut in_script_style = false;
        let mut tag_name = String::new();

        for ch in html.chars() {
            if ch == '<' {
                in_tag = true;
                tag_name.clear();
                continue;
            }

            if ch == '>' {
                in_tag = false;

                let tag_lower = tag_name.to_lowercase();
                if tag_lower.contains("script") || tag_lower.contains("style") {
                    in_script_style = !tag_name.starts_with('/');
                }
                continue;
            }

            if in_tag {
                tag_name.push(ch);
                continue;
            }

            if in_script_style {
                continue;
            }

            if ch == '\n' || ch == '\r' || ch == '\t' || ch == ' ' {
                if !text.is_empty() && !text.ends_with(' ') {
                    text.push(' ');
                }
            } else {
                text.push(ch);
            }
        }

        let mut result = String::new();
        let mut prev_space = false;
        for ch in text.chars() {
            if ch == ' ' {
                if !prev_space {
                    result.push(ch);
                }
                prev_space = true;
            } else {
                result.push(ch);
                prev_space = false;
            }
        }

        result.trim().to_string()
    }

    /// Extract metadata from EPUB OPF file
    fn extract_metadata(opf_xml: &str) -> Result<BTreeMap<String, serde_json::Value>> {
        let mut metadata = BTreeMap::new();

        let (epub_metadata, _) = Self::parse_opf(opf_xml)?;

        if let Some(title) = epub_metadata.title {
            metadata.insert("title".to_string(), serde_json::json!(title));
        }

        if let Some(creator) = epub_metadata.creator {
            metadata.insert("creator".to_string(), serde_json::json!(creator.clone()));
            metadata.insert("authors".to_string(), serde_json::json!(vec![creator]));
        }

        if let Some(date) = epub_metadata.date {
            metadata.insert("date".to_string(), serde_json::json!(date));
        }

        if let Some(language) = epub_metadata.language {
            metadata.insert("language".to_string(), serde_json::json!(language));
        }

        if let Some(identifier) = epub_metadata.identifier {
            metadata.insert("identifier".to_string(), serde_json::json!(identifier));
        }

        if let Some(publisher) = epub_metadata.publisher {
            metadata.insert("publisher".to_string(), serde_json::json!(publisher));
        }

        if let Some(subject) = epub_metadata.subject {
            metadata.insert("subject".to_string(), serde_json::json!(subject));
        }

        if let Some(description) = epub_metadata.description {
            metadata.insert("description".to_string(), serde_json::json!(description));
        }

        if let Some(rights) = epub_metadata.rights {
            metadata.insert("rights".to_string(), serde_json::json!(rights));
        }

        Ok(metadata)
    }

    /// Parse container.xml to find the OPF file path
    fn parse_container_xml(xml: &str) -> Result<String> {
        match roxmltree::Document::parse(xml) {
            Ok(doc) => {
                for node in doc.descendants() {
                    if node.tag_name().name() == "rootfile"
                        && let Some(full_path) = node.attribute("full-path")
                    {
                        return Ok(full_path.to_string());
                    }
                }
                Err(crate::KreuzbergError::Parsing {
                    message: "No rootfile found in container.xml".to_string(),
                    source: None,
                })
            }
            Err(e) => Err(crate::KreuzbergError::Parsing {
                message: format!("Failed to parse container.xml: {}", e),
                source: None,
            }),
        }
    }

    /// Parse OPF file and extract metadata and spine order
    fn parse_opf(xml: &str) -> Result<(OepbMetadata, Vec<String>)> {
        match roxmltree::Document::parse(xml) {
            Ok(doc) => {
                let root = doc.root();

                let mut metadata = OepbMetadata::default();
                let mut manifest: BTreeMap<String, String> = BTreeMap::new();
                let mut spine_order: Vec<String> = Vec::new();

                for node in root.descendants() {
                    match node.tag_name().name() {
                        "title" => {
                            if let Some(text) = node.text() {
                                metadata.title = Some(text.trim().to_string());
                            }
                        }
                        "creator" => {
                            if let Some(text) = node.text() {
                                metadata.creator = Some(text.trim().to_string());
                            }
                        }
                        "date" => {
                            if let Some(text) = node.text() {
                                metadata.date = Some(text.trim().to_string());
                            }
                        }
                        "language" => {
                            if let Some(text) = node.text() {
                                metadata.language = Some(text.trim().to_string());
                            }
                        }
                        "identifier" => {
                            if let Some(text) = node.text() {
                                metadata.identifier = Some(text.trim().to_string());
                            }
                        }
                        "publisher" => {
                            if let Some(text) = node.text() {
                                metadata.publisher = Some(text.trim().to_string());
                            }
                        }
                        "subject" => {
                            if let Some(text) = node.text() {
                                metadata.subject = Some(text.trim().to_string());
                            }
                        }
                        "description" => {
                            if let Some(text) = node.text() {
                                metadata.description = Some(text.trim().to_string());
                            }
                        }
                        "rights" => {
                            if let Some(text) = node.text() {
                                metadata.rights = Some(text.trim().to_string());
                            }
                        }
                        "item" => {
                            if let Some(id) = node.attribute("id")
                                && let Some(href) = node.attribute("href")
                            {
                                manifest.insert(id.to_string(), href.to_string());
                            }
                        }
                        _ => {}
                    }
                }

                for node in root.descendants() {
                    if node.tag_name().name() == "itemref"
                        && let Some(idref) = node.attribute("idref")
                        && let Some(href) = manifest.get(idref)
                    {
                        spine_order.push(href.clone());
                    }
                }

                Ok((metadata, spine_order))
            }
            Err(e) => Err(crate::KreuzbergError::Parsing {
                message: format!("Failed to parse OPF file: {}", e),
                source: None,
            }),
        }
    }

    /// Read a file from the ZIP archive
    fn read_file_from_zip(archive: &mut ZipArchive<Cursor<Vec<u8>>>, path: &str) -> Result<String> {
        match archive.by_name(path) {
            Ok(mut file) => {
                let mut content = String::new();
                match std::io::Read::read_to_string(&mut file, &mut content) {
                    Ok(_) => Ok(content),
                    Err(e) => Err(crate::KreuzbergError::Parsing {
                        message: format!("Failed to read file from EPUB: {}", e),
                        source: None,
                    }),
                }
            }
            Err(e) => Err(crate::KreuzbergError::Parsing {
                message: format!("File not found in EPUB: {} ({})", path, e),
                source: None,
            }),
        }
    }

    /// Resolve a relative path within the manifest directory
    fn resolve_path(base_dir: &str, relative_path: &str) -> String {
        if relative_path.starts_with('/') {
            relative_path.trim_start_matches('/').to_string()
        } else if base_dir.is_empty() || base_dir == "." {
            relative_path.to_string()
        } else {
            format!("{}/{}", base_dir.trim_end_matches('/'), relative_path)
        }
    }
}

/// Metadata extracted from OPF (Open Packaging Format) file
#[derive(Debug, Default, Clone)]
struct OepbMetadata {
    title: Option<String>,
    creator: Option<String>,
    date: Option<String>,
    language: Option<String>,
    identifier: Option<String>,
    publisher: Option<String>,
    subject: Option<String>,
    description: Option<String>,
    rights: Option<String>,
}

impl Default for EpubExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for EpubExtractor {
    fn name(&self) -> &str {
        "epub-extractor"
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
        "Extracts content and metadata from EPUB documents (native Rust implementation with permissive licenses)"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[cfg(feature = "office")]
#[async_trait]
impl DocumentExtractor for EpubExtractor {
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
        let cursor = Cursor::new(content.to_vec());

        let mut archive = ZipArchive::new(cursor).map_err(|e| crate::KreuzbergError::Parsing {
            message: format!("Failed to open EPUB as ZIP: {}", e),
            source: None,
        })?;

        let container_xml = Self::read_file_from_zip(&mut archive, "META-INF/container.xml")?;
        let opf_path = Self::parse_container_xml(&container_xml)?;

        let manifest_dir = if let Some(last_slash) = opf_path.rfind('/') {
            opf_path[..last_slash].to_string()
        } else {
            String::new()
        };

        let opf_xml = Self::read_file_from_zip(&mut archive, &opf_path)?;

        let extracted_content = Self::extract_content(&mut archive, &opf_path, &manifest_dir)?;

        let metadata_btree = Self::extract_metadata(&opf_xml)?;
        let metadata_map: std::collections::HashMap<String, serde_json::Value> = metadata_btree.into_iter().collect();

        Ok(ExtractionResult {
            content: extracted_content,
            mime_type: mime_type.to_string(),
            metadata: Metadata {
                additional: metadata_map,
                ..Default::default()
            },
            pages: None,
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[
            "application/epub+zip",
            "application/x-epub+zip",
            "application/vnd.epub+zip",
        ]
    }

    fn priority(&self) -> i32 {
        60
    }
}

#[cfg(all(test, feature = "office"))]
mod tests {
    use super::*;

    #[test]
    fn test_epub_extractor_plugin_interface() {
        let extractor = EpubExtractor::new();
        assert_eq!(extractor.name(), "epub-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 60);
        assert!(!extractor.supported_mime_types().is_empty());
    }

    #[test]
    fn test_epub_extractor_default() {
        let extractor = EpubExtractor;
        assert_eq!(extractor.name(), "epub-extractor");
    }

    #[tokio::test]
    async fn test_epub_extractor_initialize_shutdown() {
        let extractor = EpubExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_strip_html_tags_simple() {
        let html = "<html><body><p>Hello World</p></body></html>";
        let text = EpubExtractor::strip_html_tags(html);
        assert!(text.contains("Hello World"));
    }

    #[test]
    fn test_strip_html_tags_with_scripts() {
        let html = "<body><p>Text</p><script>alert('bad');</script><p>More</p></body>";
        let text = EpubExtractor::strip_html_tags(html);
        assert!(!text.contains("bad"));
        assert!(text.contains("Text"));
        assert!(text.contains("More"));
    }

    #[test]
    fn test_strip_html_tags_with_styles() {
        let html = "<body><p>Text</p><style>.class { color: red; }</style><p>More</p></body>";
        let text = EpubExtractor::strip_html_tags(html);
        assert!(!text.to_lowercase().contains("color"));
        assert!(text.contains("Text"));
        assert!(text.contains("More"));
    }

    #[test]
    fn test_strip_html_tags_normalizes_whitespace() {
        let html = "<p>Hello   \n\t   World</p>";
        let text = EpubExtractor::strip_html_tags(html);
        assert!(text.contains("Hello") && text.contains("World"));
    }

    #[test]
    fn test_remove_markdown_links() {
        let text = "This is a [link](http://example.com) in text";
        let result = EpubExtractor::remove_markdown_links(text);
        assert!(result.contains("link"));
        assert!(!result.contains("http://"));
    }

    #[test]
    fn test_resolve_path_with_base_dir() {
        let result = EpubExtractor::resolve_path("OEBPS", "chapter.xhtml");
        assert_eq!(result, "OEBPS/chapter.xhtml");
    }

    #[test]
    fn test_resolve_path_absolute() {
        let result = EpubExtractor::resolve_path("OEBPS", "/chapter.xhtml");
        assert_eq!(result, "chapter.xhtml");
    }

    #[test]
    fn test_resolve_path_empty_base() {
        let result = EpubExtractor::resolve_path("", "chapter.xhtml");
        assert_eq!(result, "chapter.xhtml");
    }

    #[test]
    fn test_epub_extractor_supported_mime_types() {
        let extractor = EpubExtractor::new();
        let supported = extractor.supported_mime_types();
        assert!(supported.contains(&"application/epub+zip"));
        assert!(supported.contains(&"application/x-epub+zip"));
        assert!(supported.contains(&"application/vnd.epub+zip"));
    }

    #[test]
    fn test_markdown_to_plain_text_removes_formatting() {
        let markdown = "# Heading\n\nThis is **bold** text with _italic_ emphasis.";
        let result = EpubExtractor::markdown_to_plain_text(markdown);
        assert!(result.contains("Heading"));
        assert!(result.contains("bold"));
        assert!(!result.contains("**"));
    }

    #[test]
    fn test_markdown_to_plain_text_removes_list_markers() {
        let markdown = "- Item 1\n- Item 2\n* Item 3";
        let result = EpubExtractor::markdown_to_plain_text(markdown);
        assert!(result.contains("Item 1"));
        assert!(result.contains("Item 2"));
        assert!(result.contains("Item 3"));
    }
}
