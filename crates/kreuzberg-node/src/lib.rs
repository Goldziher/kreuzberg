#![deny(clippy::all)]

use html_to_markdown_rs::options::{
    CodeBlockStyle, ConversionOptions, HeadingStyle, HighlightStyle, ListIndentType, NewlineStyle,
    PreprocessingOptions as HtmlPreprocessingOptions, PreprocessingPreset, WhitespaceMode,
};
use kreuzberg::keywords::{
    KeywordAlgorithm as RustKeywordAlgorithm, KeywordConfig as RustKeywordConfig, RakeParams as RustRakeParams,
    YakeParams as RustYakeParams,
};
use kreuzberg::plugins::registry::{get_post_processor_registry, get_validator_registry};
use kreuzberg::{
    Chunk as RustChunk, ChunkMetadata as RustChunkMetadata, ChunkingConfig as RustChunkingConfig,
    EmbeddingConfig as RustEmbeddingConfig, EmbeddingModelType as RustEmbeddingModelType, ExtractionConfig,
    ExtractionResult as RustExtractionResult, ImageExtractionConfig as RustImageExtractionConfig,
    LanguageDetectionConfig as RustLanguageDetectionConfig, OcrConfig as RustOcrConfig, PdfConfig as RustPdfConfig,
    PostProcessorConfig as RustPostProcessorConfig, TesseractConfig as RustTesseractConfig,
    TokenReductionConfig as RustTokenReductionConfig,
};
use napi::bindgen_prelude::*;
use napi_derive::napi;

/// Converts KreuzbergError to NAPI Error with specific error codes.
///
/// This function maps Kreuzberg error variants to appropriate NAPI status codes,
/// preserving error semantics for JavaScript/TypeScript callers:
///
/// - `Io` → GenericFailure (system-level I/O errors)
/// - `Parsing` → InvalidArg (malformed documents, corrupt files)
/// - `Ocr` → GenericFailure (OCR processing failures)
/// - `Validation` → InvalidArg (invalid configuration or parameters)
/// - `Cache` → GenericFailure (non-fatal cache errors)
/// - `ImageProcessing` → GenericFailure (image manipulation errors)
/// - `Serialization` → InvalidArg (JSON/MessagePack errors)
/// - `MissingDependency` → GenericFailure (missing system dependencies)
/// - `Plugin` → GenericFailure (plugin-specific errors)
/// - `LockPoisoned` → GenericFailure (lock poisoning, should not happen)
/// - `UnsupportedFormat` → InvalidArg (unsupported MIME types)
/// - `Other` → GenericFailure (catch-all)
///
/// # Usage
///
/// ```rust,ignore
/// kreuzberg::extract_file_sync(&path, None, &config)
///     .map_err(convert_error)
///     .and_then(JsExtractionResult::try_from)
/// ```
#[inline]
fn convert_error(err: kreuzberg::KreuzbergError) -> napi::Error {
    use kreuzberg::KreuzbergError;

    match err {
        KreuzbergError::Io(e) => Error::new(Status::GenericFailure, format!("IO error: {}", e)),

        KreuzbergError::Parsing { message, .. } => {
            Error::new(Status::InvalidArg, format!("Parsing error: {}", message))
        }

        KreuzbergError::Ocr { message, .. } => Error::new(Status::GenericFailure, format!("OCR error: {}", message)),

        KreuzbergError::Validation { message, .. } => {
            Error::new(Status::InvalidArg, format!("Validation error: {}", message))
        }

        KreuzbergError::Cache { message, .. } => {
            Error::new(Status::GenericFailure, format!("Cache error: {}", message))
        }

        KreuzbergError::ImageProcessing { message, .. } => {
            Error::new(Status::GenericFailure, format!("Image processing error: {}", message))
        }

        KreuzbergError::Serialization { message, .. } => {
            Error::new(Status::InvalidArg, format!("Serialization error: {}", message))
        }

        KreuzbergError::MissingDependency(dep) => {
            Error::new(Status::GenericFailure, format!("Missing dependency: {}", dep))
        }

        KreuzbergError::Plugin { message, plugin_name } => Error::new(
            Status::GenericFailure,
            format!("Plugin error in '{}': {}", plugin_name, message),
        ),

        KreuzbergError::LockPoisoned(msg) => Error::new(Status::GenericFailure, format!("Lock poisoned: {}", msg)),

        KreuzbergError::UnsupportedFormat(format) => {
            Error::new(Status::InvalidArg, format!("Unsupported format: {}", format))
        }

        KreuzbergError::Other(msg) => Error::new(Status::GenericFailure, msg),
    }
}

/// Validates that a JavaScript object has all required properties before plugin registration.
///
/// This helper function checks if a plugin object has all required methods and provides
/// clear error messages if any are missing. This improves developer experience by
/// catching configuration errors early with actionable error messages.
///
/// # Arguments
///
/// * `obj` - The JavaScript object to validate
/// * `plugin_type` - Type of plugin (for error messages, e.g., "PostProcessor")
/// * `required_methods` - Slice of required method names
///
/// # Returns
///
/// Returns `Ok(())` if all required methods exist, or an error with details about
/// which methods are missing.
///
/// # Example
///
/// ```rust,ignore
/// validate_plugin_object(&processor, "PostProcessor", &["name", "process"])?;
/// ```
fn validate_plugin_object(obj: &Object, plugin_type: &str, required_methods: &[&str]) -> Result<()> {
    let mut missing_methods = Vec::new();

    for method_name in required_methods {
        if !obj.has_named_property(method_name)? {
            missing_methods.push(*method_name);
        }
    }

    if !missing_methods.is_empty() {
        return Err(Error::new(
            Status::InvalidArg,
            format!(
                "{} is missing required methods: {}. Please ensure your plugin implements all required methods.",
                plugin_type,
                missing_methods.join(", ")
            ),
        ));
    }

    Ok(())
}

#[napi(object)]
pub struct JsOcrConfig {
    pub backend: String,
    pub language: Option<String>,
    pub tesseract_config: Option<JsTesseractConfig>,
}

impl From<JsOcrConfig> for RustOcrConfig {
    fn from(val: JsOcrConfig) -> Self {
        RustOcrConfig {
            backend: val.backend,
            language: val.language.unwrap_or_else(|| "eng".to_string()),
            tesseract_config: val.tesseract_config.map(Into::into),
        }
    }
}

#[napi(object)]
pub struct JsTesseractConfig {
    pub psm: Option<i32>,
    pub enable_table_detection: Option<bool>,
    pub tessedit_char_whitelist: Option<String>,
}

impl From<JsTesseractConfig> for RustTesseractConfig {
    fn from(val: JsTesseractConfig) -> Self {
        let mut config = RustTesseractConfig::default();
        if let Some(psm) = val.psm {
            config.psm = psm;
        }
        if let Some(enabled) = val.enable_table_detection {
            config.enable_table_detection = enabled;
        }
        if let Some(whitelist) = val.tessedit_char_whitelist {
            config.tessedit_char_whitelist = whitelist;
        }
        config
    }
}

/// Embedding model type configuration for Node.js bindings.
///
/// This struct represents different embedding model sources:
/// - `preset`: Use a named preset (e.g., "balanced", "fast", "quality", "multilingual")
/// - `fastembed`: Use a FastEmbed model with custom dimensions
/// - `custom`: Use a custom ONNX model
#[napi(object)]
pub struct JsEmbeddingModelType {
    /// Type of model: "preset", "fastembed", or "custom"
    pub model_type: String,
    /// For preset: preset name; for fastembed/custom: model ID
    pub value: String,
    /// Number of dimensions (only for fastembed/custom)
    pub dimensions: Option<u32>,
}

impl From<JsEmbeddingModelType> for RustEmbeddingModelType {
    fn from(val: JsEmbeddingModelType) -> Self {
        match val.model_type.as_str() {
            "preset" => RustEmbeddingModelType::Preset { name: val.value },
            "fastembed" => RustEmbeddingModelType::FastEmbed {
                model: val.value,
                dimensions: val.dimensions.unwrap_or(768) as usize,
            },
            "custom" => RustEmbeddingModelType::Custom {
                model_id: val.value,
                dimensions: val.dimensions.unwrap_or(512) as usize,
            },
            _ => RustEmbeddingModelType::Preset {
                name: "balanced".to_string(),
            },
        }
    }
}

/// Embedding generation configuration for Node.js bindings.
#[napi(object)]
pub struct JsEmbeddingConfig {
    /// Embedding model configuration
    pub model: Option<JsEmbeddingModelType>,
    /// Whether to normalize embeddings (L2 normalization)
    pub normalize: Option<bool>,
    /// Batch size for embedding generation
    pub batch_size: Option<u32>,
    /// Whether to show download progress for models
    pub show_download_progress: Option<bool>,
    /// Custom cache directory for model storage
    pub cache_dir: Option<String>,
}

impl From<JsEmbeddingConfig> for RustEmbeddingConfig {
    fn from(val: JsEmbeddingConfig) -> Self {
        RustEmbeddingConfig {
            model: val.model.map(Into::into).unwrap_or(RustEmbeddingModelType::Preset {
                name: "balanced".to_string(),
            }),
            normalize: val.normalize.unwrap_or(true),
            batch_size: val.batch_size.unwrap_or(32) as usize,
            show_download_progress: val.show_download_progress.unwrap_or(false),
            cache_dir: val.cache_dir.map(std::path::PathBuf::from),
        }
    }
}

#[napi(object)]
pub struct JsChunkingConfig {
    pub max_chars: Option<u32>,
    pub max_overlap: Option<u32>,
    /// Optional embedding configuration for generating embeddings
    pub embedding: Option<JsEmbeddingConfig>,
    /// Optional preset name for chunking parameters
    pub preset: Option<String>,
}

impl From<JsChunkingConfig> for RustChunkingConfig {
    fn from(val: JsChunkingConfig) -> Self {
        RustChunkingConfig {
            max_chars: val.max_chars.unwrap_or(1000) as usize,
            max_overlap: val.max_overlap.unwrap_or(200) as usize,
            embedding: val.embedding.map(Into::into),
            preset: val.preset,
        }
    }
}

#[napi(object)]
pub struct JsLanguageDetectionConfig {
    pub enabled: Option<bool>,
    pub min_confidence: Option<f64>,
    pub detect_multiple: Option<bool>,
}

impl From<JsLanguageDetectionConfig> for RustLanguageDetectionConfig {
    fn from(val: JsLanguageDetectionConfig) -> Self {
        RustLanguageDetectionConfig {
            enabled: val.enabled.unwrap_or(true),
            min_confidence: val.min_confidence.unwrap_or(0.8),
            detect_multiple: val.detect_multiple.unwrap_or(false),
        }
    }
}

#[napi(object)]
pub struct JsTokenReductionConfig {
    pub mode: Option<String>,
    pub preserve_important_words: Option<bool>,
}

impl From<JsTokenReductionConfig> for RustTokenReductionConfig {
    fn from(val: JsTokenReductionConfig) -> Self {
        RustTokenReductionConfig {
            mode: val.mode.unwrap_or_else(|| "off".to_string()),
            preserve_important_words: val.preserve_important_words.unwrap_or(true),
        }
    }
}

#[napi(object)]
pub struct JsPdfConfig {
    pub extract_images: Option<bool>,
    pub passwords: Option<Vec<String>>,
    pub extract_metadata: Option<bool>,
}

impl From<JsPdfConfig> for RustPdfConfig {
    fn from(val: JsPdfConfig) -> Self {
        RustPdfConfig {
            extract_images: val.extract_images.unwrap_or(false),
            passwords: val.passwords,
            extract_metadata: val.extract_metadata.unwrap_or(true),
        }
    }
}

#[napi(object)]
pub struct JsImageExtractionConfig {
    pub extract_images: Option<bool>,
    pub target_dpi: Option<i32>,
    pub max_image_dimension: Option<i32>,
    pub auto_adjust_dpi: Option<bool>,
    pub min_dpi: Option<i32>,
    pub max_dpi: Option<i32>,
}

impl From<JsImageExtractionConfig> for RustImageExtractionConfig {
    fn from(val: JsImageExtractionConfig) -> Self {
        RustImageExtractionConfig {
            extract_images: val.extract_images.unwrap_or(true),
            target_dpi: val.target_dpi.unwrap_or(300),
            max_image_dimension: val.max_image_dimension.unwrap_or(4096),
            auto_adjust_dpi: val.auto_adjust_dpi.unwrap_or(true),
            min_dpi: val.min_dpi.unwrap_or(72),
            max_dpi: val.max_dpi.unwrap_or(600),
        }
    }
}

#[napi(object)]
pub struct JsPostProcessorConfig {
    pub enabled: Option<bool>,
    pub enabled_processors: Option<Vec<String>>,
    pub disabled_processors: Option<Vec<String>>,
}

impl From<JsPostProcessorConfig> for RustPostProcessorConfig {
    fn from(val: JsPostProcessorConfig) -> Self {
        RustPostProcessorConfig {
            enabled: val.enabled.unwrap_or(true),
            enabled_processors: val.enabled_processors,
            disabled_processors: val.disabled_processors,
        }
    }
}

#[napi(object)]
#[derive(Clone)]
pub struct JsHtmlPreprocessingOptions {
    pub enabled: Option<bool>,
    pub preset: Option<String>,
    pub remove_navigation: Option<bool>,
    pub remove_forms: Option<bool>,
}

#[napi(object)]
#[derive(Clone)]
pub struct JsHtmlOptions {
    pub heading_style: Option<String>,
    pub list_indent_type: Option<String>,
    pub list_indent_width: Option<u32>,
    pub bullets: Option<String>,
    pub strong_em_symbol: Option<String>,
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
    pub wrap_width: Option<u32>,
    pub convert_as_inline: Option<bool>,
    pub sub_symbol: Option<String>,
    pub sup_symbol: Option<String>,
    pub newline_style: Option<String>,
    pub code_block_style: Option<String>,
    pub keep_inline_images_in: Option<Vec<String>>,
    pub encoding: Option<String>,
    pub debug: Option<bool>,
    pub strip_tags: Option<Vec<String>>,
    pub preserve_tags: Option<Vec<String>>,
    pub preprocessing: Option<JsHtmlPreprocessingOptions>,
}

#[napi(object)]
#[derive(Clone)]
pub struct JsYakeParams {
    pub window_size: Option<u32>,
}

#[napi(object)]
#[derive(Clone)]
pub struct JsRakeParams {
    pub min_word_length: Option<u32>,
    pub max_words_per_phrase: Option<u32>,
}

#[napi(object)]
#[derive(Clone)]
pub struct JsKeywordConfig {
    pub algorithm: Option<String>,
    pub max_keywords: Option<u32>,
    pub min_score: Option<f64>,
    #[napi(ts_type = "[number, number] | undefined")]
    pub ngram_range: Option<Vec<u32>>,
    pub language: Option<String>,
    pub yake_params: Option<JsYakeParams>,
    pub rake_params: Option<JsRakeParams>,
}

impl TryFrom<JsHtmlOptions> for ConversionOptions {
    type Error = Error;

    fn try_from(options: JsHtmlOptions) -> Result<Self> {
        let mut opts = ConversionOptions::default();

        if let Some(style) = options.heading_style {
            opts.heading_style = parse_heading_style(&style)?;
        }
        if let Some(indent) = options.list_indent_type {
            opts.list_indent_type = parse_list_indent_type(&indent)?;
        }
        if let Some(width) = options.list_indent_width {
            opts.list_indent_width = width as usize;
        }
        if let Some(bullets) = options.bullets {
            opts.bullets = bullets;
        }
        if let Some(symbol) = options.strong_em_symbol {
            let mut chars = symbol.chars();
            let first = chars.next().ok_or_else(|| {
                Error::new(
                    Status::InvalidArg,
                    "htmlOptions.strongEmSymbol must contain at least one character",
                )
            })?;
            opts.strong_em_symbol = first;
        }
        if let Some(value) = options.escape_asterisks {
            opts.escape_asterisks = value;
        }
        if let Some(value) = options.escape_underscores {
            opts.escape_underscores = value;
        }
        if let Some(value) = options.escape_misc {
            opts.escape_misc = value;
        }
        if let Some(value) = options.escape_ascii {
            opts.escape_ascii = value;
        }
        if let Some(language) = options.code_language {
            opts.code_language = language;
        }
        if let Some(value) = options.autolinks {
            opts.autolinks = value;
        }
        if let Some(value) = options.default_title {
            opts.default_title = value;
        }
        if let Some(value) = options.br_in_tables {
            opts.br_in_tables = value;
        }
        if let Some(value) = options.hocr_spatial_tables {
            opts.hocr_spatial_tables = value;
        }
        if let Some(style) = options.highlight_style {
            opts.highlight_style = parse_highlight_style(&style)?;
        }
        if let Some(value) = options.extract_metadata {
            opts.extract_metadata = value;
        }
        if let Some(mode) = options.whitespace_mode {
            opts.whitespace_mode = parse_whitespace_mode(&mode)?;
        }
        if let Some(value) = options.strip_newlines {
            opts.strip_newlines = value;
        }
        if let Some(value) = options.wrap {
            opts.wrap = value;
        }
        if let Some(width) = options.wrap_width {
            opts.wrap_width = width as usize;
        }
        if let Some(value) = options.convert_as_inline {
            opts.convert_as_inline = value;
        }
        if let Some(symbol) = options.sub_symbol {
            opts.sub_symbol = symbol;
        }
        if let Some(symbol) = options.sup_symbol {
            opts.sup_symbol = symbol;
        }
        if let Some(style) = options.newline_style {
            opts.newline_style = parse_newline_style(&style)?;
        }
        if let Some(style) = options.code_block_style {
            opts.code_block_style = parse_code_block_style(&style)?;
        }
        if let Some(tags) = options.keep_inline_images_in {
            opts.keep_inline_images_in = tags;
        }
        if let Some(encoding) = options.encoding {
            opts.encoding = encoding;
        }
        if let Some(debug) = options.debug {
            opts.debug = debug;
        }
        if let Some(tags) = options.strip_tags {
            opts.strip_tags = tags;
        }
        if let Some(tags) = options.preserve_tags {
            opts.preserve_tags = tags;
        }
        if let Some(pre) = options.preprocessing {
            let mut preprocessing = opts.preprocessing.clone();
            if let Some(enabled) = pre.enabled {
                preprocessing.enabled = enabled;
            }
            if let Some(preset) = pre.preset {
                preprocessing.preset = parse_preprocessing_preset(&preset)?;
            }
            if let Some(remove_navigation) = pre.remove_navigation {
                preprocessing.remove_navigation = remove_navigation;
            }
            if let Some(remove_forms) = pre.remove_forms {
                preprocessing.remove_forms = remove_forms;
            }
            opts.preprocessing = preprocessing;
        }

        Ok(opts)
    }
}

impl From<&HtmlPreprocessingOptions> for JsHtmlPreprocessingOptions {
    fn from(opts: &HtmlPreprocessingOptions) -> Self {
        Self {
            enabled: Some(opts.enabled),
            preset: Some(preprocessing_preset_to_string(opts.preset).to_string()),
            remove_navigation: Some(opts.remove_navigation),
            remove_forms: Some(opts.remove_forms),
        }
    }
}

impl From<&ConversionOptions> for JsHtmlOptions {
    fn from(opts: &ConversionOptions) -> Self {
        Self {
            heading_style: Some(heading_style_to_string(opts.heading_style).to_string()),
            list_indent_type: Some(list_indent_type_to_string(opts.list_indent_type).to_string()),
            list_indent_width: Some(opts.list_indent_width as u32),
            bullets: Some(opts.bullets.clone()),
            strong_em_symbol: Some(opts.strong_em_symbol.to_string()),
            escape_asterisks: Some(opts.escape_asterisks),
            escape_underscores: Some(opts.escape_underscores),
            escape_misc: Some(opts.escape_misc),
            escape_ascii: Some(opts.escape_ascii),
            code_language: Some(opts.code_language.clone()),
            autolinks: Some(opts.autolinks),
            default_title: Some(opts.default_title),
            br_in_tables: Some(opts.br_in_tables),
            hocr_spatial_tables: Some(opts.hocr_spatial_tables),
            highlight_style: Some(highlight_style_to_string(opts.highlight_style).to_string()),
            extract_metadata: Some(opts.extract_metadata),
            whitespace_mode: Some(whitespace_mode_to_string(opts.whitespace_mode).to_string()),
            strip_newlines: Some(opts.strip_newlines),
            wrap: Some(opts.wrap),
            wrap_width: Some(opts.wrap_width as u32),
            convert_as_inline: Some(opts.convert_as_inline),
            sub_symbol: Some(opts.sub_symbol.clone()),
            sup_symbol: Some(opts.sup_symbol.clone()),
            newline_style: Some(newline_style_to_string(opts.newline_style).to_string()),
            code_block_style: Some(code_block_style_to_string(opts.code_block_style).to_string()),
            keep_inline_images_in: Some(opts.keep_inline_images_in.clone()),
            encoding: Some(opts.encoding.clone()),
            debug: Some(opts.debug),
            strip_tags: Some(opts.strip_tags.clone()),
            preserve_tags: Some(opts.preserve_tags.clone()),
            preprocessing: Some(JsHtmlPreprocessingOptions::from(&opts.preprocessing)),
        }
    }
}

impl TryFrom<JsKeywordConfig> for RustKeywordConfig {
    type Error = Error;

    fn try_from(config: JsKeywordConfig) -> Result<Self> {
        let mut keywords = RustKeywordConfig::default();

        if let Some(max) = config.max_keywords {
            keywords.max_keywords = max as usize;
        }
        if let Some(score) = config.min_score {
            keywords.min_score = score as f32;
        }
        if let Some(range) = config.ngram_range {
            if range.len() != 2 {
                return Err(Error::new(
                    Status::InvalidArg,
                    "keywords.ngramRange must contain exactly two elements",
                ));
            }
            keywords.ngram_range = (range[0] as usize, range[1] as usize);
        }
        if let Some(language) = config.language {
            keywords.language = Some(language);
        }
        if let Some(algorithm) = config.algorithm {
            keywords.algorithm = parse_keyword_algorithm(&algorithm)?;
        }
        if let Some(yake) = config.yake_params {
            let mut params = RustYakeParams::default();
            if let Some(window) = yake.window_size {
                params.window_size = window as usize;
            }
            keywords.yake_params = Some(params);
        }
        if let Some(rake) = config.rake_params {
            let mut params = RustRakeParams::default();
            if let Some(min_len) = rake.min_word_length {
                params.min_word_length = min_len as usize;
            }
            if let Some(max_words) = rake.max_words_per_phrase {
                params.max_words_per_phrase = max_words as usize;
            }
            keywords.rake_params = Some(params);
        }

        Ok(keywords)
    }
}

impl From<RustKeywordConfig> for JsKeywordConfig {
    fn from(config: RustKeywordConfig) -> Self {
        Self {
            algorithm: Some(keyword_algorithm_to_string(config.algorithm).to_string()),
            max_keywords: Some(config.max_keywords as u32),
            min_score: Some(config.min_score as f64),
            ngram_range: Some(vec![config.ngram_range.0 as u32, config.ngram_range.1 as u32]),
            language: config.language,
            yake_params: config.yake_params.map(|params| JsYakeParams {
                window_size: Some(params.window_size as u32),
            }),
            rake_params: config.rake_params.map(|params| JsRakeParams {
                min_word_length: Some(params.min_word_length as u32),
                max_words_per_phrase: Some(params.max_words_per_phrase as u32),
            }),
        }
    }
}

fn parse_heading_style(value: &str) -> Result<HeadingStyle> {
    match value.to_lowercase().as_str() {
        "atx" => Ok(HeadingStyle::Atx),
        "underlined" => Ok(HeadingStyle::Underlined),
        "atx_closed" | "atx-closed" => Ok(HeadingStyle::AtxClosed),
        other => Err(Error::new(
            Status::InvalidArg,
            format!("Invalid htmlOptions.headingStyle '{}'", other),
        )),
    }
}

fn heading_style_to_string(style: HeadingStyle) -> &'static str {
    match style {
        HeadingStyle::Atx => "atx",
        HeadingStyle::Underlined => "underlined",
        HeadingStyle::AtxClosed => "atx_closed",
    }
}

fn parse_list_indent_type(value: &str) -> Result<ListIndentType> {
    match value.to_lowercase().as_str() {
        "spaces" => Ok(ListIndentType::Spaces),
        "tabs" => Ok(ListIndentType::Tabs),
        other => Err(Error::new(
            Status::InvalidArg,
            format!("Invalid htmlOptions.listIndentType '{}'", other),
        )),
    }
}

fn list_indent_type_to_string(value: ListIndentType) -> &'static str {
    match value {
        ListIndentType::Spaces => "spaces",
        ListIndentType::Tabs => "tabs",
    }
}

fn parse_highlight_style(value: &str) -> Result<HighlightStyle> {
    match value.to_lowercase().as_str() {
        "double_equal" | "==" | "double-equal" => Ok(HighlightStyle::DoubleEqual),
        "html" => Ok(HighlightStyle::Html),
        "bold" => Ok(HighlightStyle::Bold),
        "none" => Ok(HighlightStyle::None),
        other => Err(Error::new(
            Status::InvalidArg,
            format!("Invalid htmlOptions.highlightStyle '{}'", other),
        )),
    }
}

fn highlight_style_to_string(style: HighlightStyle) -> &'static str {
    match style {
        HighlightStyle::DoubleEqual => "double_equal",
        HighlightStyle::Html => "html",
        HighlightStyle::Bold => "bold",
        HighlightStyle::None => "none",
    }
}

fn parse_whitespace_mode(value: &str) -> Result<WhitespaceMode> {
    match value.to_lowercase().as_str() {
        "normalized" => Ok(WhitespaceMode::Normalized),
        "strict" => Ok(WhitespaceMode::Strict),
        other => Err(Error::new(
            Status::InvalidArg,
            format!("Invalid htmlOptions.whitespaceMode '{}'", other),
        )),
    }
}

fn whitespace_mode_to_string(mode: WhitespaceMode) -> &'static str {
    match mode {
        WhitespaceMode::Normalized => "normalized",
        WhitespaceMode::Strict => "strict",
    }
}

fn parse_newline_style(value: &str) -> Result<NewlineStyle> {
    match value.to_lowercase().as_str() {
        "spaces" => Ok(NewlineStyle::Spaces),
        "backslash" => Ok(NewlineStyle::Backslash),
        other => Err(Error::new(
            Status::InvalidArg,
            format!("Invalid htmlOptions.newlineStyle '{}'", other),
        )),
    }
}

fn newline_style_to_string(value: NewlineStyle) -> &'static str {
    match value {
        NewlineStyle::Spaces => "spaces",
        NewlineStyle::Backslash => "backslash",
    }
}

fn parse_code_block_style(value: &str) -> Result<CodeBlockStyle> {
    match value.to_lowercase().as_str() {
        "indented" => Ok(CodeBlockStyle::Indented),
        "backticks" => Ok(CodeBlockStyle::Backticks),
        "tildes" => Ok(CodeBlockStyle::Tildes),
        other => Err(Error::new(
            Status::InvalidArg,
            format!("Invalid htmlOptions.codeBlockStyle '{}'", other),
        )),
    }
}

fn code_block_style_to_string(value: CodeBlockStyle) -> &'static str {
    match value {
        CodeBlockStyle::Indented => "indented",
        CodeBlockStyle::Backticks => "backticks",
        CodeBlockStyle::Tildes => "tildes",
    }
}

fn parse_preprocessing_preset(value: &str) -> Result<PreprocessingPreset> {
    match value.to_lowercase().as_str() {
        "minimal" => Ok(PreprocessingPreset::Minimal),
        "standard" => Ok(PreprocessingPreset::Standard),
        "aggressive" => Ok(PreprocessingPreset::Aggressive),
        other => Err(Error::new(
            Status::InvalidArg,
            format!("Invalid htmlOptions.preprocessing.preset '{}'", other),
        )),
    }
}

fn preprocessing_preset_to_string(preset: PreprocessingPreset) -> &'static str {
    match preset {
        PreprocessingPreset::Minimal => "minimal",
        PreprocessingPreset::Standard => "standard",
        PreprocessingPreset::Aggressive => "aggressive",
    }
}

fn parse_keyword_algorithm(value: &str) -> Result<RustKeywordAlgorithm> {
    match value.to_lowercase().as_str() {
        "yake" => Ok(RustKeywordAlgorithm::Yake),
        "rake" => Ok(RustKeywordAlgorithm::Rake),
        other => Err(Error::new(
            Status::InvalidArg,
            format!("Invalid keywords.algorithm '{}'. Expected 'yake' or 'rake'", other),
        )),
    }
}

fn keyword_algorithm_to_string(algo: RustKeywordAlgorithm) -> &'static str {
    match algo {
        RustKeywordAlgorithm::Yake => "yake",
        RustKeywordAlgorithm::Rake => "rake",
    }
}

#[napi(object)]
pub struct JsExtractionConfig {
    pub use_cache: Option<bool>,
    pub enable_quality_processing: Option<bool>,
    pub ocr: Option<JsOcrConfig>,
    pub force_ocr: Option<bool>,
    pub chunking: Option<JsChunkingConfig>,
    pub images: Option<JsImageExtractionConfig>,
    pub pdf_options: Option<JsPdfConfig>,
    pub token_reduction: Option<JsTokenReductionConfig>,
    pub language_detection: Option<JsLanguageDetectionConfig>,
    pub postprocessor: Option<JsPostProcessorConfig>,
    pub keywords: Option<JsKeywordConfig>,
    pub html_options: Option<JsHtmlOptions>,
    pub max_concurrent_extractions: Option<u32>,
}

impl TryFrom<JsExtractionConfig> for ExtractionConfig {
    type Error = Error;

    fn try_from(val: JsExtractionConfig) -> Result<Self> {
        let html_options = match val.html_options {
            Some(options) => Some(ConversionOptions::try_from(options)?),
            None => None,
        };

        let keywords = match val.keywords {
            Some(config) => Some(RustKeywordConfig::try_from(config)?),
            None => None,
        };

        Ok(ExtractionConfig {
            use_cache: val.use_cache.unwrap_or(true),
            enable_quality_processing: val.enable_quality_processing.unwrap_or(true),
            ocr: val.ocr.map(Into::into),
            force_ocr: val.force_ocr.unwrap_or(false),
            chunking: val.chunking.map(Into::into),
            images: val.images.map(Into::into),
            pdf_options: val.pdf_options.map(Into::into),
            token_reduction: val.token_reduction.map(Into::into),
            language_detection: val.language_detection.map(Into::into),
            keywords,
            postprocessor: val.postprocessor.map(Into::into),
            html_options,
            max_concurrent_extractions: val.max_concurrent_extractions.map(|v| v as usize),
        })
    }
}

impl TryFrom<ExtractionConfig> for JsExtractionConfig {
    type Error = napi::Error;

    fn try_from(val: ExtractionConfig) -> Result<Self> {
        Ok(JsExtractionConfig {
            use_cache: Some(val.use_cache),
            enable_quality_processing: Some(val.enable_quality_processing),
            ocr: val.ocr.map(|ocr| JsOcrConfig {
                backend: ocr.backend,
                language: Some(ocr.language),
                tesseract_config: ocr.tesseract_config.map(|tc| JsTesseractConfig {
                    psm: Some(tc.psm),
                    enable_table_detection: Some(tc.enable_table_detection),
                    tessedit_char_whitelist: if tc.tessedit_char_whitelist.is_empty() {
                        None
                    } else {
                        Some(tc.tessedit_char_whitelist)
                    },
                }),
            }),
            force_ocr: Some(val.force_ocr),
            chunking: val.chunking.map(|chunk| JsChunkingConfig {
                max_chars: Some(chunk.max_chars as u32),
                max_overlap: Some(chunk.max_overlap as u32),
                embedding: chunk.embedding.map(|emb| JsEmbeddingConfig {
                    model: Some(JsEmbeddingModelType {
                        model_type: match emb.model {
                            RustEmbeddingModelType::Preset { .. } => "preset".to_string(),
                            RustEmbeddingModelType::FastEmbed { .. } => "fastembed".to_string(),
                            RustEmbeddingModelType::Custom { .. } => "custom".to_string(),
                        },
                        value: match &emb.model {
                            RustEmbeddingModelType::Preset { name } => name.clone(),
                            RustEmbeddingModelType::FastEmbed { model, .. } => model.clone(),
                            RustEmbeddingModelType::Custom { model_id, .. } => model_id.clone(),
                        },
                        dimensions: match emb.model {
                            RustEmbeddingModelType::FastEmbed { dimensions, .. } => Some(dimensions as u32),
                            RustEmbeddingModelType::Custom { dimensions, .. } => Some(dimensions as u32),
                            _ => None,
                        },
                    }),
                    normalize: Some(emb.normalize),
                    batch_size: Some(emb.batch_size as u32),
                    show_download_progress: Some(emb.show_download_progress),
                    cache_dir: emb.cache_dir.and_then(|p| p.to_str().map(String::from)),
                }),
                preset: chunk.preset,
            }),
            images: val.images.map(|img| JsImageExtractionConfig {
                extract_images: Some(img.extract_images),
                target_dpi: Some(img.target_dpi),
                max_image_dimension: Some(img.max_image_dimension),
                auto_adjust_dpi: Some(img.auto_adjust_dpi),
                min_dpi: Some(img.min_dpi),
                max_dpi: Some(img.max_dpi),
            }),
            pdf_options: val.pdf_options.map(|pdf| JsPdfConfig {
                extract_images: Some(pdf.extract_images),
                passwords: pdf.passwords,
                extract_metadata: Some(pdf.extract_metadata),
            }),
            token_reduction: val.token_reduction.map(|tr| JsTokenReductionConfig {
                mode: Some(tr.mode),
                preserve_important_words: Some(tr.preserve_important_words),
            }),
            language_detection: val.language_detection.map(|ld| JsLanguageDetectionConfig {
                enabled: Some(ld.enabled),
                min_confidence: Some(ld.min_confidence),
                detect_multiple: Some(ld.detect_multiple),
            }),
            postprocessor: val.postprocessor.map(|pp| JsPostProcessorConfig {
                enabled: Some(pp.enabled),
                enabled_processors: pp.enabled_processors,
                disabled_processors: pp.disabled_processors,
            }),
            keywords: val.keywords.map(JsKeywordConfig::from),
            html_options: val.html_options.as_ref().map(JsHtmlOptions::from),
            max_concurrent_extractions: val.max_concurrent_extractions.map(|v| v as u32),
        })
    }
}

/// Load extraction configuration from a file.
///
/// Automatically detects the file format based on extension:
/// - `.toml` - TOML format
/// - `.yaml` or `.yml` - YAML format
/// - `.json` - JSON format
///
/// # Parameters
///
/// * `file_path` - Path to the configuration file (absolute or relative)
///
/// # Returns
///
/// `JsExtractionConfig` object with loaded configuration.
///
/// # Errors
///
/// Throws an error if:
/// - File does not exist or is not accessible
/// - File content is not valid TOML/YAML/JSON
/// - Configuration structure is invalid
///
/// # Example
///
/// ```typescript
/// import { loadExtractionConfigFromFile } from '@goldziher/kreuzberg';
///
/// // Load from TOML file
/// const config = loadExtractionConfigFromFile('kreuzberg.toml');
///
/// // Load from YAML file
/// const config2 = loadExtractionConfigFromFile('./config.yaml');
///
/// // Use with extraction
/// const result = await extractFile('document.pdf', null, config);
/// ```
#[napi(js_name = "loadExtractionConfigFromFile")]
pub fn load_extraction_config_from_file(file_path: String) -> Result<JsExtractionConfig> {
    let path = std::path::Path::new(&file_path);

    let ext = path.extension().and_then(|e| e.to_str()).ok_or_else(|| {
        Error::new(
            Status::InvalidArg,
            "File path must have an extension (.toml, .yaml, .json)",
        )
    })?;

    let rust_config = match ext.to_lowercase().as_str() {
        "toml" => ExtractionConfig::from_toml_file(path).map_err(convert_error)?,
        "yaml" | "yml" => ExtractionConfig::from_yaml_file(path).map_err(convert_error)?,
        "json" => ExtractionConfig::from_json_file(path).map_err(convert_error)?,
        _ => {
            return Err(Error::new(
                Status::InvalidArg,
                format!(
                    "Unsupported file extension: '{}'. Supported: .toml, .yaml, .yml, .json",
                    ext
                ),
            ));
        }
    };

    JsExtractionConfig::try_from(rust_config)
}

#[napi(object)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct JsTable {
    pub cells: Vec<Vec<String>>,
    pub markdown: String,
    pub page_number: u32,
}

#[napi(object)]
pub struct JsExtractedImage {
    pub data: Buffer,
    pub format: String,
    pub image_index: u32,
    pub page_number: Option<u32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub colorspace: Option<String>,
    pub bits_per_component: Option<u32>,
    pub is_mask: bool,
    pub description: Option<String>,
    #[napi(ts_type = "JsExtractionResult | undefined")]
    pub ocr_result: Option<serde_json::Value>,
}

#[napi(object)]
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct JsChunkMetadata {
    pub char_start: u32,
    pub char_end: u32,
    pub token_count: Option<u32>,
    pub chunk_index: u32,
    pub total_chunks: u32,
}

#[napi(object)]
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct JsChunk {
    pub content: String,
    #[napi(ts_type = "number[] | undefined")]
    pub embedding: Option<Vec<f32>>,
    pub metadata: JsChunkMetadata,
}

fn usize_to_u32(value: usize, field: &str) -> Result<u32> {
    u32::try_from(value).map_err(|_| {
        Error::new(
            Status::InvalidArg,
            format!("{} exceeds supported range (must fit in u32)", field),
        )
    })
}

fn resolve_config(config: Option<JsExtractionConfig>) -> Result<ExtractionConfig> {
    match config {
        Some(cfg) => ExtractionConfig::try_from(cfg),
        None => Ok(ExtractionConfig::default()),
    }
}

#[napi(object)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct JsExtractionResult {
    pub content: String,
    pub mime_type: String,
    #[napi(ts_type = "Metadata")]
    pub metadata: serde_json::Value,
    pub tables: Vec<JsTable>,
    pub detected_languages: Option<Vec<String>>,
    pub chunks: Option<Vec<JsChunk>>,
    #[serde(skip)]
    pub images: Option<Vec<JsExtractedImage>>,
}

impl TryFrom<RustExtractionResult> for JsExtractionResult {
    type Error = napi::Error;

    fn try_from(val: RustExtractionResult) -> Result<Self> {
        let metadata = serde_json::to_value(&val.metadata)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to serialize metadata: {}", e)))?;

        let images = if let Some(imgs) = val.images {
            let mut js_images = Vec::with_capacity(imgs.len());
            for img in imgs {
                let ocr_result = if let Some(ocr) = img.ocr_result {
                    Some(JsExtractionResult::try_from(*ocr).and_then(|js_res| {
                        serde_json::to_value(js_res).map_err(|e| {
                            Error::new(
                                Status::GenericFailure,
                                format!("Failed to serialize OCR result metadata: {}", e),
                            )
                        })
                    })?)
                } else {
                    None
                };

                js_images.push(JsExtractedImage {
                    data: img.data.into(),
                    format: img.format,
                    image_index: img.image_index as u32,
                    page_number: img.page_number.map(|p| p as u32),
                    width: img.width,
                    height: img.height,
                    colorspace: img.colorspace,
                    bits_per_component: img.bits_per_component,
                    is_mask: img.is_mask,
                    description: img.description,
                    ocr_result,
                });
            }
            Some(js_images)
        } else {
            None
        };

        Ok(JsExtractionResult {
            content: val.content,
            mime_type: val.mime_type,
            metadata,
            tables: val
                .tables
                .into_iter()
                .map(|t| JsTable {
                    cells: t.cells,
                    markdown: t.markdown,
                    page_number: t.page_number as u32,
                })
                .collect(),
            detected_languages: val.detected_languages,
            chunks: if let Some(chunks) = val.chunks {
                let mut js_chunks = Vec::with_capacity(chunks.len());
                for chunk in chunks {
                    let metadata = JsChunkMetadata {
                        char_start: usize_to_u32(chunk.metadata.char_start, "chunks[].metadata.char_start")?,
                        char_end: usize_to_u32(chunk.metadata.char_end, "chunks[].metadata.char_end")?,
                        token_count: match chunk.metadata.token_count {
                            Some(tokens) => Some(usize_to_u32(tokens, "chunks[].metadata.token_count")?),
                            None => None,
                        },
                        chunk_index: usize_to_u32(chunk.metadata.chunk_index, "chunks[].metadata.chunk_index")?,
                        total_chunks: usize_to_u32(chunk.metadata.total_chunks, "chunks[].metadata.total_chunks")?,
                    };

                    js_chunks.push(JsChunk {
                        content: chunk.content,
                        embedding: chunk.embedding,
                        metadata,
                    });
                }
                Some(js_chunks)
            } else {
                None
            },
            images,
        })
    }
}

impl TryFrom<JsExtractionResult> for RustExtractionResult {
    type Error = napi::Error;

    fn try_from(val: JsExtractionResult) -> Result<Self> {
        let metadata = {
            let mut metadata_map: std::collections::HashMap<String, serde_json::Value> =
                serde_json::from_value(val.metadata.clone()).map_err(|e| {
                    Error::new(
                        Status::GenericFailure,
                        format!("Failed to parse metadata as map: {}", e),
                    )
                })?;

            let language = metadata_map
                .remove("language")
                .and_then(|v| serde_json::from_value(v).ok());
            let date = metadata_map.remove("date").and_then(|v| serde_json::from_value(v).ok());
            let subject = metadata_map
                .remove("subject")
                .and_then(|v| serde_json::from_value(v).ok());
            let image_preprocessing = metadata_map
                .remove("image_preprocessing")
                .and_then(|v| serde_json::from_value(v).ok());
            let json_schema = metadata_map.remove("json_schema");
            let error = metadata_map
                .remove("error")
                .and_then(|v| serde_json::from_value(v).ok());

            let known_format_fields: std::collections::HashSet<&str> = [
                "format_type",
                "title",
                "author",
                "keywords",
                "creator",
                "producer",
                "creation_date",
                "modification_date",
                "page_count",
                "sheet_count",
                "sheet_names",
                "from_email",
                "from_name",
                "to_emails",
                "cc_emails",
                "bcc_emails",
                "message_id",
                "attachments",
                "description",
                "summary",
                "fonts",
                "format",
                "file_count",
                "file_list",
                "total_size",
                "compressed_size",
                "width",
                "height",
                "exif",
                "element_count",
                "unique_elements",
                "line_count",
                "word_count",
                "character_count",
                "headers",
                "links",
                "code_blocks",
                "canonical",
                "base_href",
                "og_title",
                "og_description",
                "og_image",
                "og_url",
                "og_type",
                "og_site_name",
                "twitter_card",
                "twitter_title",
                "twitter_description",
                "twitter_image",
                "twitter_site",
                "twitter_creator",
                "link_author",
                "link_license",
                "link_alternate",
                "psm",
                "output_format",
                "table_count",
                "table_rows",
                "table_cols",
            ]
            .iter()
            .copied()
            .collect();

            let mut format_fields = serde_json::Map::new();
            for key in known_format_fields.iter() {
                if let Some(value) = metadata_map.remove(*key) {
                    format_fields.insert(key.to_string(), value);
                }
            }

            let format = if !format_fields.is_empty() {
                serde_json::from_value(serde_json::Value::Object(format_fields)).ok()
            } else {
                None
            };

            let additional = metadata_map;

            kreuzberg::Metadata {
                language,
                date,
                subject,
                format,
                image_preprocessing,
                json_schema,
                error,
                additional,
            }
        };

        let images = if let Some(imgs) = val.images {
            let mut rust_images = Vec::with_capacity(imgs.len());
            for img in imgs {
                let ocr_result = if let Some(json) = img.ocr_result {
                    Some(Box::new(
                        serde_json::from_value::<JsExtractionResult>(json)
                            .map_err(|e| {
                                Error::new(
                                    Status::GenericFailure,
                                    format!("Failed to deserialize OCR result: {}", e),
                                )
                            })
                            .and_then(RustExtractionResult::try_from)?,
                    ))
                } else {
                    None
                };

                rust_images.push(kreuzberg::ExtractedImage {
                    data: img.data.to_vec(),
                    format: img.format,
                    image_index: img.image_index as usize,
                    page_number: img.page_number.map(|p| p as usize),
                    width: img.width,
                    height: img.height,
                    colorspace: img.colorspace,
                    bits_per_component: img.bits_per_component,
                    is_mask: img.is_mask,
                    description: img.description,
                    ocr_result,
                });
            }
            Some(rust_images)
        } else {
            None
        };

        Ok(RustExtractionResult {
            content: val.content,
            mime_type: val.mime_type,
            metadata,
            tables: val
                .tables
                .into_iter()
                .map(|t| kreuzberg::Table {
                    cells: t.cells,
                    markdown: t.markdown,
                    page_number: t.page_number as usize,
                })
                .collect(),
            detected_languages: val.detected_languages,
            chunks: val.chunks.map(|chunks| {
                chunks
                    .into_iter()
                    .map(|chunk| RustChunk {
                        content: chunk.content,
                        embedding: chunk.embedding,
                        metadata: RustChunkMetadata {
                            char_start: chunk.metadata.char_start as usize,
                            char_end: chunk.metadata.char_end as usize,
                            token_count: chunk.metadata.token_count.map(|v| v as usize),
                            chunk_index: chunk.metadata.chunk_index as usize,
                            total_chunks: chunk.metadata.total_chunks as usize,
                        },
                    })
                    .collect()
            }),
            images,
        })
    }
}

/// Extract content from a file (synchronous).
///
/// Synchronously extracts text, tables, images, and metadata from a document file.
/// Supports 118+ file formats including PDFs, Office documents, images, and more.
///
/// # Parameters
///
/// * `file_path` - Path to the file to extract (absolute or relative)
/// * `mime_type` - Optional MIME type hint (auto-detected if omitted)
/// * `config` - Optional extraction configuration (OCR, chunking, etc.)
///
/// # Returns
///
/// `ExtractionResult` containing:
/// - `content`: Extracted text content
/// - `mimeType`: Detected MIME type
/// - `metadata`: File metadata (author, title, etc.)
/// - `tables`: Extracted tables (if any)
/// - `images`: Extracted images (if configured)
/// - `chunks`: Text chunks (if chunking enabled)
/// - `detectedLanguages`: Detected languages (if enabled)
///
/// # Errors
///
/// Throws an error if:
/// - File does not exist or is not accessible
/// - File format is unsupported
/// - File is corrupted or malformed
/// - OCR processing fails (if enabled)
///
/// # Example
///
/// ```typescript
/// import { extractFileSync, ExtractionConfig } from '@kreuzberg/node';
///
/// // Basic extraction
/// const result = extractFileSync('document.pdf', null, null);
/// console.log(result.content);
///
/// // With MIME type hint
/// const result2 = extractFileSync('file.bin', 'application/pdf', null);
///
/// // With OCR enabled
/// const config: ExtractionConfig = {
///   ocr: {
///     backend: 'tesseract',
///     language: 'eng',
///   }
/// };
/// const result3 = extractFileSync('scanned.pdf', null, config);
/// ```
#[napi]
pub fn extract_file_sync(
    file_path: String,
    mime_type: Option<String>,
    config: Option<JsExtractionConfig>,
) -> Result<JsExtractionResult> {
    let rust_config = resolve_config(config)?;

    kreuzberg::extract_file_sync(&file_path, mime_type.as_deref(), &rust_config)
        .map_err(convert_error)
        .and_then(JsExtractionResult::try_from)
}

/// Extract content from a file (asynchronous).
///
/// Asynchronously extracts text, tables, images, and metadata from a document file.
/// Non-blocking alternative to `extractFileSync` for use in async/await contexts.
///
/// # Parameters
///
/// * `file_path` - Path to the file to extract (absolute or relative)
/// * `mime_type` - Optional MIME type hint (auto-detected if omitted)
/// * `config` - Optional extraction configuration (OCR, chunking, etc.)
///
/// # Returns
///
/// Promise resolving to `ExtractionResult` with extracted content and metadata.
///
/// # Errors
///
/// Rejects if file processing fails (see `extractFileSync` for error conditions).
///
/// # Example
///
/// ```typescript
/// import { extractFile } from '@kreuzberg/node';
///
/// // Async/await usage
/// const result = await extractFile('document.pdf', null, null);
/// console.log(result.content);
///
/// // Promise usage
/// extractFile('report.docx', null, null)
///   .then(result => console.log(result.content))
///   .catch(err => console.error(err));
/// ```
#[napi]
pub async fn extract_file(
    file_path: String,
    mime_type: Option<String>,
    config: Option<JsExtractionConfig>,
) -> Result<JsExtractionResult> {
    let rust_config = resolve_config(config)?;

    kreuzberg::extract_file(&file_path, mime_type.as_deref(), &rust_config)
        .await
        .map_err(convert_error)
        .and_then(JsExtractionResult::try_from)
}

/// Extract content from bytes (synchronous).
///
/// Synchronously extracts content from a byte buffer without requiring a file path.
/// Useful for processing in-memory data, network streams, or database BLOBs.
///
/// # Parameters
///
/// * `data` - Buffer containing the document bytes
/// * `mime_type` - MIME type of the data (e.g., "application/pdf", "image/png")
/// * `config` - Optional extraction configuration
///
/// # Returns
///
/// `ExtractionResult` with extracted content and metadata.
///
/// # Errors
///
/// Throws an error if data is malformed or MIME type is unsupported.
///
/// # Example
///
/// ```typescript
/// import { extractBytesSync } from '@kreuzberg/node';
/// import fs from 'fs';
///
/// const buffer = fs.readFileSync('document.pdf');
/// const result = extractBytesSync(buffer, 'application/pdf', null);
/// console.log(result.content);
/// ```
#[napi]
pub fn extract_bytes_sync(
    data: Buffer,
    mime_type: String,
    config: Option<JsExtractionConfig>,
) -> Result<JsExtractionResult> {
    let rust_config = resolve_config(config)?;

    let owned_data = data.to_vec();

    kreuzberg::extract_bytes_sync(&owned_data, &mime_type, &rust_config)
        .map_err(convert_error)
        .and_then(JsExtractionResult::try_from)
}

/// Extract content from bytes (asynchronous).
///
/// Asynchronously extracts content from a byte buffer. Non-blocking alternative
/// to `extractBytesSync` for processing in-memory data.
///
/// # Parameters
///
/// * `data` - Buffer containing the document bytes
/// * `mime_type` - MIME type of the data
/// * `config` - Optional extraction configuration
///
/// # Returns
///
/// Promise resolving to `ExtractionResult`.
///
/// # Example
///
/// ```typescript
/// import { extractBytes } from '@kreuzberg/node';
///
/// const response = await fetch('https://example.com/document.pdf');
/// const buffer = Buffer.from(await response.arrayBuffer());
/// const result = await extractBytes(buffer, 'application/pdf', null);
/// ```
#[napi]
pub async fn extract_bytes(
    data: Buffer,
    mime_type: String,
    config: Option<JsExtractionConfig>,
) -> Result<JsExtractionResult> {
    let rust_config = resolve_config(config)?;
    let owned_data = data.to_vec();
    #[cfg(debug_assertions)]
    {
        if std::env::var("KREUZBERG_DEBUG_GUTEN").as_deref() == Ok("1") && mime_type.starts_with("image/") {
            let header: Vec<u8> = owned_data.iter().take(8).copied().collect();
            eprintln!("[Rust Binding] Debug input header: {:?}", header);
        }
    }

    kreuzberg::extract_bytes(&owned_data, &mime_type, &rust_config)
        .await
        .map_err(convert_error)
        .and_then(JsExtractionResult::try_from)
}

/// Batch extract from multiple files (synchronous).
///
/// Synchronously processes multiple files in parallel using Rayon. Significantly
/// faster than sequential processing for large batches.
///
/// # Parameters
///
/// * `paths` - Array of file paths to extract
/// * `config` - Optional extraction configuration (applied to all files)
///
/// # Returns
///
/// Array of `ExtractionResult` in the same order as input paths.
///
/// # Example
///
/// ```typescript
/// import { batchExtractFilesSync } from '@kreuzberg/node';
///
/// const files = ['doc1.pdf', 'doc2.docx', 'doc3.txt'];
/// const results = batchExtractFilesSync(files, null);
/// results.forEach((result, i) => {
///   console.log(`File ${files[i]}: ${result.content.substring(0, 100)}...`);
/// });
/// ```
#[napi]
pub fn batch_extract_files_sync(
    paths: Vec<String>,
    config: Option<JsExtractionConfig>,
) -> Result<Vec<JsExtractionResult>> {
    let rust_config = resolve_config(config)?;

    kreuzberg::batch_extract_file_sync(paths, &rust_config)
        .map_err(convert_error)
        .and_then(|results| results.into_iter().map(JsExtractionResult::try_from).collect())
}

/// Batch extract from multiple files (asynchronous).
///
/// Asynchronously processes multiple files in parallel. Non-blocking alternative
/// to `batchExtractFilesSync` with same performance benefits.
///
/// # Parameters
///
/// * `paths` - Array of file paths to extract
/// * `config` - Optional extraction configuration (applied to all files)
///
/// # Returns
///
/// Promise resolving to array of `ExtractionResult`.
///
/// # Example
///
/// ```typescript
/// import { batchExtractFiles } from '@kreuzberg/node';
///
/// const files = ['report1.pdf', 'report2.pdf', 'report3.pdf'];
/// const results = await batchExtractFiles(files, null);
/// console.log(`Processed ${results.length} files`);
/// ```
#[napi]
pub async fn batch_extract_files(
    paths: Vec<String>,
    config: Option<JsExtractionConfig>,
) -> Result<Vec<JsExtractionResult>> {
    let rust_config = resolve_config(config)?;

    kreuzberg::batch_extract_file(paths, &rust_config)
        .await
        .map_err(convert_error)
        .and_then(|results| results.into_iter().map(JsExtractionResult::try_from).collect())
}

/// Batch extract from multiple byte arrays (synchronous).
///
/// Synchronously processes multiple in-memory buffers in parallel. Requires
/// corresponding MIME types for each buffer.
///
/// # Parameters
///
/// * `data_list` - Array of buffers to extract
/// * `mime_types` - Array of MIME types (must match data_list length)
/// * `config` - Optional extraction configuration
///
/// # Returns
///
/// Array of `ExtractionResult` in the same order as inputs.
///
/// # Errors
///
/// Throws if data_list and mime_types lengths don't match.
///
/// # Example
///
/// ```typescript
/// import { batchExtractBytesSync } from '@kreuzberg/node';
///
/// const buffers = [buffer1, buffer2, buffer3];
/// const mimeTypes = ['application/pdf', 'image/png', 'text/plain'];
/// const results = batchExtractBytesSync(buffers, mimeTypes, null);
/// ```
#[napi]
pub fn batch_extract_bytes_sync(
    data_list: Vec<Buffer>,
    mime_types: Vec<String>,
    config: Option<JsExtractionConfig>,
) -> Result<Vec<JsExtractionResult>> {
    let rust_config = resolve_config(config)?;

    let owned_data: Vec<Vec<u8>> = data_list.iter().map(|b| b.to_vec()).collect();

    let contents: Vec<(&[u8], &str)> = owned_data
        .iter()
        .zip(mime_types.iter())
        .map(|(data, mime)| (data.as_slice(), mime.as_str()))
        .collect();

    kreuzberg::batch_extract_bytes_sync(contents, &rust_config)
        .map_err(convert_error)
        .and_then(|results| results.into_iter().map(JsExtractionResult::try_from).collect())
}

/// Batch extract from multiple byte arrays (asynchronous).
///
/// Asynchronously processes multiple in-memory buffers in parallel. Non-blocking
/// alternative to `batchExtractBytesSync`.
///
/// # Parameters
///
/// * `data_list` - Array of buffers to extract
/// * `mime_types` - Array of MIME types (must match data_list length)
/// * `config` - Optional extraction configuration
///
/// # Returns
///
/// Promise resolving to array of `ExtractionResult`.
///
/// # Example
///
/// ```typescript
/// import { batchExtractBytes } from '@kreuzberg/node';
///
/// const responses = await Promise.all([
///   fetch('https://example.com/doc1.pdf'),
///   fetch('https://example.com/doc2.pdf')
/// ]);
/// const buffers = await Promise.all(
///   responses.map(r => r.arrayBuffer().then(b => Buffer.from(b)))
/// );
/// const results = await batchExtractBytes(
///   buffers,
///   ['application/pdf', 'application/pdf'],
///   null
/// );
/// ```
#[napi]
pub async fn batch_extract_bytes(
    data_list: Vec<Buffer>,
    mime_types: Vec<String>,
    config: Option<JsExtractionConfig>,
) -> Result<Vec<JsExtractionResult>> {
    let rust_config = resolve_config(config)?;

    let owned_data: Vec<Vec<u8>> = data_list.iter().map(|b| b.to_vec()).collect();

    let contents: Vec<(&[u8], &str)> = owned_data
        .iter()
        .zip(mime_types.iter())
        .map(|(data, mime)| (data.as_slice(), mime.as_str()))
        .collect();

    kreuzberg::batch_extract_bytes(contents, &rust_config)
        .await
        .map_err(convert_error)
        .and_then(|results| results.into_iter().map(JsExtractionResult::try_from).collect())
}

use async_trait::async_trait;
use base64::Engine;
use kreuzberg::plugins::{Plugin, PostProcessor as RustPostProcessor, ProcessingStage};
use napi::bindgen_prelude::Promise;
use napi::threadsafe_function::ThreadsafeFunction;
use std::sync::Arc;

/// Wrapper that makes a JavaScript PostProcessor usable from Rust.
///
/// Uses JSON serialization to pass data between Rust and JavaScript due to NAPI limitations
/// with complex object types across ThreadsafeFunction boundaries.
///
/// Wrapper that holds the ThreadsafeFunction to call JavaScript from Rust.
/// The process_fn is an async JavaScript function that:
/// - Takes: String (JSON-serialized ExtractionResult)
/// - Returns: Promise<String> (JSON-serialized ExtractionResult)
///
/// Type parameters:
/// - Input: String
/// - Return: Promise<String>
/// - CallJsBackArgs: Vec<String> (because build_callback returns vec![value])
/// - ErrorStatus: napi::Status
/// - CalleeHandled: false (default with build_callback)
struct JsPostProcessor {
    name: String,
    process_fn: Arc<ThreadsafeFunction<String, Promise<String>, Vec<String>, napi::Status, false>>,
    stage: ProcessingStage,
}

unsafe impl Send for JsPostProcessor {}
unsafe impl Sync for JsPostProcessor {}

impl Plugin for JsPostProcessor {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }

    fn initialize(&self) -> std::result::Result<(), kreuzberg::KreuzbergError> {
        Ok(())
    }

    fn shutdown(&self) -> std::result::Result<(), kreuzberg::KreuzbergError> {
        Ok(())
    }
}

#[async_trait]
impl RustPostProcessor for JsPostProcessor {
    async fn process(
        &self,
        result: &mut kreuzberg::ExtractionResult,
        _config: &kreuzberg::ExtractionConfig,
    ) -> std::result::Result<(), kreuzberg::KreuzbergError> {
        eprintln!("\n[POST-PROCESSOR] === Starting JS Post-Processor '{}' ===", self.name);
        eprintln!(
            "[POST-PROCESSOR] Original Rust metadata.additional keys: {:?}",
            result.metadata.additional.keys().collect::<Vec<_>>()
        );

        let js_result =
            JsExtractionResult::try_from(result.clone()).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                message: format!("Failed to convert result for JavaScript PostProcessor: {}", e),
                plugin_name: self.name.clone(),
            })?;
        let json_input = serde_json::to_string(&js_result).map_err(|e| kreuzberg::KreuzbergError::Plugin {
            message: format!("Failed to serialize result for JavaScript PostProcessor: {}", e),
            plugin_name: self.name.clone(),
        })?;

        eprintln!(
            "[POST-PROCESSOR] JSON being sent to JS (first 500 chars): {}",
            &json_input.chars().take(500).collect::<String>()
        );

        let json_output = self
            .process_fn
            .call_async(json_input)
            .await
            .map_err(|e| kreuzberg::KreuzbergError::Plugin {
                message: format!("JavaScript PostProcessor '{}' call failed: {}", self.name, e),
                plugin_name: self.name.clone(),
            })?
            .await
            .map_err(|e| kreuzberg::KreuzbergError::Plugin {
                message: format!("JavaScript PostProcessor '{}' promise failed: {}", self.name, e),
                plugin_name: self.name.clone(),
            })?;

        eprintln!(
            "[POST-PROCESSOR] JSON received from JS (first 500 chars): {}",
            &json_output.chars().take(500).collect::<String>()
        );

        let updated: JsExtractionResult =
            serde_json::from_str(&json_output).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                message: format!(
                    "Failed to deserialize result from JavaScript PostProcessor '{}': {}",
                    self.name, e
                ),
                plugin_name: self.name.clone(),
            })?;

        let rust_result =
            kreuzberg::ExtractionResult::try_from(updated).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                message: format!("Failed to convert result from JavaScript PostProcessor: {}", e),
                plugin_name: self.name.clone(),
            })?;

        eprintln!(
            "[POST-PROCESSOR] Final Rust metadata.additional keys after conversion: {:?}",
            rust_result.metadata.additional.keys().collect::<Vec<_>>()
        );
        eprintln!("[POST-PROCESSOR] === Completed JS Post-Processor '{}' ===\n", self.name);

        *result = rust_result;
        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        self.stage
    }
}

/// Register a custom postprocessor
///
/// Registers a JavaScript PostProcessor that will be called after extraction.
///
/// # Arguments
///
/// * `processor` - JavaScript object with the following interface:
///   - `name(): string` - Unique processor name
///   - `process(...args): string` - Process function that receives JSON string as args\[0\]
///   - `processingStage(): "early" | "middle" | "late"` - Optional processing stage
///
/// # Implementation Notes
///
/// Due to NAPI ThreadsafeFunction limitations, the process function receives the extraction
/// result as a JSON string in args\[0\] and must return a JSON string. Use the TypeScript
/// wrapper functions for a cleaner API.
///
/// # Example
///
/// ```typescript
/// import { registerPostProcessor } from '@kreuzberg/node';
///
/// registerPostProcessor({
///   name: () => "word-counter",
///   processingStage: () => "middle",
///   process: (...args) => {
///     const result = JSON.parse(args[0]);
///     const wordCount = result.content.split(/\s+/).length;
///     result.metadata.word_count = wordCount;
///     return JSON.stringify(result);
///   }
/// });
/// ```
#[napi]
pub fn register_post_processor(_env: Env, processor: Object) -> Result<()> {
    validate_plugin_object(&processor, "PostProcessor", &["name", "process"])?;

    let name_fn: Function<(), String> = processor.get_named_property("name")?;
    let name: String = name_fn.call(())?;

    if name.is_empty() {
        return Err(Error::new(
            Status::InvalidArg,
            "Processor name cannot be empty".to_string(),
        ));
    }

    let stage = if let Ok(stage_fn) = processor.get_named_property::<Function<(), String>>("processingStage") {
        let stage_str: String = stage_fn.call(())?;
        match stage_str.to_lowercase().as_str() {
            "early" => ProcessingStage::Early,
            "middle" => ProcessingStage::Middle,
            "late" => ProcessingStage::Late,
            _ => ProcessingStage::Middle,
        }
    } else {
        ProcessingStage::Middle
    };

    let process_fn: Function<String, Promise<String>> = processor.get_named_property("process")?;

    let tsfn = process_fn
        .build_threadsafe_function()
        .build_callback(|ctx| Ok(vec![ctx.value]))?;

    let js_processor = JsPostProcessor {
        name: name.clone(),
        process_fn: Arc::new(tsfn),
        stage,
    };

    let arc_processor: Arc<dyn RustPostProcessor> = Arc::new(js_processor);
    let registry = get_post_processor_registry();
    let mut registry = registry.write().map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to acquire write lock on PostProcessor registry: {}", e),
        )
    })?;

    registry.register(arc_processor, 0).map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to register PostProcessor '{}': {}", name, e),
        )
    })?;

    Ok(())
}

/// Unregister a postprocessor by name
#[napi]
pub fn unregister_post_processor(name: String) -> Result<()> {
    let registry = get_post_processor_registry();
    let mut registry = registry.write().map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to acquire write lock on PostProcessor registry: {}", e),
        )
    })?;

    registry.remove(&name).map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to unregister PostProcessor '{}': {}", name, e),
        )
    })?;
    Ok(())
}

/// Clear all registered postprocessors
#[napi]
pub fn clear_post_processors() -> Result<()> {
    let registry = get_post_processor_registry();
    let mut registry = registry.write().map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to acquire write lock on PostProcessor registry: {}", e),
        )
    })?;

    *registry = Default::default();
    Ok(())
}

/// List all registered post-processors
#[napi]
pub fn list_post_processors() -> Result<Vec<String>> {
    let registry = get_post_processor_registry();
    let registry = registry.read().map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to acquire read lock on PostProcessor registry: {}", e),
        )
    })?;

    Ok(registry.list())
}

use kreuzberg::plugins::Validator as RustValidator;

/// Wrapper that makes a JavaScript Validator usable from Rust.
///
/// Uses JSON serialization to pass data between Rust and JavaScript due to NAPI limitations
/// with complex object types across ThreadsafeFunction boundaries.
///
/// Wrapper that holds the ThreadsafeFunction to call JavaScript from Rust.
/// The validate_fn is an async JavaScript function that:
/// - Takes: String (JSON-serialized ExtractionResult)
/// - Returns: Promise<String> (empty string on success, rejects on validation failure)
///
/// Type parameters:
/// - Input: String
/// - Return: Promise<String>
/// - CallJsBackArgs: Vec<String> (because build_callback returns vec![value])
/// - ErrorStatus: napi::Status
/// - CalleeHandled: false (default with build_callback)
struct JsValidator {
    name: String,
    validate_fn: Arc<ThreadsafeFunction<String, Promise<String>, Vec<String>, napi::Status, false>>,
    priority: i32,
}

unsafe impl Send for JsValidator {}
unsafe impl Sync for JsValidator {}

impl Plugin for JsValidator {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }

    fn initialize(&self) -> std::result::Result<(), kreuzberg::KreuzbergError> {
        Ok(())
    }

    fn shutdown(&self) -> std::result::Result<(), kreuzberg::KreuzbergError> {
        Ok(())
    }
}

#[async_trait]
impl RustValidator for JsValidator {
    async fn validate(
        &self,
        result: &kreuzberg::ExtractionResult,
        _config: &kreuzberg::ExtractionConfig,
    ) -> std::result::Result<(), kreuzberg::KreuzbergError> {
        let js_result =
            JsExtractionResult::try_from(result.clone()).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                message: format!("Failed to convert result for JavaScript Validator: {}", e),
                plugin_name: self.name.clone(),
            })?;
        let json_input = serde_json::to_string(&js_result).map_err(|e| kreuzberg::KreuzbergError::Plugin {
            message: format!("Failed to serialize result for JavaScript Validator: {}", e),
            plugin_name: self.name.clone(),
        })?;

        self.validate_fn
            .call_async(json_input)
            .await
            .map_err(|e| {
                let err_msg = e.to_string();
                if err_msg.contains("ValidationError") || err_msg.contains("validation") {
                    kreuzberg::KreuzbergError::Validation {
                        message: err_msg,
                        source: None,
                    }
                } else {
                    kreuzberg::KreuzbergError::Plugin {
                        message: format!("JavaScript Validator '{}' call failed: {}", self.name, err_msg),
                        plugin_name: self.name.clone(),
                    }
                }
            })?
            .await
            .map_err(|e| {
                let err_msg = e.to_string();
                if err_msg.contains("ValidationError") || err_msg.contains("validation") {
                    kreuzberg::KreuzbergError::Validation {
                        message: err_msg,
                        source: None,
                    }
                } else {
                    kreuzberg::KreuzbergError::Plugin {
                        message: format!("JavaScript Validator '{}' promise failed: {}", self.name, err_msg),
                        plugin_name: self.name.clone(),
                    }
                }
            })?;

        Ok(())
    }

    fn priority(&self) -> i32 {
        self.priority
    }
}

/// Register a custom validator
///
/// Registers a JavaScript Validator that will be called after extraction.
///
/// # Arguments
///
/// * `validator` - JavaScript object with the following interface:
///   - `name(): string` - Unique validator name
///   - `validate(...args): Promise<string>` - Validate function that receives JSON string as args\[0\]
///   - `priority(): number` - Optional priority (defaults to 50, higher runs first)
///
/// # Implementation Notes
///
/// Due to NAPI ThreadsafeFunction limitations, the validate function receives the extraction
/// result as a JSON string in args\[0\]. On success, return an empty string. On validation
/// failure, throw an error (the Promise should reject). Use the TypeScript wrapper functions
/// for a cleaner API.
///
/// # Example
///
/// ```typescript
/// import { registerValidator } from '@kreuzberg/node';
///
/// registerValidator({
///   name: () => "min-length",
///   priority: () => 100,
///   validate: async (...args) => {
///     const result = JSON.parse(args[0]);
///     if (result.content.length < 100) {
///       throw new Error("ValidationError: Content too short");
///     }
///     return ""; // Success - return empty string
///   }
/// });
/// ```
#[napi]
pub fn register_validator(_env: Env, validator: Object) -> Result<()> {
    validate_plugin_object(&validator, "Validator", &["name", "validate"])?;

    let name_fn: Function<(), String> = validator.get_named_property("name")?;
    let name: String = name_fn.call(())?;

    if name.is_empty() {
        return Err(Error::new(
            Status::InvalidArg,
            "Validator name cannot be empty".to_string(),
        ));
    }

    let priority = if let Ok(priority_fn) = validator.get_named_property::<Function<(), i32>>("priority") {
        priority_fn.call(())?
    } else {
        50
    };

    let validate_fn: Function<String, Promise<String>> = validator.get_named_property("validate")?;

    let tsfn = validate_fn
        .build_threadsafe_function()
        .build_callback(|ctx| Ok(vec![ctx.value]))?;

    let js_validator = JsValidator {
        name: name.clone(),
        validate_fn: Arc::new(tsfn),
        priority,
    };

    let arc_validator: Arc<dyn RustValidator> = Arc::new(js_validator);
    let registry = get_validator_registry();
    let mut registry = registry.write().map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to acquire write lock on Validator registry: {}", e),
        )
    })?;

    registry.register(arc_validator).map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to register Validator '{}': {}", name, e),
        )
    })?;

    Ok(())
}

/// Unregister a validator by name
#[napi]
pub fn unregister_validator(name: String) -> Result<()> {
    let registry = get_validator_registry();
    let mut registry = registry.write().map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to acquire write lock on Validator registry: {}", e),
        )
    })?;

    registry.remove(&name).map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to unregister Validator '{}': {}", name, e),
        )
    })?;
    Ok(())
}

/// Clear all registered validators
#[napi]
pub fn clear_validators() -> Result<()> {
    let registry = get_validator_registry();
    let mut registry = registry.write().map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to acquire write lock on Validator registry: {}", e),
        )
    })?;

    *registry = Default::default();
    Ok(())
}

/// List all registered validators
#[napi]
pub fn list_validators() -> Result<Vec<String>> {
    let registry = get_validator_registry();
    let registry = registry.read().map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to acquire read lock on Validator registry: {}", e),
        )
    })?;

    Ok(registry.list())
}

use kreuzberg::plugins::registry::get_ocr_backend_registry;
use kreuzberg::plugins::{OcrBackend as RustOcrBackend, OcrBackendType};

/// Wrapper that makes a JavaScript OCR backend usable from Rust.
///
/// Uses JSON serialization to pass data between Rust and JavaScript due to NAPI limitations
/// with complex object types across ThreadsafeFunction boundaries.
///
/// Wrapper that holds the ThreadsafeFunction to call JavaScript from Rust.
/// The processImage_fn is an async JavaScript function that:
/// - Takes: (String, String) - base64-encoded image bytes and language code
/// - Returns: Promise<String> (JSON-serialized ExtractionResult)
///
/// Type parameters:
/// - Input: (String, String)
/// - Return: Promise<String>
/// - CallJsBackArgs: Vec<(String, String)> (because build_callback returns vec![value])
/// - ErrorStatus: napi::Status
/// - CalleeHandled: false (default with build_callback)
type ProcessImageFn =
    Arc<ThreadsafeFunction<(String, String), Promise<String>, Vec<(String, String)>, napi::Status, false>>;

struct JsOcrBackend {
    name: String,
    supported_languages: Vec<String>,
    process_image_fn: ProcessImageFn,
}

unsafe impl Send for JsOcrBackend {}
unsafe impl Sync for JsOcrBackend {}

impl Plugin for JsOcrBackend {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }

    fn initialize(&self) -> std::result::Result<(), kreuzberg::KreuzbergError> {
        Ok(())
    }

    fn shutdown(&self) -> std::result::Result<(), kreuzberg::KreuzbergError> {
        Ok(())
    }
}

#[async_trait]
impl RustOcrBackend for JsOcrBackend {
    async fn process_image(
        &self,
        image_bytes: &[u8],
        config: &kreuzberg::OcrConfig,
    ) -> std::result::Result<kreuzberg::ExtractionResult, kreuzberg::KreuzbergError> {
        #[cfg(debug_assertions)]
        {
            if std::env::var("KREUZBERG_DEBUG_GUTEN").as_deref() == Ok("1") {
                let header: Vec<u8> = image_bytes.iter().take(8).copied().collect();
                eprintln!("[Rust OCR] Debug input header: {:?}", header);
            }
        }
        let encoded = base64::engine::general_purpose::STANDARD.encode(image_bytes);
        let language = config.language.clone();
        let backend_name = self.name.clone();

        let json_output = self
            .process_image_fn
            .call_async((encoded, language))
            .await
            .map_err(|e| kreuzberg::KreuzbergError::Ocr {
                message: format!("JavaScript OCR backend '{}' failed: {}", backend_name, e),
                source: Some(Box::new(e)),
            })?
            .await
            .map_err(|e| kreuzberg::KreuzbergError::Ocr {
                message: format!("JavaScript OCR backend '{}' failed: {}", backend_name, e),
                source: Some(Box::new(e)),
            })?;

        let wire_result: serde_json::Value =
            serde_json::from_str(&json_output).map_err(|e| kreuzberg::KreuzbergError::Ocr {
                message: format!(
                    "Failed to deserialize result from JavaScript OCR backend '{}': {}",
                    backend_name, e
                ),
                source: Some(Box::new(e)),
            })?;

        let content = wire_result
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| kreuzberg::KreuzbergError::Ocr {
                message: format!(
                    "JavaScript OCR backend '{}' result missing 'content' field",
                    backend_name
                ),
                source: None,
            })?
            .to_string();

        let mime_type = wire_result
            .get("mime_type")
            .and_then(|v| v.as_str())
            .unwrap_or("text/plain")
            .to_string();

        let metadata = wire_result
            .get("metadata")
            .cloned()
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        let metadata: kreuzberg::types::Metadata =
            serde_json::from_value(metadata).map_err(|e| kreuzberg::KreuzbergError::Ocr {
                message: format!(
                    "Failed to parse metadata from JavaScript OCR backend '{}': {}",
                    backend_name, e
                ),
                source: Some(Box::new(e)),
            })?;

        let tables = wire_result
            .get("tables")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| serde_json::from_value::<kreuzberg::Table>(t.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();

        Ok(kreuzberg::ExtractionResult {
            content,
            mime_type,
            metadata,
            tables,
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }

    async fn process_file(
        &self,
        path: &std::path::Path,
        config: &kreuzberg::OcrConfig,
    ) -> std::result::Result<kreuzberg::ExtractionResult, kreuzberg::KreuzbergError> {
        use kreuzberg::core::io;
        let bytes = io::read_file_async(path).await?;
        self.process_image(&bytes, config).await
    }

    fn supports_language(&self, lang: &str) -> bool {
        self.supported_languages.iter().any(|l| l == lang)
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }

    fn supported_languages(&self) -> Vec<String> {
        self.supported_languages.clone()
    }
}

/// Register a custom OCR backend
///
/// Registers a JavaScript OCR backend that can process images and extract text.
///
/// # Arguments
///
/// * `backend` - JavaScript object with the following interface:
///   - `name(): string` - Unique backend name
///   - `supportedLanguages(): string[]` - Array of supported ISO 639-2/3 language codes
///   - `processImage(imageBytes: string, language: string): Promise<result>` - Process image and return extraction result
///
/// # Implementation Notes
///
/// Due to NAPI ThreadsafeFunction limitations, the processImage function receives:
/// - `imageBytes` as a Base64 string (first argument)
/// - `language` as string (second argument)
///
/// And must return a Promise resolving to a JSON-serializable object with:
/// ```typescript
/// {
///   content: string,
///   mime_type: string,  // default: "text/plain"
///   metadata: object,   // default: {}
///   tables: array       // default: []
/// }
/// ```
///
/// # Example
///
/// ```typescript
/// import { registerOcrBackend } from '@kreuzberg/node';
///
/// registerOcrBackend({
///   name: () => "my-ocr",
///   supportedLanguages: () => ["eng", "deu", "fra"],
///   processImage: async (imageBytes, language) => {
///     const buffer = Buffer.from(imageBytes, "base64");
///     const text = await myOcrLibrary.process(buffer, language);
///     return {
///       content: text,
///       mime_type: "text/plain",
///       metadata: { confidence: 0.95 },
///       tables: []
///     };
///   }
/// });
/// ```
#[napi]
pub fn register_ocr_backend(_env: Env, backend: Object) -> Result<()> {
    validate_plugin_object(&backend, "OCR Backend", &["name", "supportedLanguages", "processImage"])?;

    let name_fn: Function<(), String> = backend.get_named_property("name")?;
    let name: String = name_fn.call(())?;

    if name.is_empty() {
        return Err(Error::new(
            Status::InvalidArg,
            "OCR backend name cannot be empty".to_string(),
        ));
    }

    let supported_languages_fn: Function<(), Vec<String>> = backend.get_named_property("supportedLanguages")?;
    let supported_languages: Vec<String> = supported_languages_fn.call(())?;

    if supported_languages.is_empty() {
        return Err(Error::new(
            Status::InvalidArg,
            "OCR backend must support at least one language".to_string(),
        ));
    }

    let process_image_fn: Function<(String, String), Promise<String>> = backend.get_named_property("processImage")?;

    let tsfn = process_image_fn
        .build_threadsafe_function()
        .build_callback(|ctx| Ok(vec![ctx.value]))?;

    let js_ocr_backend = JsOcrBackend {
        name: name.clone(),
        supported_languages,
        process_image_fn: Arc::new(tsfn),
    };

    let arc_backend: Arc<dyn RustOcrBackend> = Arc::new(js_ocr_backend);
    let registry = get_ocr_backend_registry();
    let mut registry = registry.write().map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to acquire write lock on OCR backend registry: {}", e),
        )
    })?;

    registry.register(arc_backend).map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to register OCR backend '{}': {}", name, e),
        )
    })?;

    Ok(())
}

/// Detect MIME type from a file path.
///
/// Uses file extension to determine MIME type. Falls back to `mime_guess` crate
/// if extension-based detection fails.
///
/// # Parameters
///
/// * `path` - Path to the file (string)
/// * `check_exists` - Whether to verify file existence (defaults to true)
///
/// # Returns
///
/// The detected MIME type string.
///
/// # Errors
///
/// Throws an error if:
/// - File doesn't exist (when `check_exists` is true)
/// - MIME type cannot be determined from path/extension
/// - Extension is unknown
///
/// # Example
///
/// ```typescript
/// import { detectMimeType } from '@goldziher/kreuzberg';
///
/// // Detect from existing file
/// const mimeType = detectMimeType('document.pdf');
/// console.log(mimeType); // 'application/pdf'
///
/// // Detect without checking file existence
/// const mimeType2 = detectMimeType('document.docx', false);
/// console.log(mimeType2); // 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'
/// ```
#[napi]
pub fn detect_mime_type(path: String, check_exists: Option<bool>) -> Result<String> {
    let check = check_exists.unwrap_or(true);

    kreuzberg::core::mime::detect_mime_type(&path, check).map_err(convert_error)
}

/// Validate that a MIME type is supported by Kreuzberg.
///
/// Checks if a MIME type is in the list of supported formats. Note that any
/// `image/*` MIME type is automatically considered valid.
///
/// # Parameters
///
/// * `mime_type` - The MIME type to validate (string)
///
/// # Returns
///
/// The validated MIME type (may be normalized).
///
/// # Errors
///
/// Throws an error if the MIME type is not supported.
///
/// # Example
///
/// ```typescript
/// import { validateMimeType } from '@goldziher/kreuzberg';
///
/// // Validate supported type
/// const validated = validateMimeType('application/pdf');
/// console.log(validated); // 'application/pdf'
///
/// // Validate custom image type
/// const validated2 = validateMimeType('image/custom-format');
/// console.log(validated2); // 'image/custom-format' (any image/* is valid)
///
/// // Validate unsupported type (throws error)
/// try {
///   validateMimeType('video/mp4');
/// } catch (err) {
///   console.error(err); // Error: Unsupported format: video/mp4
/// }
/// ```
#[napi]
pub fn validate_mime_type(mime_type: String) -> Result<String> {
    kreuzberg::core::mime::validate_mime_type(&mime_type).map_err(convert_error)
}

/// Embedding preset configuration for TypeScript bindings.
///
/// Contains all settings for a specific embedding model preset.
#[napi(object)]
pub struct EmbeddingPreset {
    /// Name of the preset (e.g., "fast", "balanced", "quality", "multilingual")
    pub name: String,
    /// Recommended chunk size in characters
    pub chunk_size: u32,
    /// Recommended overlap in characters
    pub overlap: u32,
    /// Model identifier (e.g., "AllMiniLML6V2Q", "BGEBaseENV15")
    pub model_name: String,
    /// Embedding vector dimensions
    pub dimensions: u32,
    /// Human-readable description of the preset
    pub description: String,
}

/// List all available embedding preset names.
///
/// Returns an array of preset names that can be used with `getEmbeddingPreset`.
///
/// # Returns
///
/// Array of 4 preset names: ["fast", "balanced", "quality", "multilingual"]
///
/// # Example
///
/// ```typescript
/// import { listEmbeddingPresets } from '@goldziher/kreuzberg';
///
/// const presets = listEmbeddingPresets();
/// console.log(presets); // ['fast', 'balanced', 'quality', 'multilingual']
/// ```
#[napi(js_name = "listEmbeddingPresets")]
pub fn list_embedding_presets() -> Vec<String> {
    kreuzberg::embeddings::list_presets()
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}

/// Get a specific embedding preset by name.
///
/// Returns a preset configuration object, or null if the preset name is not found.
///
/// # Arguments
///
/// * `name` - The preset name (case-sensitive)
///
/// # Returns
///
/// An `EmbeddingPreset` object with the following properties:
/// - `name`: string - Preset name
/// - `chunkSize`: number - Recommended chunk size in characters
/// - `overlap`: number - Recommended overlap in characters
/// - `modelName`: string - Model identifier
/// - `dimensions`: number - Embedding vector dimensions
/// - `description`: string - Human-readable description
///
/// Returns `null` if preset name is not found.
///
/// # Example
///
/// ```typescript
/// import { getEmbeddingPreset } from '@goldziher/kreuzberg';
///
/// const preset = getEmbeddingPreset('balanced');
/// if (preset) {
///   console.log(`Model: ${preset.modelName}, Dims: ${preset.dimensions}`);
///   // Model: BGEBaseENV15, Dims: 768
/// }
/// ```
#[napi(js_name = "getEmbeddingPreset")]
pub fn get_embedding_preset(name: String) -> Option<EmbeddingPreset> {
    let preset = kreuzberg::embeddings::get_preset(&name)?;

    let model_name = format!("{:?}", preset.model);

    Some(EmbeddingPreset {
        name: preset.name.to_string(),
        chunk_size: preset.chunk_size as u32,
        overlap: preset.overlap as u32,
        model_name,
        dimensions: preset.dimensions as u32,
        description: preset.description.to_string(),
    })
}

// #[cfg(all(
// #[global_allocator]
