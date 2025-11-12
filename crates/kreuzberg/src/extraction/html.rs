//! HTML to Markdown conversion functions.
//!
//! This module provides HTML to Markdown conversion using the `html-to-markdown-rs` library.
//! It supports inline image extraction and YAML frontmatter parsing for HTML metadata.
//!
//! # Features
//!
//! - **HTML to Markdown conversion**: Clean, readable Markdown output
//! - **Inline image extraction**: Extract base64 and data URI images
//! - **YAML frontmatter**: Parse YAML metadata from Markdown output
//! - **Customizable conversion**: Full access to `html-to-markdown-rs` options
//!
//! # Example
//!
//! ```rust
//! use kreuzberg::extraction::html::convert_html_to_markdown;
//!
//! # fn example() -> kreuzberg::Result<()> {
//! let html = r#"<h1>Title</h1><p>This is <strong>bold</strong> text.</p>"#;
//! let markdown = convert_html_to_markdown(html, None)?;
//!
//! assert!(markdown.contains("# Title"));
//! assert!(markdown.contains("**bold**"));
//! # Ok(())
//! # }
//! ```
use crate::error::{KreuzbergError, Result};
use crate::types::HtmlMetadata;
use html_to_markdown_rs::{
    ConversionOptions, InlineImage, InlineImageConfig as LibInlineImageConfig, InlineImageFormat,
    convert as convert_html, convert_with_inline_images,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use html_to_markdown_rs::{
    CodeBlockStyle, HeadingStyle, HighlightStyle, ListIndentType, NewlineStyle, PreprocessingOptions,
    PreprocessingPreset, WhitespaceMode,
};

/// Result of HTML extraction with optional images and warnings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HtmlExtractionResult {
    pub markdown: String,
    pub images: Vec<ExtractedInlineImage>,
    pub warnings: Vec<String>,
}

/// Extracted inline image with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedInlineImage {
    pub data: Vec<u8>,
    pub format: String,
    pub filename: Option<String>,
    pub description: Option<String>,
    pub dimensions: Option<(u32, u32)>,
    pub attributes: HashMap<String, String>,
}

fn inline_image_format_to_str(format: &InlineImageFormat) -> String {
    match format {
        InlineImageFormat::Png => "png".to_string(),
        InlineImageFormat::Jpeg => "jpeg".to_string(),
        InlineImageFormat::Gif => "gif".to_string(),
        InlineImageFormat::Bmp => "bmp".to_string(),
        InlineImageFormat::Webp => "webp".to_string(),
        InlineImageFormat::Svg => "svg".to_string(),
        InlineImageFormat::Other(custom) => {
            let trimmed = custom.trim();
            if trimmed.is_empty() {
                return "bin".to_string();
            }

            let lower = trimmed.to_ascii_lowercase();
            if lower.starts_with("svg") {
                return "svg".to_string();
            }

            let mut candidate = lower.as_str();

            if let Some(idx) = candidate.find(['+', ';']) {
                candidate = &candidate[..idx];
            }

            if let Some(idx) = candidate.rfind('.') {
                candidate = &candidate[idx + 1..];
            }

            candidate = candidate.trim_start_matches("x-");

            if candidate.is_empty() {
                "bin".to_string()
            } else {
                candidate.to_string()
            }
        }
    }
}

fn inline_image_to_extracted(image: InlineImage) -> ExtractedInlineImage {
    ExtractedInlineImage {
        data: image.data,
        format: inline_image_format_to_str(&image.format),
        filename: image.filename,
        description: image.description,
        dimensions: image.dimensions,
        attributes: image.attributes.into_iter().collect(),
    }
}

/// Convert HTML to markdown with optional configuration.
///
/// Uses sensible defaults if no configuration is provided:
/// - `extract_metadata = true` (parse YAML frontmatter)
/// - `hocr_spatial_tables = false` (disable hOCR table detection)
pub fn convert_html_to_markdown(html: &str, options: Option<ConversionOptions>) -> Result<String> {
    let opts = options.unwrap_or_else(|| ConversionOptions {
        extract_metadata: true,
        hocr_spatial_tables: false,
        ..Default::default()
    });

    convert_html(html, Some(opts))
        .map_err(|e| KreuzbergError::parsing(format!("Failed to convert HTML to Markdown: {}", e)))
}

/// Process HTML with optional image extraction.
pub fn process_html(
    html: &str,
    options: Option<ConversionOptions>,
    extract_images: bool,
    max_image_size: u64,
) -> Result<HtmlExtractionResult> {
    let opts = options.unwrap_or_else(|| ConversionOptions {
        extract_metadata: true,
        hocr_spatial_tables: false,
        ..Default::default()
    });

    if extract_images {
        let mut img_config = LibInlineImageConfig::new(max_image_size);
        img_config.filename_prefix = Some("inline-image".to_string());

        let extraction = convert_with_inline_images(html, Some(opts), img_config)
            .map_err(|e| KreuzbergError::parsing(format!("Failed to convert HTML to Markdown with images: {}", e)))?;

        let images = extraction
            .inline_images
            .into_iter()
            .map(inline_image_to_extracted)
            .collect();

        let warnings = extraction.warnings.into_iter().map(|w| w.message).collect();

        Ok(HtmlExtractionResult {
            markdown: extraction.markdown,
            images,
            warnings,
        })
    } else {
        let markdown = convert_html(html, Some(opts))
            .map_err(|e| KreuzbergError::parsing(format!("Failed to convert HTML to Markdown: {}", e)))?;

        Ok(HtmlExtractionResult {
            markdown,
            images: Vec::new(),
            warnings: Vec::new(),
        })
    }
}

/// Parse YAML frontmatter from markdown and extract HTML metadata.
///
/// Returns a tuple of (HtmlMetadata, content_without_frontmatter).
pub fn parse_html_metadata(markdown: &str) -> Result<(Option<HtmlMetadata>, String)> {
    if !markdown.starts_with("---\n") && !markdown.starts_with("---\r\n") {
        return Ok((None, markdown.to_string()));
    }

    let after_opening = if let Some(stripped) = markdown.strip_prefix("---\r\n") {
        stripped
    } else if let Some(stripped) = markdown.strip_prefix("---\n") {
        stripped
    } else {
        return Ok((None, markdown.to_string()));
    };

    let (yaml_content, remaining_content) = if after_opening.starts_with("---\n") {
        let content = after_opening.strip_prefix("---\n").unwrap_or(after_opening);
        ("", content)
    } else if after_opening.starts_with("---\r\n") {
        let content = after_opening.strip_prefix("---\r\n").unwrap_or(after_opening);
        ("", content)
    } else if let Some(pos) = after_opening
        .find("\n---\n")
        .or_else(|| after_opening.find("\r\n---\r\n"))
    {
        let yaml = &after_opening[..pos];
        let content_start = pos + if after_opening[pos..].starts_with("\r\n") { 7 } else { 5 };
        let content = &after_opening[content_start..];
        (yaml, content)
    } else {
        return Ok((None, markdown.to_string()));
    };

    if yaml_content.is_empty() {
        return Ok((None, remaining_content.to_string()));
    }

    let yaml_value: serde_json::Value = serde_yaml_ng::from_str(yaml_content)
        .map_err(|e| KreuzbergError::parsing(format!("Failed to parse YAML frontmatter: {}", e)))?;

    let mut metadata = HtmlMetadata::default();

    if let serde_json::Value::Object(mapping) = yaml_value {
        for (key, value) in mapping {
            if let serde_json::Value::String(value_str) = value {
                match key.as_str() {
                    "title" => metadata.title = Some(value_str),
                    "base-href" => metadata.base_href = Some(value_str),
                    "canonical" => metadata.canonical = Some(value_str),
                    "meta-description" => metadata.description = Some(value_str),
                    "meta-keywords" => metadata.keywords = Some(value_str),
                    "meta-author" => metadata.author = Some(value_str),
                    "meta-og-title" | "meta-og:title" => metadata.og_title = Some(value_str),
                    "meta-og-description" | "meta-og:description" => metadata.og_description = Some(value_str),
                    "meta-og-image" | "meta-og:image" => metadata.og_image = Some(value_str),
                    "meta-og-url" | "meta-og:url" => metadata.og_url = Some(value_str),
                    "meta-og-type" | "meta-og:type" => metadata.og_type = Some(value_str),
                    "meta-og-site-name" | "meta-og:site-name" | "meta-og:site_name" => {
                        metadata.og_site_name = Some(value_str)
                    }
                    "meta-twitter-card" | "meta-twitter:card" => metadata.twitter_card = Some(value_str),
                    "meta-twitter-title" | "meta-twitter:title" => metadata.twitter_title = Some(value_str),
                    "meta-twitter-description" | "meta-twitter:description" => {
                        metadata.twitter_description = Some(value_str)
                    }
                    "meta-twitter-image" | "meta-twitter:image" => metadata.twitter_image = Some(value_str),
                    "meta-twitter-site" | "meta-twitter:site" => metadata.twitter_site = Some(value_str),
                    "meta-twitter-creator" | "meta-twitter:creator" => metadata.twitter_creator = Some(value_str),
                    "link-author" => metadata.link_author = Some(value_str),
                    "link-license" => metadata.link_license = Some(value_str),
                    "link-alternate" => metadata.link_alternate = Some(value_str),
                    _ => {}
                }
            }
        }
    }

    let has_metadata = metadata.title.is_some()
        || metadata.description.is_some()
        || metadata.keywords.is_some()
        || metadata.author.is_some()
        || metadata.canonical.is_some()
        || metadata.base_href.is_some()
        || metadata.og_title.is_some()
        || metadata.og_description.is_some()
        || metadata.og_image.is_some()
        || metadata.twitter_card.is_some();

    if has_metadata {
        Ok((Some(metadata), remaining_content.to_string()))
    } else {
        Ok((None, remaining_content.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_simple_html() {
        let html = "<h1>Hello World</h1><p>This is a test.</p>";
        let result = convert_html_to_markdown(html, None).unwrap();
        assert!(result.contains("# Hello World"));
        assert!(result.contains("This is a test."));
    }

    #[test]
    fn test_process_html_without_images() {
        let html = "<h1>Test</h1><p>Content</p>";
        let result = process_html(html, None, false, 1024 * 1024).unwrap();
        assert!(result.markdown.contains("# Test"));
        assert!(result.markdown.contains("Content"));
        assert!(result.images.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_html_with_inline_image() {
        let html = r#"<p>Image: <img src="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==" alt="Test"></p>"#;
        let mut options = ConversionOptions::default();
        options.preprocessing.enabled = false;
        let result = process_html(html, Some(options), true, 1024 * 1024).unwrap();
        assert_eq!(result.images.len(), 1);
        assert_eq!(result.images[0].format, "png");
    }

    #[test]
    fn test_html_config_heading_style() {
        let html = "<h1>Heading</h1>";
        let options = ConversionOptions {
            heading_style: HeadingStyle::Atx,
            ..Default::default()
        };
        let result = convert_html_to_markdown(html, Some(options)).unwrap();
        assert!(result.contains("# Heading"));
    }

    #[test]
    fn test_html_with_list() {
        let html = "<ul><li>Item 1</li><li>Item 2</li></ul>";
        let result = convert_html_to_markdown(html, None).unwrap();
        assert!(result.contains("Item 1"));
        assert!(result.contains("Item 2"));
    }

    #[test]
    fn test_html_with_table() {
        let html = "<table><tr><th>Header</th></tr><tr><td>Data</td></tr></table>";
        let result = convert_html_to_markdown(html, None).unwrap();
        assert!(result.contains("Header"));
        assert!(result.contains("Data"));
    }

    #[test]
    fn test_inline_image_format_conversion() {
        assert_eq!(inline_image_format_to_str(&InlineImageFormat::Png), "png");
        assert_eq!(inline_image_format_to_str(&InlineImageFormat::Jpeg), "jpeg");
        assert_eq!(inline_image_format_to_str(&InlineImageFormat::Svg), "svg");
    }

    #[test]
    fn test_preprocessing_config() {
        let html = "<nav>Navigation</nav><p>Content</p>";
        let mut options = ConversionOptions::default();
        options.preprocessing.enabled = true;
        options.preprocessing.preset = PreprocessingPreset::Standard;
        options.preprocessing.remove_navigation = true;
        let result = convert_html_to_markdown(html, Some(options)).unwrap();
        assert!(result.contains("Content"));
    }

    #[test]
    fn test_inline_image_format_other_with_extension() {
        let format = InlineImageFormat::Other("image/x-custom.jpg".to_string());
        assert_eq!(inline_image_format_to_str(&format), "jpg");
    }

    #[test]
    fn test_inline_image_format_other_empty() {
        let format = InlineImageFormat::Other("".to_string());
        assert_eq!(inline_image_format_to_str(&format), "bin");
    }

    #[test]
    fn test_inline_image_format_other_x_prefix() {
        let format = InlineImageFormat::Other("x-custom".to_string());
        assert_eq!(inline_image_format_to_str(&format), "custom");
    }

    #[test]
    fn test_process_html_empty_string() {
        let result = process_html("", None, false, 1024).unwrap();
        assert!(result.markdown.is_empty() || result.markdown.trim().is_empty());
        assert!(result.images.is_empty());
    }

    #[test]
    fn test_parse_html_metadata_with_frontmatter() {
        let markdown = "---\ntitle: Test Page\nmeta-description: A test page\nmeta-keywords: test, page\n---\n\n# Content\n\nSome content.";
        let (metadata, content) = parse_html_metadata(markdown).unwrap();

        assert!(metadata.is_some());
        let meta = metadata.unwrap();
        assert_eq!(meta.title, Some("Test Page".to_string()));
        assert_eq!(meta.description, Some("A test page".to_string()));
        assert_eq!(meta.keywords, Some("test, page".to_string()));
        assert_eq!(content.trim(), "# Content\n\nSome content.");
    }

    #[test]
    fn test_parse_html_metadata_without_frontmatter() {
        let markdown = "# Content\n\nSome content without frontmatter.";
        let (metadata, content) = parse_html_metadata(markdown).unwrap();

        assert!(metadata.is_none());
        assert_eq!(content, markdown);
    }

    #[test]
    fn test_parse_html_metadata_with_open_graph() {
        let markdown = "---\ntitle: OG Test\nmeta-og-title: OG Title\nmeta-og-description: OG Description\nmeta-og-image: https://example.com/image.jpg\n---\n\nContent";
        let (metadata, _content) = parse_html_metadata(markdown).unwrap();

        assert!(metadata.is_some());
        let meta = metadata.unwrap();
        assert_eq!(meta.title, Some("OG Test".to_string()));
        assert_eq!(meta.og_title, Some("OG Title".to_string()));
        assert_eq!(meta.og_description, Some("OG Description".to_string()));
        assert_eq!(meta.og_image, Some("https://example.com/image.jpg".to_string()));
    }

    #[test]
    fn test_parse_html_metadata_with_twitter_card() {
        let markdown = "---\nmeta-twitter-card: summary\nmeta-twitter-title: Twitter Title\nmeta-twitter-image: https://example.com/twitter.jpg\n---\n\nContent";
        let (metadata, _content) = parse_html_metadata(markdown).unwrap();

        assert!(metadata.is_some());
        let meta = metadata.unwrap();
        assert_eq!(meta.twitter_card, Some("summary".to_string()));
        assert_eq!(meta.twitter_title, Some("Twitter Title".to_string()));
        assert_eq!(meta.twitter_image, Some("https://example.com/twitter.jpg".to_string()));
    }

    #[test]
    fn test_parse_html_metadata_with_links() {
        let markdown = "---\ncanonical: https://example.com/page\nlink-author: https://example.com/author\nlink-license: https://creativecommons.org/licenses/by/4.0/\n---\n\nContent";
        let (metadata, _content) = parse_html_metadata(markdown).unwrap();

        assert!(metadata.is_some());
        let meta = metadata.unwrap();
        assert_eq!(meta.canonical, Some("https://example.com/page".to_string()));
        assert_eq!(meta.link_author, Some("https://example.com/author".to_string()));
        assert_eq!(
            meta.link_license,
            Some("https://creativecommons.org/licenses/by/4.0/".to_string())
        );
    }

    #[test]
    fn test_parse_html_metadata_empty_frontmatter() {
        let markdown = "---\n---\n\nContent";
        let (metadata, content) = parse_html_metadata(markdown).unwrap();

        assert!(metadata.is_none());
        assert_eq!(content.trim(), "Content");
    }

    #[test]
    fn test_parse_html_metadata_incomplete_frontmatter() {
        let markdown = "---\ntitle: Test\n\nNo closing delimiter";
        let (metadata, content) = parse_html_metadata(markdown).unwrap();

        assert!(metadata.is_none());
        assert_eq!(content, markdown);
    }

    #[test]
    fn test_parse_html_metadata_crlf_line_endings() {
        let markdown = "---\r\ntitle: Test\r\nmeta-author: John Doe\r\n---\r\n\r\nContent";
        let (metadata, content) = parse_html_metadata(markdown).unwrap();

        assert!(metadata.is_some());
        let meta = metadata.unwrap();
        assert_eq!(meta.title, Some("Test".to_string()));
        assert_eq!(meta.author, Some("John Doe".to_string()));
        assert_eq!(content.trim(), "Content");
    }

    #[test]
    fn test_preprocessing_keeps_main_content() {
        let html = r#"
<!DOCTYPE html>
<html>
  <body>
    <nav><p>Skip me</p></nav>
    <main id="content">
      <article>
        <h1>Taylor Swift</h1>
        <p>Taylor Alison Swift is an American singer-songwriter.</p>
      </article>
    </main>
  </body>
</html>
"#;
        let markdown = convert_html_to_markdown(html, None).expect("conversion failed");
        assert!(markdown.contains("Taylor Alison Swift"), "{markdown}");
    }
}
