//! Native EPUB extractor using the `epub` crate.
//!
//! This extractor provides native Rust-based EPUB extraction as a replacement
//! for Pandoc, extracting:
//! - Metadata from OPF (Open Packaging Format) using Dublin Core standards
//! - Content from XHTML files in spine order
//! - Cover image detection

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata};
use async_trait::async_trait;
use std::collections::HashMap;
use std::io::Cursor;

#[cfg(feature = "office")]
use epub::doc::EpubDoc;

/// EPUB format extractor.
///
/// Extracts content and metadata from EPUB files (both EPUB2 and EPUB3)
/// using native Rust parsing without external dependencies like Pandoc.
pub struct EpubExtractor;

impl EpubExtractor {
    /// Create a new EPUB extractor.
    pub fn new() -> Self {
        Self
    }

    /// Extract text content from an EPUB document
    #[cfg(feature = "office")]
    fn extract_content<R: std::io::Read + std::io::Seek>(epub: &mut EpubDoc<R>) -> String {
        let mut content = String::new();
        let num_chapters = epub.get_num_chapters();

        // Iterate through all chapters in the EPUB
        for chapter_num in 0..num_chapters {
            // Set current chapter (returns bool, not Result)
            epub.set_current_chapter(chapter_num);

            // Get current chapter content as string (returns Option)
            if let Some((data, _mime)) = epub.get_current_str() {
                // Extract text from XHTML content
                let extracted_text = Self::extract_text_from_xhtml(&data);
                if !extracted_text.is_empty() {
                    content.push_str(&extracted_text);
                    content.push('\n');
                }
            }
        }

        content.trim().to_string()
    }

    /// Extract plain text from XHTML content
    #[cfg(feature = "office")]
    fn extract_text_from_xhtml(html: &str) -> String {
        let mut text = String::new();
        let mut in_tag = false;
        let mut in_script_style = false;
        let mut script_style_tag = String::new();

        let mut chars = html.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '<' {
                in_tag = true;
                script_style_tag.clear();
                continue;
            }

            if ch == '>' {
                in_tag = false;

                // Check for script and style closing tags
                if script_style_tag.to_lowercase().contains("script")
                    || script_style_tag.to_lowercase().contains("style")
                {
                    in_script_style = !script_style_tag.starts_with('/');
                }
                continue;
            }

            if in_tag {
                script_style_tag.push(ch);
                continue;
            }

            if in_script_style {
                continue;
            }

            // Handle HTML entities
            if ch == '&' {
                let mut entity = String::from("&");
                while let Some(&next_ch) = chars.peek() {
                    entity.push(next_ch);
                    chars.next();
                    if next_ch == ';' {
                        break;
                    }
                }

                let decoded = match entity.as_str() {
                    "&nbsp;" => " ",
                    "&lt;" => "<",
                    "&gt;" => ">",
                    "&amp;" => "&",
                    "&quot;" => "\"",
                    "&apos;" => "'",
                    _ => {
                        text.push_str(&entity);
                        continue;
                    }
                };
                text.push_str(decoded);
            } else if ch == '\n' || ch == '\r' || ch == '\t' {
                // Normalize whitespace
                if !text.ends_with(' ') && !text.is_empty() {
                    text.push(' ');
                }
            } else if ch == ' ' {
                // Avoid multiple spaces
                if !text.ends_with(' ') {
                    text.push(' ');
                }
            } else {
                text.push(ch);
            }
        }

        // Clean up multiple spaces
        let mut cleaned = String::new();
        let mut prev_space = false;
        for ch in text.chars() {
            if ch == ' ' {
                if !prev_space {
                    cleaned.push(ch);
                }
                prev_space = true;
            } else {
                cleaned.push(ch);
                prev_space = false;
            }
        }

        cleaned.trim().to_string()
    }

    /// Extract metadata from EPUB document
    #[cfg(feature = "office")]
    fn extract_metadata<R: std::io::Read + std::io::Seek>(epub: &mut EpubDoc<R>) -> HashMap<String, serde_json::Value> {
        let mut metadata = HashMap::new();

        // Extract Dublin Core metadata using mdata() convenience method
        // mdata() returns Option<MetadataItem> where MetadataItem has property and value fields

        // Title
        if let Some(title_item) = epub.mdata("title") {
            metadata.insert("title".to_string(), serde_json::json!(title_item.value));
        }

        // Creator/Author
        if let Some(creator_item) = epub.mdata("creator") {
            let creator_str = creator_item.value.clone();
            metadata.insert("creator".to_string(), serde_json::json!(creator_str.clone()));
            // Also store as authors array (single item)
            metadata.insert("authors".to_string(), serde_json::json!(vec![creator_str]));
        }

        // Date
        if let Some(date_item) = epub.mdata("date") {
            metadata.insert("date".to_string(), serde_json::json!(date_item.value));
        }

        // Language
        if let Some(lang_item) = epub.mdata("language") {
            metadata.insert("language".to_string(), serde_json::json!(lang_item.value));
        }

        // Identifier
        if let Some(id_item) = epub.mdata("identifier") {
            metadata.insert("identifier".to_string(), serde_json::json!(id_item.value));
        }

        // Publisher
        if let Some(pub_item) = epub.mdata("publisher") {
            metadata.insert("publisher".to_string(), serde_json::json!(pub_item.value));
        }

        // Subject/Keywords
        if let Some(subj_item) = epub.mdata("subject") {
            metadata.insert("subject".to_string(), serde_json::json!(subj_item.value));
        }

        // Description
        if let Some(desc_item) = epub.mdata("description") {
            metadata.insert("description".to_string(), serde_json::json!(desc_item.value));
        }

        // Rights
        if let Some(rights_item) = epub.mdata("rights") {
            metadata.insert("rights".to_string(), serde_json::json!(rights_item.value));
        }

        // Release Identifier
        if let Some(release_id) = epub.get_release_identifier() {
            metadata.insert("release_identifier".to_string(), serde_json::json!(release_id));
        }

        metadata
    }

    /// Detect if EPUB has a cover image
    #[cfg(feature = "office")]
    fn detect_cover<R: std::io::Read + std::io::Seek>(epub: &mut EpubDoc<R>) -> Option<String> {
        // Try to get cover ID from metadata
        // get_cover_id() returns Option<String>
        epub.get_cover_id()
    }
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
        "Extracts content and metadata from EPUB documents (native Rust implementation)"
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
        // Create a cursor from the content bytes
        let cursor = Cursor::new(content.to_vec());

        // Open EPUB document from reader
        let mut epub = EpubDoc::from_reader(cursor)
            .map_err(|e| crate::KreuzbergError::Other(format!("Failed to open EPUB: {}", e)))?;

        // Extract content
        let extracted_content = Self::extract_content(&mut epub);

        // Extract metadata
        let metadata_map = Self::extract_metadata(&mut epub);

        // Add cover detection information
        let mut metadata_with_cover = metadata_map.clone();
        if let Some(cover) = Self::detect_cover(&mut epub) {
            metadata_with_cover.insert("cover".to_string(), serde_json::json!(cover));
        }

        Ok(ExtractionResult {
            content: extracted_content,
            mime_type: mime_type.to_string(),
            metadata: Metadata {
                additional: metadata_with_cover,
                ..Default::default()
            },
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
        let extractor = EpubExtractor::default();
        assert_eq!(extractor.name(), "epub-extractor");
    }

    #[tokio::test]
    async fn test_epub_extractor_initialize_shutdown() {
        let extractor = EpubExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_extract_text_from_xhtml_simple() {
        let html = "<html><body><p>Hello World</p></body></html>";
        let text = EpubExtractor::extract_text_from_xhtml(html);
        assert!(text.contains("Hello World"));
    }

    #[test]
    fn test_extract_text_from_xhtml_with_entities() {
        let html = "<p>Hello&nbsp;&amp;&nbsp;World</p>";
        let text = EpubExtractor::extract_text_from_xhtml(html);
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
    }

    #[test]
    fn test_extract_text_from_xhtml_removes_script() {
        let html = "<body><p>Text</p><script>alert('bad');</script><p>More</p></body>";
        let text = EpubExtractor::extract_text_from_xhtml(html);
        assert!(!text.contains("bad"));
        assert!(text.contains("Text"));
        assert!(text.contains("More"));
    }

    #[test]
    fn test_extract_text_from_xhtml_removes_style() {
        let html = "<body><p>Text</p><style>.class { color: red; }</style><p>More</p></body>";
        let text = EpubExtractor::extract_text_from_xhtml(html);
        assert!(!text.to_lowercase().contains("color"));
        assert!(text.contains("Text"));
        assert!(text.contains("More"));
    }

    #[test]
    fn test_extract_text_from_xhtml_normalizes_whitespace() {
        let html = "<p>Hello   \n\t   World</p>";
        let text = EpubExtractor::extract_text_from_xhtml(html);
        // Should have single spaces
        assert!(text.contains("Hello World") || text.contains("Hello  World"));
    }

    #[test]
    fn test_epub_extractor_supported_mime_types() {
        let extractor = EpubExtractor::new();
        let supported = extractor.supported_mime_types();
        assert!(supported.contains(&"application/epub+zip"));
        assert!(supported.contains(&"application/x-epub+zip"));
        assert!(supported.contains(&"application/vnd.epub+zip"));
    }
}
