use crate::error::{KreuzbergError, Result};
use crate::types::HtmlMetadata;
use html_to_markdown_rs::{
    CodeBlockStyle, ConversionOptions, HeadingStyle, HighlightStyle, InlineImage,
    InlineImageConfig as RustInlineImageConfig, InlineImageFormat, ListIndentType, NewlineStyle, PreprocessingOptions,
    PreprocessingPreset, WhitespaceMode, convert as convert_html, convert_with_inline_images,
};
use saphyr::LoadableYamlNode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HtmlExtractionResult {
    pub markdown: String,
    pub images: Vec<ExtractedInlineImage>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedInlineImage {
    pub data: Vec<u8>,
    pub format: String,
    pub filename: Option<String>,
    pub description: Option<String>,
    pub dimensions: Option<(u32, u32)>,
    pub attributes: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HtmlConversionConfig {
    pub heading_style: Option<String>,
    pub list_indent_type: Option<String>,
    pub list_indent_width: Option<usize>,
    pub bullets: Option<String>,
    pub strong_em_symbol: Option<char>,
    pub escape_asterisks: Option<bool>,
    pub escape_underscores: Option<bool>,
    pub escape_misc: Option<bool>,
    pub escape_ascii: Option<bool>,
    pub code_language: Option<String>,
    pub autolinks: Option<bool>,
    pub default_title: Option<bool>,
    pub br_in_tables: Option<bool>,
    pub hocr_spatial_tables: Option<bool>,
    pub highlight_style: Option<String>,
    pub extract_metadata: Option<bool>,
    pub whitespace_mode: Option<String>,
    pub strip_newlines: Option<bool>,
    pub wrap: Option<bool>,
    pub wrap_width: Option<usize>,
    pub convert_as_inline: Option<bool>,
    pub sub_symbol: Option<String>,
    pub sup_symbol: Option<String>,
    pub newline_style: Option<String>,
    pub code_block_style: Option<String>,
    pub keep_inline_images_in: Option<Vec<String>>,
    pub debug: Option<bool>,
    pub strip_tags: Option<Vec<String>>,
    pub encoding: Option<String>,
    pub preprocessing: Option<HtmlPreprocessingConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HtmlPreprocessingConfig {
    pub enabled: Option<bool>,
    pub preset: Option<String>,
    pub remove_navigation: Option<bool>,
    pub remove_forms: Option<bool>,
}

impl Default for HtmlConversionConfig {
    fn default() -> Self {
        Self {
            heading_style: None,
            list_indent_type: None,
            list_indent_width: None,
            bullets: None,
            strong_em_symbol: None,
            escape_asterisks: None,
            escape_underscores: None,
            escape_misc: None,
            escape_ascii: None,
            code_language: None,
            autolinks: None,
            default_title: None,
            br_in_tables: None,
            hocr_spatial_tables: Some(false),
            highlight_style: None,
            extract_metadata: Some(true),
            whitespace_mode: None,
            strip_newlines: None,
            wrap: None,
            wrap_width: None,
            convert_as_inline: None,
            sub_symbol: None,
            sup_symbol: None,
            newline_style: None,
            code_block_style: None,
            keep_inline_images_in: None,
            debug: None,
            strip_tags: None,
            encoding: None,
            preprocessing: None,
        }
    }
}

fn parse_heading_style(value: &str) -> Result<HeadingStyle> {
    match value.to_lowercase().as_str() {
        "underlined" => Ok(HeadingStyle::Underlined),
        "atx" => Ok(HeadingStyle::Atx),
        "atx_closed" => Ok(HeadingStyle::AtxClosed),
        _ => Err(KreuzbergError::validation(format!(
            "Unsupported heading_style '{}'",
            value
        ))),
    }
}

fn parse_list_indent_type(value: &str) -> Result<ListIndentType> {
    match value.to_lowercase().as_str() {
        "spaces" => Ok(ListIndentType::Spaces),
        "tabs" => Ok(ListIndentType::Tabs),
        _ => Err(KreuzbergError::validation(format!(
            "Unsupported list_indent_type '{}'",
            value
        ))),
    }
}

fn parse_whitespace_mode(value: &str) -> Result<WhitespaceMode> {
    match value.to_lowercase().as_str() {
        "normalized" => Ok(WhitespaceMode::Normalized),
        "strict" => Ok(WhitespaceMode::Strict),
        _ => Err(KreuzbergError::validation(format!(
            "Unsupported whitespace_mode '{}'",
            value
        ))),
    }
}

fn parse_newline_style(value: &str) -> Result<NewlineStyle> {
    match value.to_lowercase().as_str() {
        "spaces" => Ok(NewlineStyle::Spaces),
        "backslash" => Ok(NewlineStyle::Backslash),
        _ => Err(KreuzbergError::validation(format!(
            "Unsupported newline_style '{}'",
            value
        ))),
    }
}

fn parse_code_block_style(value: &str) -> Result<CodeBlockStyle> {
    match value.to_lowercase().as_str() {
        "indented" => Ok(CodeBlockStyle::Indented),
        "backticks" => Ok(CodeBlockStyle::Backticks),
        "tildes" => Ok(CodeBlockStyle::Tildes),
        _ => Err(KreuzbergError::validation(format!(
            "Unsupported code_block_style '{}'",
            value
        ))),
    }
}

fn parse_highlight_style(value: &str) -> Result<HighlightStyle> {
    match value.to_lowercase().as_str() {
        "double-equal" => Ok(HighlightStyle::DoubleEqual),
        "html" => Ok(HighlightStyle::Html),
        "bold" => Ok(HighlightStyle::Bold),
        "none" => Ok(HighlightStyle::None),
        _ => Err(KreuzbergError::validation(format!(
            "Unsupported highlight_style '{}'",
            value
        ))),
    }
}

fn parse_preprocessing_preset(value: &str) -> Result<PreprocessingPreset> {
    match value.to_lowercase().as_str() {
        "minimal" => Ok(PreprocessingPreset::Minimal),
        "standard" => Ok(PreprocessingPreset::Standard),
        "aggressive" => Ok(PreprocessingPreset::Aggressive),
        _ => Err(KreuzbergError::validation(format!(
            "Unsupported preprocessing preset '{}'",
            value
        ))),
    }
}

fn build_conversion_options(config: Option<HtmlConversionConfig>) -> Result<ConversionOptions> {
    let mut options = ConversionOptions::default();

    if config.is_none() {
        options.hocr_spatial_tables = false;
        options.extract_metadata = true;
        return Ok(options);
    }

    let config = config.unwrap();

    if let Some(value) = config.heading_style {
        options.heading_style = parse_heading_style(&value)?;
    }
    if let Some(value) = config.list_indent_type {
        options.list_indent_type = parse_list_indent_type(&value)?;
    }
    if let Some(value) = config.list_indent_width {
        options.list_indent_width = value;
    }
    if let Some(value) = config.bullets {
        options.bullets = value;
    }
    if let Some(value) = config.strong_em_symbol {
        options.strong_em_symbol = value;
    }
    if let Some(value) = config.escape_asterisks {
        options.escape_asterisks = value;
    }
    if let Some(value) = config.escape_underscores {
        options.escape_underscores = value;
    }
    if let Some(value) = config.escape_misc {
        options.escape_misc = value;
    }
    if let Some(value) = config.escape_ascii {
        options.escape_ascii = value;
    }
    if let Some(value) = config.code_language {
        options.code_language = value;
    }
    if let Some(value) = config.autolinks {
        options.autolinks = value;
    }
    if let Some(value) = config.default_title {
        options.default_title = value;
    }
    if let Some(value) = config.br_in_tables {
        options.br_in_tables = value;
    }
    if let Some(value) = config.hocr_spatial_tables {
        options.hocr_spatial_tables = value;
    }
    if let Some(value) = config.highlight_style {
        options.highlight_style = parse_highlight_style(&value)?;
    }
    if let Some(value) = config.extract_metadata {
        options.extract_metadata = value;
    }
    if let Some(value) = config.whitespace_mode {
        options.whitespace_mode = parse_whitespace_mode(&value)?;
    }
    if let Some(value) = config.strip_newlines {
        options.strip_newlines = value;
    }
    if let Some(value) = config.wrap {
        options.wrap = value;
    }
    if let Some(value) = config.wrap_width {
        options.wrap_width = value;
    }
    if let Some(value) = config.convert_as_inline {
        options.convert_as_inline = value;
    }
    if let Some(value) = config.sub_symbol {
        options.sub_symbol = value;
    }
    if let Some(value) = config.sup_symbol {
        options.sup_symbol = value;
    }
    if let Some(value) = config.newline_style {
        options.newline_style = parse_newline_style(&value)?;
    }
    if let Some(value) = config.code_block_style {
        options.code_block_style = parse_code_block_style(&value)?;
    }
    if let Some(value) = config.keep_inline_images_in {
        options.keep_inline_images_in = value;
    }
    if let Some(value) = config.debug {
        options.debug = value;
    }
    if let Some(value) = config.strip_tags {
        options.strip_tags = value;
    }
    if let Some(value) = config.encoding {
        options.encoding = value;
    }

    if let Some(prep_config) = config.preprocessing {
        let mut preprocessing = PreprocessingOptions::default();
        if let Some(enabled) = prep_config.enabled {
            preprocessing.enabled = enabled;
        }
        if let Some(preset) = prep_config.preset {
            preprocessing.preset = parse_preprocessing_preset(&preset)?;
        }
        if let Some(remove_navigation) = prep_config.remove_navigation {
            preprocessing.remove_navigation = remove_navigation;
        }
        if let Some(remove_forms) = prep_config.remove_forms {
            preprocessing.remove_forms = remove_forms;
        }
        options.preprocessing = preprocessing;
    }

    Ok(options)
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

/// Convert HTML to markdown without extracting images
pub fn convert_html_to_markdown(html: &str, config: Option<HtmlConversionConfig>) -> Result<String> {
    let options = build_conversion_options(config)?;
    convert_html(html, Some(options))
        .map_err(|e| KreuzbergError::parsing(format!("Failed to convert HTML to Markdown: {}", e)))
}

/// Process HTML with optional image extraction
pub fn process_html(
    html: &str,
    config: Option<HtmlConversionConfig>,
    extract_images: bool,
    max_image_size: u64,
) -> Result<HtmlExtractionResult> {
    let options = build_conversion_options(config)?;

    if extract_images {
        let mut img_config = RustInlineImageConfig::new(max_image_size);
        img_config.filename_prefix = Some("inline-image".to_string());

        let extraction = convert_with_inline_images(html, Some(options), img_config)
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
        let markdown = convert_html(html, Some(options))
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
    // Check if markdown starts with YAML frontmatter delimiter
    if !markdown.starts_with("---\n") && !markdown.starts_with("---\r\n") {
        return Ok((None, markdown.to_string()));
    }

    // Find the closing delimiter
    let after_opening = if let Some(stripped) = markdown.strip_prefix("---\r\n") {
        stripped
    } else if let Some(stripped) = markdown.strip_prefix("---\n") {
        stripped
    } else {
        // Shouldn't happen due to check above, but handle it gracefully
        return Ok((None, markdown.to_string()));
    };

    // Check for closing delimiter at the start (empty frontmatter) or after content
    let (yaml_content, remaining_content) = if after_opening.starts_with("---\n") {
        // Empty frontmatter: ---\n---\n
        let content = after_opening.strip_prefix("---\n").unwrap_or(after_opening);
        ("", content)
    } else if after_opening.starts_with("---\r\n") {
        // Empty frontmatter with CRLF: ---\r\n---\r\n
        let content = after_opening.strip_prefix("---\r\n").unwrap_or(after_opening);
        ("", content)
    } else if let Some(pos) = after_opening
        .find("\n---\n")
        .or_else(|| after_opening.find("\r\n---\r\n"))
    {
        // Frontmatter with content
        let yaml = &after_opening[..pos];
        let content_start = pos + if after_opening[pos..].starts_with("\r\n") { 7 } else { 5 };
        let content = &after_opening[content_start..];
        (yaml, content)
    } else {
        // No closing delimiter found, treat as no frontmatter
        return Ok((None, markdown.to_string()));
    };

    // If yaml_content is empty, return None for metadata
    if yaml_content.is_empty() {
        return Ok((None, remaining_content.to_string()));
    }

    // Parse YAML using saphyr
    let yaml_docs = saphyr::Yaml::load_from_str(yaml_content)
        .map_err(|e| KreuzbergError::parsing(format!("Failed to parse YAML frontmatter: {}", e)))?;

    if yaml_docs.is_empty() {
        return Ok((None, remaining_content.to_string()));
    }

    let yaml_doc = &yaml_docs[0];

    // Extract metadata fields
    let mut metadata = HtmlMetadata::default();

    if let saphyr::Yaml::Mapping(mapping) = yaml_doc {
        for (key, value) in mapping {
            if let (saphyr::Yaml::Value(saphyr::Scalar::String(k)), saphyr::Yaml::Value(saphyr::Scalar::String(v))) =
                (key, value)
            {
                let key_str = k.to_string();
                let value_str = v.to_string();

                match key_str.as_str() {
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
                    _ => {} // Ignore unknown fields
                }
            }
        }
    }

    // Check if any metadata was extracted
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
        let config = HtmlConversionConfig {
            preprocessing: Some(HtmlPreprocessingConfig {
                enabled: Some(false),
                preset: None,
                remove_navigation: None,
                remove_forms: None,
            }),
            ..Default::default()
        };
        let result = process_html(html, Some(config), true, 1024 * 1024).unwrap();
        assert_eq!(result.images.len(), 1);
        assert_eq!(result.images[0].format, "png");
    }

    #[test]
    fn test_html_config_heading_style() {
        let html = "<h1>Heading</h1>";
        let config = HtmlConversionConfig {
            heading_style: Some("atx".to_string()),
            ..Default::default()
        };
        let result = convert_html_to_markdown(html, Some(config)).unwrap();
        assert!(result.contains("# Heading"));
    }

    #[test]
    fn test_invalid_heading_style() {
        let config = HtmlConversionConfig {
            heading_style: Some("invalid".to_string()),
            ..Default::default()
        };
        let result = build_conversion_options(Some(config));
        assert!(result.is_err());
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
        let config = HtmlConversionConfig {
            preprocessing: Some(HtmlPreprocessingConfig {
                enabled: Some(true),
                preset: Some("standard".to_string()),
                remove_navigation: Some(true),
                remove_forms: None,
            }),
            ..Default::default()
        };
        let result = convert_html_to_markdown(html, Some(config)).unwrap();
        assert!(result.contains("Content"));
    }

    #[test]
    fn test_parse_list_indent_type_valid() {
        assert!(parse_list_indent_type("spaces").is_ok());
        assert!(parse_list_indent_type("tabs").is_ok());
        assert!(parse_list_indent_type("TABS").is_ok());
    }

    #[test]
    fn test_parse_list_indent_type_invalid() {
        let result = parse_list_indent_type("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_whitespace_mode_valid() {
        assert!(parse_whitespace_mode("normalized").is_ok());
        assert!(parse_whitespace_mode("strict").is_ok());
        assert!(parse_whitespace_mode("STRICT").is_ok());
    }

    #[test]
    fn test_parse_whitespace_mode_invalid() {
        let result = parse_whitespace_mode("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_newline_style_valid() {
        assert!(parse_newline_style("spaces").is_ok());
        assert!(parse_newline_style("backslash").is_ok());
    }

    #[test]
    fn test_parse_newline_style_invalid() {
        let result = parse_newline_style("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_code_block_style_valid() {
        assert!(parse_code_block_style("indented").is_ok());
        assert!(parse_code_block_style("backticks").is_ok());
        assert!(parse_code_block_style("tildes").is_ok());
    }

    #[test]
    fn test_parse_code_block_style_invalid() {
        let result = parse_code_block_style("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_highlight_style_valid() {
        assert!(parse_highlight_style("double-equal").is_ok());
        assert!(parse_highlight_style("html").is_ok());
        assert!(parse_highlight_style("bold").is_ok());
        assert!(parse_highlight_style("none").is_ok());
    }

    #[test]
    fn test_parse_highlight_style_invalid() {
        let result = parse_highlight_style("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_preprocessing_preset_valid() {
        assert!(parse_preprocessing_preset("minimal").is_ok());
        assert!(parse_preprocessing_preset("standard").is_ok());
        assert!(parse_preprocessing_preset("aggressive").is_ok());
    }

    #[test]
    fn test_parse_preprocessing_preset_invalid() {
        let result = parse_preprocessing_preset("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_heading_style_valid() {
        assert!(parse_heading_style("underlined").is_ok());
        assert!(parse_heading_style("atx").is_ok());
        assert!(parse_heading_style("atx_closed").is_ok());
    }

    #[test]
    fn test_parse_heading_style_invalid() {
        let result = parse_heading_style("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_inline_image_format_other_with_extension() {
        let format = InlineImageFormat::Other("image/x-custom.jpg".to_string());
        assert_eq!(inline_image_format_to_str(&format), "jpg");
    }

    #[test]
    fn test_inline_image_format_other_with_plus() {
        let format = InlineImageFormat::Other("image/svg+xml".to_string());
        let result = inline_image_format_to_str(&format);
        assert!(result == "svg" || result == "image/svg");
    }

    #[test]
    fn test_inline_image_format_other_empty() {
        let format = InlineImageFormat::Other("".to_string());
        assert_eq!(inline_image_format_to_str(&format), "bin");
    }

    #[test]
    fn test_inline_image_format_other_whitespace() {
        let format = InlineImageFormat::Other("   ".to_string());
        assert_eq!(inline_image_format_to_str(&format), "bin");
    }

    #[test]
    fn test_inline_image_format_other_x_prefix() {
        let format = InlineImageFormat::Other("x-custom".to_string());
        assert_eq!(inline_image_format_to_str(&format), "custom");
    }

    #[test]
    fn test_html_config_default_values() {
        let config = HtmlConversionConfig::default();
        assert_eq!(config.hocr_spatial_tables, Some(false));
        assert_eq!(config.extract_metadata, Some(true));
        assert!(config.heading_style.is_none());
    }

    #[test]
    fn test_build_conversion_options_none() {
        let options = build_conversion_options(None).unwrap();
        assert!(!options.hocr_spatial_tables);
        assert!(options.extract_metadata);
    }

    #[test]
    fn test_build_conversion_options_with_config() {
        let config = HtmlConversionConfig {
            heading_style: Some("atx".to_string()),
            list_indent_width: Some(4),
            escape_asterisks: Some(true),
            ..Default::default()
        };
        let options = build_conversion_options(Some(config)).unwrap();
        assert_eq!(options.list_indent_width, 4);
        assert!(options.escape_asterisks);
    }

    #[test]
    fn test_html_extraction_result_structure() {
        let result = HtmlExtractionResult {
            markdown: "test".to_string(),
            images: vec![],
            warnings: vec!["warning".to_string()],
        };
        assert_eq!(result.markdown, "test");
        assert!(result.images.is_empty());
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_extracted_inline_image_structure() {
        let image = ExtractedInlineImage {
            data: vec![1, 2, 3],
            format: "png".to_string(),
            filename: Some("test.png".to_string()),
            description: Some("alt text".to_string()),
            dimensions: Some((100, 200)),
            attributes: HashMap::new(),
        };
        assert_eq!(image.data.len(), 3);
        assert_eq!(image.format, "png");
        assert_eq!(image.dimensions, Some((100, 200)));
    }

    #[test]
    fn test_process_html_empty_string() {
        let result = process_html("", None, false, 1024).unwrap();
        assert!(result.markdown.is_empty() || result.markdown.trim().is_empty());
        assert!(result.images.is_empty());
    }

    #[test]
    fn test_convert_html_with_code_block() {
        let html = "<pre><code>let x = 5;</code></pre>";
        let result = convert_html_to_markdown(html, None).unwrap();
        assert!(result.contains("let x = 5;"));
    }

    #[test]
    fn test_html_with_multiple_paragraphs() {
        let html = "<p>First paragraph.</p><p>Second paragraph.</p><p>Third paragraph.</p>";
        let result = convert_html_to_markdown(html, None).unwrap();
        assert!(result.contains("First paragraph"));
        assert!(result.contains("Second paragraph"));
        assert!(result.contains("Third paragraph"));
    }

    #[test]
    fn test_html_with_nested_lists() {
        let html = "<ul><li>Item 1<ul><li>Nested 1</li></ul></li></ul>";
        let result = convert_html_to_markdown(html, None).unwrap();
        assert!(result.contains("Item 1"));
        assert!(result.contains("Nested 1"));
    }

    #[test]
    fn test_inline_image_format_all_variants() {
        assert_eq!(inline_image_format_to_str(&InlineImageFormat::Png), "png");
        assert_eq!(inline_image_format_to_str(&InlineImageFormat::Jpeg), "jpeg");
        assert_eq!(inline_image_format_to_str(&InlineImageFormat::Gif), "gif");
        assert_eq!(inline_image_format_to_str(&InlineImageFormat::Bmp), "bmp");
        assert_eq!(inline_image_format_to_str(&InlineImageFormat::Webp), "webp");
        assert_eq!(inline_image_format_to_str(&InlineImageFormat::Svg), "svg");
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

        // Should treat as no frontmatter when closing delimiter is missing
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
}
