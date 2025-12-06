//! Native OPML (Outline Processor Markup Language) extractor using the `roxmltree` library.
//!
//! This extractor provides native Rust-based OPML extraction, parsing outline structures
//! commonly used for RSS feed lists, podcast directories, and general outlines.
//!
//! Extracts:
//! - Metadata from `<head>`: title, dateCreated, dateModified, ownerName, ownerEmail
//! - Content from `<body><outline>` hierarchy using text attributes
//! - Outline hierarchy structure preserved in plain text format with indentation
//! - Note: URLs (xmlUrl, htmlUrl) are extracted from attributes but not included in main content
//!   to maintain parity with Pandoc extraction behavior
//!
//! Example OPML structure:
//! ```xml
//! <opml version="2.0">
//!   <head>
//!     <title>My Feeds</title>
//!     <ownerName>John</ownerName>
//!   </head>
//!   <body>
//!     <outline text="Tech" type="folder">
//!       <outline text="Hacker News" type="rss" xmlUrl="https://..." />
//!     </outline>
//!   </body>
//! </opml>
//! ```

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata};
use async_trait::async_trait;
use std::collections::HashMap;

#[cfg(feature = "office")]
use roxmltree::Node;

/// OPML format extractor.
///
/// Extracts outline structure and metadata from OPML documents using native Rust parsing.
pub struct OpmlExtractor;

impl OpmlExtractor {
    /// Create a new OPML extractor.
    pub fn new() -> Self {
        Self
    }

    /// Extract OPML content and structure
    #[cfg(feature = "office")]
    fn extract_content_and_metadata(content: &[u8]) -> Result<(String, HashMap<String, serde_json::Value>)> {
        let doc = roxmltree::Document::parse(
            std::str::from_utf8(content)
                .map_err(|e| crate::KreuzbergError::Other(format!("Invalid UTF-8 in OPML: {}", e)))?,
        )
        .map_err(|e| crate::KreuzbergError::Other(format!("Failed to parse OPML: {}", e)))?;

        let mut extracted_content = String::new();
        let mut metadata = HashMap::new();

        // Find the root opml element
        if let Some(opml) = doc.root().children().find(|n| n.tag_name().name() == "opml") {
            // Extract head metadata
            if let Some(head) = opml.children().find(|n| n.tag_name().name() == "head") {
                for child in head.children().filter(|n| n.is_element()) {
                    let tag = child.tag_name().name();
                    let text = child.text().unwrap_or("").trim();

                    match tag {
                        "title" => {
                            if !text.is_empty() {
                                metadata.insert("title".to_string(), serde_json::json!(text));
                            }
                        }
                        "dateCreated" => {
                            if !text.is_empty() {
                                metadata.insert("dateCreated".to_string(), serde_json::json!(text));
                            }
                        }
                        "dateModified" => {
                            if !text.is_empty() {
                                metadata.insert("dateModified".to_string(), serde_json::json!(text));
                            }
                        }
                        "ownerName" => {
                            if !text.is_empty() {
                                metadata.insert("ownerName".to_string(), serde_json::json!(text));
                            }
                        }
                        "ownerEmail" => {
                            if !text.is_empty() {
                                metadata.insert("ownerEmail".to_string(), serde_json::json!(text));
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Extract body outlines
            if let Some(body) = opml.children().find(|n| n.tag_name().name() == "body") {
                // Add title to content if present
                if let Some(title) = metadata.get("title").and_then(|v| v.as_str()) {
                    extracted_content.push_str(title);
                    extracted_content.push('\n');
                    extracted_content.push('\n');
                }

                // Process outlines recursively
                for outline in body.children().filter(|n| n.tag_name().name() == "outline") {
                    Self::process_outline(outline, 0, &mut extracted_content);
                }
            }
        }

        Ok((extracted_content.trim().to_string(), metadata))
    }

    /// Process outline elements recursively
    ///
    /// Extracts text content from outline hierarchy while preserving nesting depth
    /// through indentation. URL attributes are excluded from the main content
    /// to maintain parity with Pandoc's extraction behavior.
    #[cfg(feature = "office")]
    fn process_outline(node: Node, depth: usize, output: &mut String) {
        // Get text attribute
        let text = node.attribute("text").unwrap_or("").trim();

        // Add indentation and text
        if !text.is_empty() {
            let indent = "  ".repeat(depth);
            output.push_str(&indent);
            output.push_str(text);
            output.push('\n');
        }

        // Process child outlines
        for child in node.children().filter(|n| n.tag_name().name() == "outline") {
            Self::process_outline(child, depth + 1, output);
        }
    }
}

impl Default for OpmlExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for OpmlExtractor {
    fn name(&self) -> &str {
        "opml-extractor"
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
        "Extracts content and metadata from OPML (Outline Processor Markup Language) documents"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[cfg(feature = "office")]
#[async_trait]
impl DocumentExtractor for OpmlExtractor {
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
        let (extracted_content, metadata_map) = Self::extract_content_and_metadata(content)?;

        Ok(ExtractionResult {
            content: extracted_content,
            mime_type: mime_type.to_string(),
            metadata: Metadata {
                additional: metadata_map,
                ..Default::default()
            },
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["text/x-opml", "application/xml+opml"]
    }

    fn priority(&self) -> i32 {
        55
    }
}

#[cfg(all(test, feature = "office"))]
mod tests {
    use super::*;

    #[test]
    fn test_opml_extractor_plugin_interface() {
        let extractor = OpmlExtractor::new();
        assert_eq!(extractor.name(), "opml-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 55);
        assert!(!extractor.supported_mime_types().is_empty());
    }

    #[test]
    fn test_opml_extractor_default() {
        let extractor = OpmlExtractor::default();
        assert_eq!(extractor.name(), "opml-extractor");
    }

    #[tokio::test]
    async fn test_opml_extractor_initialize_shutdown() {
        let extractor = OpmlExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_opml_simple_outline_parsing() {
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Test</title>
  </head>
  <body>
    <outline text="Item 1" />
    <outline text="Item 2" />
  </body>
</opml>"#;

        let (content, metadata) = OpmlExtractor::extract_content_and_metadata(opml).expect("Should parse simple OPML");

        assert!(content.contains("Item 1"), "Should extract first item");
        assert!(content.contains("Item 2"), "Should extract second item");
        assert_eq!(
            metadata.get("title").and_then(|v| v.as_str()),
            Some("Test"),
            "Should extract title"
        );
    }

    #[test]
    fn test_opml_nested_hierarchy() {
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Hierarchy Test</title>
  </head>
  <body>
    <outline text="Category">
      <outline text="Subcategory">
        <outline text="Item" />
      </outline>
    </outline>
  </body>
</opml>"#;

        let (content, _) = OpmlExtractor::extract_content_and_metadata(opml).expect("Should parse nested OPML");

        assert!(content.contains("Category"), "Should contain top level");
        assert!(content.contains("Subcategory"), "Should contain nested level");
        assert!(content.contains("Item"), "Should contain deep item");
        // Check for indentation
        assert!(content.contains("  "), "Should have indentation for nested items");
    }

    #[test]
    fn test_opml_rss_feeds() {
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Feeds</title>
  </head>
  <body>
    <outline text="Tech">
      <outline text="Hacker News" type="rss" xmlUrl="https://news.ycombinator.com/rss" htmlUrl="https://news.ycombinator.com/" />
      <outline text="TechCrunch" type="rss" xmlUrl="https://techcrunch.com/feed/" />
    </outline>
  </body>
</opml>"#;

        let (content, _) = OpmlExtractor::extract_content_and_metadata(opml).expect("Should parse RSS OPML");

        assert!(content.contains("Hacker News"), "Should extract feed title");
        // URLs are intentionally excluded from content to maintain parity with Pandoc extraction behavior
        assert!(
            !content.contains("https://"),
            "Should NOT extract feed URLs (text-only extraction)"
        );
        assert!(content.contains("TechCrunch"), "Should extract multiple feeds");
    }

    #[test]
    fn test_opml_metadata_extraction() {
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>My Feeds</title>
    <dateCreated>Mon, 06 Nov 2023 00:00:00 GMT</dateCreated>
    <dateModified>Fri, 01 Dec 2023 12:00:00 GMT</dateModified>
    <ownerName>John Doe</ownerName>
    <ownerEmail>john@example.com</ownerEmail>
  </head>
  <body>
    <outline text="Item" />
  </body>
</opml>"#;

        let (_content, metadata) = OpmlExtractor::extract_content_and_metadata(opml).expect("Should extract metadata");

        assert_eq!(metadata.get("title").and_then(|v| v.as_str()), Some("My Feeds"));
        assert_eq!(metadata.get("ownerName").and_then(|v| v.as_str()), Some("John Doe"));
        assert_eq!(
            metadata.get("ownerEmail").and_then(|v| v.as_str()),
            Some("john@example.com")
        );
        assert!(metadata.contains_key("dateCreated"));
        assert!(metadata.contains_key("dateModified"));
    }

    #[test]
    fn test_opml_supported_mime_types() {
        let extractor = OpmlExtractor::new();
        let supported = extractor.supported_mime_types();
        assert!(supported.contains(&"text/x-opml"));
        assert!(supported.contains(&"application/xml+opml"));
    }

    #[test]
    fn test_opml_with_special_characters() {
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Test &amp; Special</title>
  </head>
  <body>
    <outline text="Business &amp; Startups" />
    <outline text="Science &lt;Advanced&gt;" />
  </body>
</opml>"#;

        let (content, metadata) =
            OpmlExtractor::extract_content_and_metadata(opml).expect("Should handle special characters");

        assert!(
            content.contains("Business") && content.contains("Startups"),
            "Should decode HTML entities"
        );
        // Title metadata should be extracted
        let title = metadata.get("title").and_then(|v| v.as_str()).unwrap_or("");
        assert!(!title.is_empty(), "Should extract title");
    }

    #[test]
    fn test_opml_empty_body() {
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Empty</title>
  </head>
  <body>
  </body>
</opml>"#;

        let (_content, metadata) = OpmlExtractor::extract_content_and_metadata(opml).expect("Should handle empty body");

        assert_eq!(metadata.get("title").and_then(|v| v.as_str()), Some("Empty"));
    }

    #[tokio::test]
    async fn test_opml_extractor_async_extraction() {
        let extractor = OpmlExtractor::new();
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Async Test</title>
  </head>
  <body>
    <outline text="Item" />
  </body>
</opml>"#;

        let result = extractor
            .extract_bytes(opml, "text/x-opml", &ExtractionConfig::default())
            .await
            .expect("Should extract OPML asynchronously");

        assert_eq!(result.mime_type, "text/x-opml");
        assert!(result.content.contains("Item"));
        assert_eq!(
            result.metadata.additional.get("title").and_then(|v| v.as_str()),
            Some("Async Test")
        );
    }

    // ============================================================================
    // EDGE CASE TESTS FOR MALFORMED XML AND BOUNDARY CONDITIONS
    // ============================================================================

    #[test]
    fn test_opml_malformed_missing_closing_tag() {
        // Missing closing </outline> tag - should fail gracefully
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Broken</title>
  </head>
  <body>
    <outline text="Unclosed"
  </body>
</opml>"#;

        let result = OpmlExtractor::extract_content_and_metadata(opml);
        // Should fail to parse - XML is malformed
        assert!(result.is_err(), "Should fail to parse OPML with missing closing tags");
    }

    #[test]
    fn test_opml_malformed_invalid_nesting() {
        // Invalid nesting: closing head before outline
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Invalid Nesting</title>
  <body>
    <outline text="Item" />
  </body>
</opml>"#;

        let result = OpmlExtractor::extract_content_and_metadata(opml);
        // Should fail to parse - XML structure is invalid
        assert!(result.is_err(), "Should fail to parse OPML with invalid nesting");
    }

    #[test]
    fn test_opml_empty_outline_elements() {
        // Outline elements with empty text attributes
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Empty Outlines</title>
  </head>
  <body>
    <outline text="" />
    <outline />
    <outline text="Valid Item">
      <outline text="" />
      <outline text="Another Valid" />
    </outline>
  </body>
</opml>"#;

        let (content, metadata) =
            OpmlExtractor::extract_content_and_metadata(opml).expect("Should handle empty outline elements");

        // Should extract only non-empty text content
        assert!(content.contains("Valid Item"), "Should extract valid items");
        assert!(content.contains("Another Valid"), "Should extract nested valid items");
        // Empty outlines should be skipped
        let empty_count = content.matches("\n\n").count();
        assert!(empty_count < 3, "Should skip empty outline elements");

        // Metadata should still be extracted
        assert_eq!(metadata.get("title").and_then(|v| v.as_str()), Some("Empty Outlines"));
    }

    #[test]
    fn test_opml_deeply_nested_empty_nodes() {
        // Deep nesting with many empty nodes
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Deep Nesting</title>
  </head>
  <body>
    <outline text="Level 1">
      <outline text="">
        <outline text="">
          <outline text="Deep Item">
            <outline text="" />
          </outline>
        </outline>
      </outline>
      <outline text="Level 1 Sibling" />
    </outline>
  </body>
</opml>"#;

        let (content, _) =
            OpmlExtractor::extract_content_and_metadata(opml).expect("Should handle deeply nested structures");

        assert!(content.contains("Level 1"), "Should extract top-level item");
        assert!(content.contains("Deep Item"), "Should extract deeply nested item");
        assert!(content.contains("Level 1 Sibling"), "Should extract sibling items");
    }

    #[test]
    fn test_opml_outline_with_missing_text_attribute() {
        // Outline elements missing text attribute
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Missing Attributes</title>
  </head>
  <body>
    <outline type="folder" />
    <outline text="Valid Item" type="rss" />
    <outline type="rss" xmlUrl="https://example.com/feed" />
  </body>
</opml>"#;

        let (content, metadata) = OpmlExtractor::extract_content_and_metadata(opml)
            .expect("Should handle outline with missing text attribute");

        // Only the outline with text attribute should appear
        assert!(content.contains("Valid Item"), "Should extract item with text");
        assert!(!content.contains("https://"), "Should not extract URLs");

        // Metadata should still be available
        assert_eq!(
            metadata.get("title").and_then(|v| v.as_str()),
            Some("Missing Attributes")
        );
    }

    #[test]
    fn test_opml_whitespace_only_text_attribute() {
        // Outline with whitespace-only text attribute
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Whitespace Test</title>
  </head>
  <body>
    <outline text="   " />
    <outline text="
" />
    <outline text="Real Content" />
  </body>
</opml>"#;

        let (content, _) =
            OpmlExtractor::extract_content_and_metadata(opml).expect("Should handle whitespace-only text");

        // Only content with non-whitespace should appear
        assert!(
            content.contains("Real Content"),
            "Should extract non-whitespace content"
        );
        // The content should contain the title and real content but skip empty outlines
        let trimmed = content.trim();
        assert!(trimmed.contains("Whitespace Test"), "Should have title");
        assert!(trimmed.contains("Real Content"), "Should have real content");
    }

    #[test]
    fn test_opml_html_entity_in_nested_structure() {
        // HTML entities in nested outline structure
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Entities &amp; Nesting</title>
  </head>
  <body>
    <outline text="News &amp; Updates">
      <outline text="Tech &lt; Science" />
      <outline text="Health &gt; Wealth" />
    </outline>
  </body>
</opml>"#;

        let (content, metadata) =
            OpmlExtractor::extract_content_and_metadata(opml).expect("Should handle HTML entities");

        assert!(
            content.contains("News") && content.contains("Updates"),
            "Should decode &amp; entity"
        );
        assert!(content.contains("Tech"), "Should handle &lt; entity");
        assert!(content.contains("Science"), "Should decode entity properly");

        // Title should also be decoded
        let title = metadata.get("title").and_then(|v| v.as_str()).unwrap_or("");
        assert!(
            title.contains("&") && title.contains("Nesting"),
            "Title should have decoded entity"
        );
    }

    #[test]
    fn test_opml_single_outline_no_children() {
        // Simple case: single outline with no children
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>Single</title>
  </head>
  <body>
    <outline text="Only Item" />
  </body>
</opml>"#;

        let (content, metadata) =
            OpmlExtractor::extract_content_and_metadata(opml).expect("Should handle single outline");

        assert!(content.contains("Only Item"), "Should extract single item");
        assert_eq!(metadata.get("title").and_then(|v| v.as_str()), Some("Single"));
    }

    #[test]
    fn test_opml_head_without_body() {
        // OPML with head but no body element
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <head>
    <title>No Body</title>
  </head>
</opml>"#;

        let (content, metadata) =
            OpmlExtractor::extract_content_and_metadata(opml).expect("Should handle OPML without body");

        // Should extract metadata but have minimal content
        assert_eq!(metadata.get("title").and_then(|v| v.as_str()), Some("No Body"));
        // Content should be just the title (or empty)
        assert!(content.is_empty() || content.trim() == "No Body");
    }

    #[test]
    fn test_opml_body_without_head() {
        // OPML with body but no head element
        let opml = br#"<?xml version="1.0"?>
<opml version="2.0">
  <body>
    <outline text="Item" />
  </body>
</opml>"#;

        let (content, metadata) =
            OpmlExtractor::extract_content_and_metadata(opml).expect("Should handle OPML without head");

        assert!(content.contains("Item"), "Should extract body content");
        // No metadata should be extracted
        assert!(metadata.is_empty(), "Should have no metadata without head");
    }
}
