//! Kreuzberg Ruby Bindings (Magnus)
//!
//! High-performance document intelligence framework bindings for Ruby.
//! Provides extraction, OCR, chunking, and language detection for 30+ file formats.

use html_to_markdown_rs::options::{
    CodeBlockStyle, ConversionOptions, HeadingStyle, HighlightStyle, ListIndentType, NewlineStyle, PreprocessingPreset,
    WhitespaceMode,
};
use kreuzberg::keywords::{
    KeywordAlgorithm as RustKeywordAlgorithm, KeywordConfig as RustKeywordConfig, RakeParams as RustRakeParams,
    YakeParams as RustYakeParams,
};
use kreuzberg::types::TesseractConfig as RustTesseractConfig;
use kreuzberg::{
    ChunkingConfig, EmbeddingConfig, ExtractionConfig, ExtractionResult as RustExtractionResult, ImageExtractionConfig,
    ImagePreprocessingConfig, KreuzbergError, LanguageDetectionConfig, OcrConfig, PdfConfig, PostProcessorConfig,
    TokenReductionConfig,
};
use magnus::exception::ExceptionClass;
use magnus::r_hash::ForEach;
use magnus::value::ReprValue;
use magnus::{Error, IntoValue, RArray, RHash, Ruby, Symbol, TryConvert, Value, function, scan_args::scan_args};
use std::fs;
use std::path::{Path, PathBuf};

/// Keeps Ruby values alive across plugin registrations by informing the GC.
struct GcGuardedValue {
    value: Value,
}

impl GcGuardedValue {
    fn new(value: Value) -> Self {
        let ruby = Ruby::get().expect("Ruby not initialized");
        ruby.gc_register_address(&value);
        Self { value }
    }

    fn value(&self) -> Value {
        self.value
    }
}

impl Drop for GcGuardedValue {
    fn drop(&mut self) {
        if let Ok(ruby) = Ruby::get() {
            ruby.gc_unregister_address(&self.value);
        }
    }
}

/// Convert Kreuzberg errors to Ruby exceptions
fn kreuzberg_error(err: KreuzbergError) -> Error {
    let ruby = Ruby::get().expect("Ruby not initialized");

    let fetch_error_class = |name: &str| -> Option<ExceptionClass> {
        ruby.eval::<ExceptionClass>(&format!("Kreuzberg::Errors::{}", name))
            .ok()
    };

    match err {
        KreuzbergError::Validation { message, .. } => {
            if let Some(class) = fetch_error_class("ValidationError") {
                Error::new(class, message)
            } else {
                Error::new(ruby.exception_arg_error(), message)
            }
        }
        KreuzbergError::Parsing { message, .. } => {
            if let Some(class) = fetch_error_class("ParsingError") {
                Error::new(class, message)
            } else {
                Error::new(ruby.exception_runtime_error(), format!("ParsingError: {}", message))
            }
        }
        KreuzbergError::Ocr { message, .. } => {
            if let Some(class) = fetch_error_class("OCRError") {
                Error::new(class, message)
            } else {
                Error::new(ruby.exception_runtime_error(), format!("OCRError: {}", message))
            }
        }
        KreuzbergError::MissingDependency(message) => {
            if let Some(class) = fetch_error_class("MissingDependencyError") {
                Error::new(class, message)
            } else {
                Error::new(
                    ruby.exception_runtime_error(),
                    format!("MissingDependencyError: {}", message),
                )
            }
        }
        KreuzbergError::Plugin { message, plugin_name } => {
            if let Some(class) = fetch_error_class("PluginError") {
                Error::new(class, format!("{}: {}", plugin_name, message))
            } else {
                Error::new(
                    ruby.exception_runtime_error(),
                    format!("Plugin error in '{}': {}", plugin_name, message),
                )
            }
        }
        KreuzbergError::Io(err) => {
            if let Some(class) = fetch_error_class("IOError") {
                Error::new(class, err.to_string())
            } else {
                Error::new(ruby.exception_runtime_error(), format!("IO error: {}", err))
            }
        }
        KreuzbergError::UnsupportedFormat(message) => {
            if let Some(class) = fetch_error_class("UnsupportedFormatError") {
                Error::new(class, message)
            } else {
                Error::new(
                    ruby.exception_runtime_error(),
                    format!("UnsupportedFormatError: {}", message),
                )
            }
        }
        other => Error::new(ruby.exception_runtime_error(), other.to_string()),
    }
}

fn runtime_error(message: impl Into<String>) -> Error {
    let ruby = Ruby::get().expect("Ruby not initialized");
    Error::new(ruby.exception_runtime_error(), message.into())
}

/// Convert Ruby Symbol or String to Rust String
fn symbol_to_string(value: Value) -> Result<String, Error> {
    if let Some(symbol) = Symbol::from_value(value) {
        Ok(symbol.name()?.to_string())
    } else {
        String::try_convert(value)
    }
}

/// Get keyword argument from hash (supports both symbol and string keys)
fn get_kw(ruby: &Ruby, hash: RHash, name: &str) -> Option<Value> {
    hash.get(name).or_else(|| {
        let sym = ruby.intern(name);
        hash.get(sym)
    })
}

fn set_hash_entry(_ruby: &Ruby, hash: &RHash, key: &str, value: Value) -> Result<(), Error> {
    hash.aset(key, value)?;
    Ok(())
}

fn ocr_config_to_ruby_hash(ruby: &Ruby, config: &kreuzberg::OcrConfig) -> Result<RHash, Error> {
    let value =
        serde_json::to_value(config).map_err(|e| runtime_error(format!("Failed to serialize OCR config: {}", e)))?;
    let ruby_value = json_value_to_ruby(ruby, &value)?;
    RHash::try_convert(ruby_value).map_err(|_| runtime_error("OCR config must return a Hash"))
}

fn cache_root_dir() -> Result<PathBuf, Error> {
    std::env::current_dir()
        .map(|dir| dir.join(".kreuzberg"))
        .map_err(|e| runtime_error(format!("Failed to get current directory: {}", e)))
}

fn cache_directories(root: &Path) -> Result<Vec<PathBuf>, Error> {
    if !root.exists() {
        return Ok(vec![]);
    }

    let mut dirs = vec![root.to_path_buf()];
    let entries = fs::read_dir(root).map_err(|e| runtime_error(format!("Failed to read cache root: {}", e)))?;

    for entry in entries {
        let entry = entry.map_err(|e| runtime_error(format!("Failed to read cache directory entry: {}", e)))?;
        if entry
            .file_type()
            .map_err(|e| runtime_error(format!("Failed to determine cache entry type: {}", e)))?
            .is_dir()
        {
            dirs.push(entry.path());
        }
    }

    Ok(dirs)
}

fn json_value_to_ruby(ruby: &Ruby, value: &serde_json::Value) -> Result<Value, Error> {
    Ok(match value {
        serde_json::Value::Null => ruby.qnil().as_value(),
        serde_json::Value::Bool(b) => {
            if *b {
                ruby.qtrue().as_value()
            } else {
                ruby.qfalse().as_value()
            }
        }
        serde_json::Value::Number(num) => {
            if let Some(i) = num.as_i64() {
                ruby.integer_from_i64(i).into_value_with(ruby)
            } else if let Some(u) = num.as_u64() {
                ruby.integer_from_u64(u).into_value_with(ruby)
            } else if let Some(f) = num.as_f64() {
                ruby.float_from_f64(f).into_value_with(ruby)
            } else {
                ruby.qnil().as_value()
            }
        }
        serde_json::Value::String(s) => ruby.str_new(s).into_value_with(ruby),
        serde_json::Value::Array(items) => {
            let ary = ruby.ary_new();
            for item in items {
                ary.push(json_value_to_ruby(ruby, item)?)?;
            }
            ary.into_value_with(ruby)
        }
        serde_json::Value::Object(map) => {
            let hash = ruby.hash_new();
            for (key, val) in map {
                let key_value = ruby.str_new(key).into_value_with(ruby);
                let val_value = json_value_to_ruby(ruby, val)?;
                hash.aset(key_value, val_value)?;
            }
            hash.into_value_with(ruby)
        }
    })
}

fn ruby_key_to_string(value: Value) -> Result<String, Error> {
    if let Ok(sym) = Symbol::try_convert(value) {
        Ok(sym.name()?.to_string())
    } else {
        String::try_convert(value)
    }
}

fn ruby_value_to_json(value: Value) -> Result<serde_json::Value, Error> {
    let ruby = Ruby::get().expect("Ruby not initialized");

    if value.is_nil() {
        return Ok(serde_json::Value::Null);
    }

    if value.equal(ruby.qtrue())? {
        return Ok(serde_json::Value::Bool(true));
    }

    if value.equal(ruby.qfalse())? {
        return Ok(serde_json::Value::Bool(false));
    }

    if let Ok(integer) = i64::try_convert(value) {
        return Ok(serde_json::Value::Number(integer.into()));
    }

    if let Ok(unsigned) = u64::try_convert(value) {
        return Ok(serde_json::Value::Number(serde_json::Number::from(unsigned)));
    }

    if let Ok(float) = f64::try_convert(value) {
        if let Some(num) = serde_json::Number::from_f64(float) {
            return Ok(serde_json::Value::Number(num));
        }
    }

    if let Ok(sym) = Symbol::try_convert(value) {
        return Ok(serde_json::Value::String(sym.name()?.to_string()));
    }

    if let Ok(string) = String::try_convert(value) {
        return Ok(serde_json::Value::String(string));
    }

    if let Ok(array) = RArray::try_convert(value) {
        let mut values = Vec::with_capacity(array.len());
        for item in array.into_iter() {
            values.push(ruby_value_to_json(item)?);
        }
        return Ok(serde_json::Value::Array(values));
    }

    if let Ok(hash) = RHash::try_convert(value) {
        let mut map = serde_json::Map::new();
        hash.foreach(|key: Value, val: Value| {
            let key_string = ruby_key_to_string(key)?;
            let json_value = ruby_value_to_json(val)?;
            map.insert(key_string, json_value);
            Ok(ForEach::Continue)
        })?;

        return Ok(serde_json::Value::Object(map));
    }

    Err(runtime_error("Unsupported Ruby value for JSON conversion"))
}

/// Parse OcrConfig from Ruby Hash
fn parse_ocr_config(ruby: &Ruby, hash: RHash) -> Result<OcrConfig, Error> {
    let backend = if let Some(val) = get_kw(ruby, hash, "backend") {
        symbol_to_string(val)?
    } else {
        "tesseract".to_string()
    };

    let language = if let Some(val) = get_kw(ruby, hash, "language") {
        symbol_to_string(val)?
    } else {
        "eng".to_string()
    };

    let mut config = OcrConfig {
        backend,
        language,
        tesseract_config: None,
    };

    if let Some(val) = get_kw(ruby, hash, "tesseract_config")
        && !val.is_nil()
    {
        let tc_json = ruby_value_to_json(val)?;
        let parsed: RustTesseractConfig =
            serde_json::from_value(tc_json).map_err(|e| runtime_error(format!("Invalid tesseract_config: {}", e)))?;
        config.tesseract_config = Some(parsed);
    }

    Ok(config)
}

/// Parse ChunkingConfig from Ruby Hash
fn parse_chunking_config(ruby: &Ruby, hash: RHash) -> Result<ChunkingConfig, Error> {
    let max_chars = if let Some(val) = get_kw(ruby, hash, "max_chars") {
        usize::try_convert(val)?
    } else {
        1000
    };

    let max_overlap = if let Some(val) = get_kw(ruby, hash, "max_overlap") {
        usize::try_convert(val)?
    } else {
        200
    };

    let preset = if let Some(val) = get_kw(ruby, hash, "preset")
        && !val.is_nil()
    {
        Some(symbol_to_string(val)?)
    } else {
        None
    };

    let embedding = if let Some(val) = get_kw(ruby, hash, "embedding")
        && !val.is_nil()
    {
        let json_value = ruby_value_to_json(val)?;
        let parsed: EmbeddingConfig = serde_json::from_value(json_value)
            .map_err(|e| runtime_error(format!("Invalid chunking.embedding: {}", e)))?;
        Some(parsed)
    } else {
        None
    };

    let config = ChunkingConfig {
        max_chars,
        max_overlap,
        embedding,
        preset,
    };

    Ok(config)
}

/// Parse LanguageDetectionConfig from Ruby Hash
fn parse_language_detection_config(ruby: &Ruby, hash: RHash) -> Result<LanguageDetectionConfig, Error> {
    let enabled = if let Some(val) = get_kw(ruby, hash, "enabled") {
        bool::try_convert(val)?
    } else {
        true
    };

    let min_confidence = if let Some(val) = get_kw(ruby, hash, "min_confidence") {
        f64::try_convert(val)?
    } else {
        0.8
    };

    let detect_multiple = if let Some(val) = get_kw(ruby, hash, "detect_multiple") {
        bool::try_convert(val)?
    } else {
        false
    };

    let config = LanguageDetectionConfig {
        enabled,
        min_confidence,
        detect_multiple,
    };

    Ok(config)
}

/// Parse PdfConfig from Ruby Hash
fn parse_pdf_config(ruby: &Ruby, hash: RHash) -> Result<PdfConfig, Error> {
    let extract_images = if let Some(val) = get_kw(ruby, hash, "extract_images") {
        bool::try_convert(val)?
    } else {
        false
    };

    let passwords = if let Some(val) = get_kw(ruby, hash, "passwords") {
        if !val.is_nil() {
            let arr = RArray::try_convert(val)?;
            Some(arr.to_vec::<String>()?)
        } else {
            None
        }
    } else {
        None
    };

    let extract_metadata = if let Some(val) = get_kw(ruby, hash, "extract_metadata") {
        bool::try_convert(val)?
    } else {
        true
    };

    let config = PdfConfig {
        extract_images,
        passwords,
        extract_metadata,
    };

    Ok(config)
}

/// Parse ImageExtractionConfig from Ruby Hash
fn parse_image_extraction_config(ruby: &Ruby, hash: RHash) -> Result<ImageExtractionConfig, Error> {
    let extract_images = if let Some(val) = get_kw(ruby, hash, "extract_images") {
        bool::try_convert(val)?
    } else {
        true
    };

    let target_dpi = if let Some(val) = get_kw(ruby, hash, "target_dpi") {
        i32::try_convert(val)?
    } else {
        300
    };

    let max_image_dimension = if let Some(val) = get_kw(ruby, hash, "max_image_dimension") {
        i32::try_convert(val)?
    } else {
        4096
    };

    let auto_adjust_dpi = if let Some(val) = get_kw(ruby, hash, "auto_adjust_dpi") {
        bool::try_convert(val)?
    } else {
        true
    };

    let min_dpi = if let Some(val) = get_kw(ruby, hash, "min_dpi") {
        i32::try_convert(val)?
    } else {
        72
    };

    let max_dpi = if let Some(val) = get_kw(ruby, hash, "max_dpi") {
        i32::try_convert(val)?
    } else {
        600
    };

    let config = ImageExtractionConfig {
        extract_images,
        target_dpi,
        max_image_dimension,
        auto_adjust_dpi,
        min_dpi,
        max_dpi,
    };

    Ok(config)
}

/// Parse ImagePreprocessingConfig from Ruby Hash
///
/// Note: Currently not used in ExtractionConfig but provided for completeness.
/// ImagePreprocessingConfig is typically used in OCR operations.
#[allow(dead_code)]
fn parse_image_preprocessing_config(ruby: &Ruby, hash: RHash) -> Result<ImagePreprocessingConfig, Error> {
    let target_dpi = if let Some(val) = get_kw(ruby, hash, "target_dpi") {
        i32::try_convert(val)?
    } else {
        300
    };

    let auto_rotate = if let Some(val) = get_kw(ruby, hash, "auto_rotate") {
        bool::try_convert(val)?
    } else {
        true
    };

    let deskew = if let Some(val) = get_kw(ruby, hash, "deskew") {
        bool::try_convert(val)?
    } else {
        true
    };

    let denoise = if let Some(val) = get_kw(ruby, hash, "denoise") {
        bool::try_convert(val)?
    } else {
        false
    };

    let contrast_enhance = if let Some(val) = get_kw(ruby, hash, "contrast_enhance") {
        bool::try_convert(val)?
    } else {
        false
    };

    let binarization_method = if let Some(val) = get_kw(ruby, hash, "binarization_method") {
        symbol_to_string(val)?
    } else {
        "otsu".to_string()
    };

    let invert_colors = if let Some(val) = get_kw(ruby, hash, "invert_colors") {
        bool::try_convert(val)?
    } else {
        false
    };

    let config = ImagePreprocessingConfig {
        target_dpi,
        auto_rotate,
        deskew,
        denoise,
        contrast_enhance,
        binarization_method,
        invert_colors,
    };

    Ok(config)
}

/// Parse PostProcessorConfig from Ruby Hash
fn parse_postprocessor_config(ruby: &Ruby, hash: RHash) -> Result<PostProcessorConfig, Error> {
    let enabled = if let Some(val) = get_kw(ruby, hash, "enabled") {
        bool::try_convert(val)?
    } else {
        true
    };

    let enabled_processors = if let Some(val) = get_kw(ruby, hash, "enabled_processors")
        && !val.is_nil()
    {
        let arr = RArray::try_convert(val)?;
        Some(arr.to_vec::<String>()?)
    } else {
        None
    };

    let disabled_processors = if let Some(val) = get_kw(ruby, hash, "disabled_processors")
        && !val.is_nil()
    {
        let arr = RArray::try_convert(val)?;
        Some(arr.to_vec::<String>()?)
    } else {
        None
    };

    let config = PostProcessorConfig {
        enabled,
        enabled_processors,
        disabled_processors,
    };

    Ok(config)
}

/// Parse TokenReductionConfig from Ruby Hash
fn parse_token_reduction_config(ruby: &Ruby, hash: RHash) -> Result<TokenReductionConfig, Error> {
    let mode = if let Some(val) = get_kw(ruby, hash, "mode") {
        symbol_to_string(val)?
    } else {
        "off".to_string()
    };

    let preserve_important_words = if let Some(val) = get_kw(ruby, hash, "preserve_important_words") {
        bool::try_convert(val)?
    } else {
        true
    };

    let config = TokenReductionConfig {
        mode,
        preserve_important_words,
    };

    Ok(config)
}

fn parse_keyword_config(ruby: &Ruby, hash: RHash) -> Result<RustKeywordConfig, Error> {
    let mut config = RustKeywordConfig::default();

    if let Some(val) = get_kw(ruby, hash, "algorithm") {
        let algo = symbol_to_string(val)?;
        config.algorithm = match algo.to_lowercase().as_str() {
            "yake" => RustKeywordAlgorithm::Yake,
            "rake" => RustKeywordAlgorithm::Rake,
            other => {
                return Err(runtime_error(format!(
                    "Invalid keywords.algorithm '{}', expected 'yake' or 'rake'",
                    other
                )));
            }
        };
    }

    if let Some(val) = get_kw(ruby, hash, "max_keywords") {
        config.max_keywords = usize::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "min_score") {
        config.min_score = f64::try_convert(val)? as f32;
    }

    if let Some(val) = get_kw(ruby, hash, "ngram_range") {
        let ary = RArray::try_convert(val)?;
        if ary.len() == 2 {
            let values = ary.to_vec::<i64>()?;
            config.ngram_range = (values[0] as usize, values[1] as usize);
        } else {
            return Err(runtime_error("keywords.ngram_range must have exactly two values"));
        }
    }

    if let Some(val) = get_kw(ruby, hash, "language") {
        if !val.is_nil() {
            config.language = Some(symbol_to_string(val)?);
        }
    }

    if let Some(val) = get_kw(ruby, hash, "yake_params")
        && !val.is_nil()
    {
        let yake_hash = RHash::try_convert(val)?;
        let window = if let Some(window_val) = get_kw(ruby, yake_hash, "window_size") {
            usize::try_convert(window_val)?
        } else {
            2
        };
        config.yake_params = Some(RustYakeParams { window_size: window });
    }

    if let Some(val) = get_kw(ruby, hash, "rake_params")
        && !val.is_nil()
    {
        let rake_hash = RHash::try_convert(val)?;
        let mut params = RustRakeParams::default();
        if let Some(val) = get_kw(ruby, rake_hash, "min_word_length") {
            params.min_word_length = usize::try_convert(val)?;
        }
        if let Some(val) = get_kw(ruby, rake_hash, "max_words_per_phrase") {
            params.max_words_per_phrase = usize::try_convert(val)?;
        }
        config.rake_params = Some(params);
    }

    Ok(config)
}

fn parse_html_options(ruby: &Ruby, hash: RHash) -> Result<ConversionOptions, Error> {
    let mut options = ConversionOptions::default();

    if let Some(val) = get_kw(ruby, hash, "heading_style") {
        let style = symbol_to_string(val)?;
        options.heading_style = match style.to_lowercase().as_str() {
            "atx" => HeadingStyle::Atx,
            "underlined" => HeadingStyle::Underlined,
            "atx_closed" | "atx-closed" => HeadingStyle::AtxClosed,
            other => return Err(runtime_error(format!("Invalid html_options.heading_style '{}'", other))),
        };
    }

    if let Some(val) = get_kw(ruby, hash, "list_indent_type") {
        let val_str = symbol_to_string(val)?;
        options.list_indent_type = match val_str.to_lowercase().as_str() {
            "spaces" => ListIndentType::Spaces,
            "tabs" => ListIndentType::Tabs,
            other => {
                return Err(runtime_error(format!(
                    "Invalid html_options.list_indent_type '{}'",
                    other
                )));
            }
        };
    }

    if let Some(val) = get_kw(ruby, hash, "list_indent_width") {
        options.list_indent_width = usize::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "bullets") {
        options.bullets = String::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "strong_em_symbol") {
        let symbol = String::try_convert(val)?;
        let mut chars = symbol.chars();
        options.strong_em_symbol = chars
            .next()
            .ok_or_else(|| runtime_error("html_options.strong_em_symbol must not be empty"))?;
    }

    if let Some(val) = get_kw(ruby, hash, "escape_asterisks") {
        options.escape_asterisks = bool::try_convert(val)?;
    }
    if let Some(val) = get_kw(ruby, hash, "escape_underscores") {
        options.escape_underscores = bool::try_convert(val)?;
    }
    if let Some(val) = get_kw(ruby, hash, "escape_misc") {
        options.escape_misc = bool::try_convert(val)?;
    }
    if let Some(val) = get_kw(ruby, hash, "escape_ascii") {
        options.escape_ascii = bool::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "code_language") {
        options.code_language = String::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "autolinks") {
        options.autolinks = bool::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "default_title") {
        options.default_title = bool::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "br_in_tables") {
        options.br_in_tables = bool::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "hocr_spatial_tables") {
        options.hocr_spatial_tables = bool::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "highlight_style") {
        let style = symbol_to_string(val)?;
        options.highlight_style = match style.to_lowercase().as_str() {
            "double_equal" | "double-equal" => HighlightStyle::DoubleEqual,
            "html" => HighlightStyle::Html,
            "bold" => HighlightStyle::Bold,
            "none" => HighlightStyle::None,
            other => {
                return Err(runtime_error(format!(
                    "Invalid html_options.highlight_style '{}'",
                    other
                )));
            }
        };
    }

    if let Some(val) = get_kw(ruby, hash, "extract_metadata") {
        options.extract_metadata = bool::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "whitespace_mode") {
        let mode = symbol_to_string(val)?;
        options.whitespace_mode = match mode.to_lowercase().as_str() {
            "normalized" => WhitespaceMode::Normalized,
            "strict" => WhitespaceMode::Strict,
            other => {
                return Err(runtime_error(format!(
                    "Invalid html_options.whitespace_mode '{}'",
                    other
                )));
            }
        };
    }

    if let Some(val) = get_kw(ruby, hash, "strip_newlines") {
        options.strip_newlines = bool::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "wrap") {
        options.wrap = bool::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "wrap_width") {
        options.wrap_width = usize::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "convert_as_inline") {
        options.convert_as_inline = bool::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "sub_symbol") {
        options.sub_symbol = String::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "sup_symbol") {
        options.sup_symbol = String::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "newline_style") {
        let style = symbol_to_string(val)?;
        options.newline_style = match style.to_lowercase().as_str() {
            "spaces" => NewlineStyle::Spaces,
            "backslash" => NewlineStyle::Backslash,
            other => return Err(runtime_error(format!("Invalid html_options.newline_style '{}'", other))),
        };
    }

    if let Some(val) = get_kw(ruby, hash, "code_block_style") {
        let style = symbol_to_string(val)?;
        options.code_block_style = match style.to_lowercase().as_str() {
            "indented" => CodeBlockStyle::Indented,
            "backticks" => CodeBlockStyle::Backticks,
            "tildes" => CodeBlockStyle::Tildes,
            other => {
                return Err(runtime_error(format!(
                    "Invalid html_options.code_block_style '{}'",
                    other
                )));
            }
        };
    }

    if let Some(val) = get_kw(ruby, hash, "keep_inline_images_in") {
        let arr = RArray::try_convert(val)?;
        options.keep_inline_images_in = arr.to_vec::<String>()?;
    }

    if let Some(val) = get_kw(ruby, hash, "encoding") {
        options.encoding = String::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "debug") {
        options.debug = bool::try_convert(val)?;
    }

    if let Some(val) = get_kw(ruby, hash, "strip_tags") {
        let arr = RArray::try_convert(val)?;
        options.strip_tags = arr.to_vec::<String>()?;
    }

    if let Some(val) = get_kw(ruby, hash, "preserve_tags") {
        let arr = RArray::try_convert(val)?;
        options.preserve_tags = arr.to_vec::<String>()?;
    }

    if let Some(val) = get_kw(ruby, hash, "preprocessing")
        && !val.is_nil()
    {
        let pre_hash = RHash::try_convert(val)?;
        let mut preprocessing = options.preprocessing.clone();
        if let Some(v) = get_kw(ruby, pre_hash, "enabled") {
            preprocessing.enabled = bool::try_convert(v)?;
        }
        if let Some(v) = get_kw(ruby, pre_hash, "preset") {
            let preset = symbol_to_string(v)?;
            preprocessing.preset = match preset.to_lowercase().as_str() {
                "minimal" => PreprocessingPreset::Minimal,
                "standard" => PreprocessingPreset::Standard,
                "aggressive" => PreprocessingPreset::Aggressive,
                other => {
                    return Err(runtime_error(format!(
                        "Invalid html_options.preprocessing.preset '{}'",
                        other
                    )));
                }
            };
        }
        if let Some(v) = get_kw(ruby, pre_hash, "remove_navigation") {
            preprocessing.remove_navigation = bool::try_convert(v)?;
        }
        if let Some(v) = get_kw(ruby, pre_hash, "remove_forms") {
            preprocessing.remove_forms = bool::try_convert(v)?;
        }
        options.preprocessing = preprocessing;
    }

    Ok(options)
}

fn keyword_algorithm_to_str(algo: RustKeywordAlgorithm) -> &'static str {
    match algo {
        RustKeywordAlgorithm::Yake => "yake",
        RustKeywordAlgorithm::Rake => "rake",
    }
}

fn keyword_config_to_ruby_hash(ruby: &Ruby, config: &RustKeywordConfig) -> Result<RHash, Error> {
    let hash = ruby.hash_new();
    hash.aset("algorithm", keyword_algorithm_to_str(config.algorithm))?;
    hash.aset("max_keywords", config.max_keywords as i64)?;
    hash.aset("min_score", config.min_score)?;
    hash.aset("language", config.language.clone().unwrap_or_default())?;

    let range_array = ruby.ary_new();
    range_array.push(config.ngram_range.0 as i64)?;
    range_array.push(config.ngram_range.1 as i64)?;
    hash.aset("ngram_range", range_array)?;

    if let Some(yake) = &config.yake_params {
        let yake_hash = ruby.hash_new();
        yake_hash.aset("window_size", yake.window_size as i64)?;
        hash.aset("yake_params", yake_hash)?;
    }

    if let Some(rake) = &config.rake_params {
        let rake_hash = ruby.hash_new();
        rake_hash.aset("min_word_length", rake.min_word_length as i64)?;
        rake_hash.aset("max_words_per_phrase", rake.max_words_per_phrase as i64)?;
        hash.aset("rake_params", rake_hash)?;
    }

    Ok(hash)
}

fn html_options_to_ruby_hash(ruby: &Ruby, options: &ConversionOptions) -> Result<RHash, Error> {
    let hash = ruby.hash_new();
    hash.aset(
        "heading_style",
        match options.heading_style {
            HeadingStyle::Atx => "atx",
            HeadingStyle::Underlined => "underlined",
            HeadingStyle::AtxClosed => "atx_closed",
        },
    )?;
    hash.aset(
        "list_indent_type",
        match options.list_indent_type {
            ListIndentType::Spaces => "spaces",
            ListIndentType::Tabs => "tabs",
        },
    )?;
    hash.aset("list_indent_width", options.list_indent_width as i64)?;
    hash.aset("bullets", options.bullets.clone())?;
    hash.aset("strong_em_symbol", options.strong_em_symbol.to_string())?;
    hash.aset("escape_asterisks", options.escape_asterisks)?;
    hash.aset("escape_underscores", options.escape_underscores)?;
    hash.aset("escape_misc", options.escape_misc)?;
    hash.aset("escape_ascii", options.escape_ascii)?;
    hash.aset("code_language", options.code_language.clone())?;
    hash.aset("autolinks", options.autolinks)?;
    hash.aset("default_title", options.default_title)?;
    hash.aset("br_in_tables", options.br_in_tables)?;
    hash.aset("hocr_spatial_tables", options.hocr_spatial_tables)?;
    hash.aset(
        "highlight_style",
        match options.highlight_style {
            HighlightStyle::DoubleEqual => "double_equal",
            HighlightStyle::Html => "html",
            HighlightStyle::Bold => "bold",
            HighlightStyle::None => "none",
        },
    )?;
    hash.aset("extract_metadata", options.extract_metadata)?;
    hash.aset(
        "whitespace_mode",
        match options.whitespace_mode {
            WhitespaceMode::Normalized => "normalized",
            WhitespaceMode::Strict => "strict",
        },
    )?;
    hash.aset("strip_newlines", options.strip_newlines)?;
    hash.aset("wrap", options.wrap)?;
    hash.aset("wrap_width", options.wrap_width as i64)?;
    hash.aset("convert_as_inline", options.convert_as_inline)?;
    hash.aset("sub_symbol", options.sub_symbol.clone())?;
    hash.aset("sup_symbol", options.sup_symbol.clone())?;
    hash.aset(
        "newline_style",
        match options.newline_style {
            NewlineStyle::Spaces => "spaces",
            NewlineStyle::Backslash => "backslash",
        },
    )?;
    hash.aset(
        "code_block_style",
        match options.code_block_style {
            CodeBlockStyle::Indented => "indented",
            CodeBlockStyle::Backticks => "backticks",
            CodeBlockStyle::Tildes => "tildes",
        },
    )?;

    let keep_inline = ruby.ary_new();
    for tag in &options.keep_inline_images_in {
        keep_inline.push(tag.as_str())?;
    }
    hash.aset("keep_inline_images_in", keep_inline)?;

    hash.aset("encoding", options.encoding.clone())?;
    hash.aset("debug", options.debug)?;

    let strip_tags = ruby.ary_new();
    for tag in &options.strip_tags {
        strip_tags.push(tag.as_str())?;
    }
    hash.aset("strip_tags", strip_tags)?;

    let preserve_tags = ruby.ary_new();
    for tag in &options.preserve_tags {
        preserve_tags.push(tag.as_str())?;
    }
    hash.aset("preserve_tags", preserve_tags)?;

    let pre_hash = ruby.hash_new();
    pre_hash.aset("enabled", options.preprocessing.enabled)?;
    pre_hash.aset(
        "preset",
        match options.preprocessing.preset {
            PreprocessingPreset::Minimal => "minimal",
            PreprocessingPreset::Standard => "standard",
            PreprocessingPreset::Aggressive => "aggressive",
        },
    )?;
    pre_hash.aset("remove_navigation", options.preprocessing.remove_navigation)?;
    pre_hash.aset("remove_forms", options.preprocessing.remove_forms)?;
    hash.aset("preprocessing", pre_hash)?;

    Ok(hash)
}
/// Parse ExtractionConfig from Ruby Hash
fn parse_extraction_config(ruby: &Ruby, opts: Option<RHash>) -> Result<ExtractionConfig, Error> {
    let mut config = ExtractionConfig::default();

    if let Some(hash) = opts {
        if let Some(val) = get_kw(ruby, hash, "use_cache") {
            config.use_cache = bool::try_convert(val)?;
        }

        if let Some(val) = get_kw(ruby, hash, "enable_quality_processing") {
            config.enable_quality_processing = bool::try_convert(val)?;
        }

        if let Some(val) = get_kw(ruby, hash, "force_ocr") {
            config.force_ocr = bool::try_convert(val)?;
        }

        if let Some(val) = get_kw(ruby, hash, "ocr")
            && !val.is_nil()
        {
            let ocr_hash = RHash::try_convert(val)?;
            config.ocr = Some(parse_ocr_config(ruby, ocr_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "chunking")
            && !val.is_nil()
        {
            let chunking_hash = RHash::try_convert(val)?;
            config.chunking = Some(parse_chunking_config(ruby, chunking_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "language_detection")
            && !val.is_nil()
        {
            let lang_hash = RHash::try_convert(val)?;
            config.language_detection = Some(parse_language_detection_config(ruby, lang_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "pdf_options")
            && !val.is_nil()
        {
            let pdf_hash = RHash::try_convert(val)?;
            config.pdf_options = Some(parse_pdf_config(ruby, pdf_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "images")
            && !val.is_nil()
        {
            let images_hash = RHash::try_convert(val)?;
            config.images = Some(parse_image_extraction_config(ruby, images_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "postprocessor")
            && !val.is_nil()
        {
            let postprocessor_hash = RHash::try_convert(val)?;
            config.postprocessor = Some(parse_postprocessor_config(ruby, postprocessor_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "token_reduction")
            && !val.is_nil()
        {
            let token_reduction_hash = RHash::try_convert(val)?;
            config.token_reduction = Some(parse_token_reduction_config(ruby, token_reduction_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "keywords")
            && !val.is_nil()
        {
            let keywords_hash = RHash::try_convert(val)?;
            config.keywords = Some(parse_keyword_config(ruby, keywords_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "html_options")
            && !val.is_nil()
        {
            let html_hash = RHash::try_convert(val)?;
            config.html_options = Some(parse_html_options(ruby, html_hash)?);
        }

        if let Some(val) = get_kw(ruby, hash, "max_concurrent_extractions") {
            let value = usize::try_convert(val)?;
            config.max_concurrent_extractions = Some(value);
        }
    }

    Ok(config)
}

/// Convert ExtractionConfig to Ruby Hash for Config::Extraction.
///
/// This function converts a Rust ExtractionConfig into a Ruby hash that can be passed
/// to Kreuzberg::Config::Extraction.new(**hash).
fn extraction_config_to_ruby_hash(ruby: &Ruby, config: ExtractionConfig) -> Result<RHash, Error> {
    let hash = ruby.hash_new();

    set_hash_entry(
        ruby,
        &hash,
        "use_cache",
        if config.use_cache {
            ruby.qtrue().as_value()
        } else {
            ruby.qfalse().as_value()
        },
    )?;
    set_hash_entry(
        ruby,
        &hash,
        "enable_quality_processing",
        if config.enable_quality_processing {
            ruby.qtrue().as_value()
        } else {
            ruby.qfalse().as_value()
        },
    )?;
    set_hash_entry(
        ruby,
        &hash,
        "force_ocr",
        if config.force_ocr {
            ruby.qtrue().as_value()
        } else {
            ruby.qfalse().as_value()
        },
    )?;

    if let Some(ocr) = config.ocr {
        let ocr_hash = ruby.hash_new();
        set_hash_entry(
            ruby,
            &ocr_hash,
            "backend",
            ruby.str_new(&ocr.backend).into_value_with(ruby),
        )?;
        set_hash_entry(
            ruby,
            &ocr_hash,
            "language",
            ruby.str_new(&ocr.language).into_value_with(ruby),
        )?;
        if let Some(tesseract_config) = ocr.tesseract_config {
            let tc_json = serde_json::to_value(&tesseract_config)
                .map_err(|e| runtime_error(format!("Failed to serialize tesseract_config: {}", e)))?;
            let tc_ruby = json_value_to_ruby(ruby, &tc_json)?;
            set_hash_entry(ruby, &ocr_hash, "tesseract_config", tc_ruby)?;
        }
        set_hash_entry(ruby, &hash, "ocr", ocr_hash.into_value_with(ruby))?;
    }

    if let Some(chunking) = config.chunking {
        let chunking_hash = ruby.hash_new();
        set_hash_entry(
            ruby,
            &chunking_hash,
            "max_chars",
            ruby.integer_from_i64(chunking.max_chars as i64).into_value_with(ruby),
        )?;
        set_hash_entry(
            ruby,
            &chunking_hash,
            "max_overlap",
            ruby.integer_from_i64(chunking.max_overlap as i64).into_value_with(ruby),
        )?;
        if let Some(preset) = chunking.preset {
            set_hash_entry(
                ruby,
                &chunking_hash,
                "preset",
                ruby.str_new(&preset).into_value_with(ruby),
            )?;
        }
        if let Some(embedding) = chunking.embedding {
            let embedding_json = serde_json::to_value(&embedding)
                .map_err(|e| runtime_error(format!("Failed to serialize embedding config: {}", e)))?;
            let embedding_value = json_value_to_ruby(ruby, &embedding_json)?;
            set_hash_entry(ruby, &chunking_hash, "embedding", embedding_value)?;
        }
        set_hash_entry(ruby, &hash, "chunking", chunking_hash.into_value_with(ruby))?;
    }

    if let Some(lang_detection) = config.language_detection {
        let lang_hash = ruby.hash_new();
        set_hash_entry(
            ruby,
            &lang_hash,
            "enabled",
            if lang_detection.enabled {
                ruby.qtrue().as_value()
            } else {
                ruby.qfalse().as_value()
            },
        )?;
        set_hash_entry(
            ruby,
            &lang_hash,
            "min_confidence",
            ruby.float_from_f64(lang_detection.min_confidence).into_value_with(ruby),
        )?;
        set_hash_entry(
            ruby,
            &lang_hash,
            "detect_multiple",
            if lang_detection.detect_multiple {
                ruby.qtrue().as_value()
            } else {
                ruby.qfalse().as_value()
            },
        )?;
        set_hash_entry(ruby, &hash, "language_detection", lang_hash.into_value_with(ruby))?;
    }

    if let Some(pdf_options) = config.pdf_options {
        let pdf_hash = ruby.hash_new();
        set_hash_entry(
            ruby,
            &pdf_hash,
            "extract_images",
            if pdf_options.extract_images {
                ruby.qtrue().as_value()
            } else {
                ruby.qfalse().as_value()
            },
        )?;
        if let Some(passwords) = pdf_options.passwords {
            let passwords_array = ruby.ary_from_vec(passwords);
            set_hash_entry(ruby, &pdf_hash, "passwords", passwords_array.into_value_with(ruby))?;
        }
        set_hash_entry(
            ruby,
            &pdf_hash,
            "extract_metadata",
            if pdf_options.extract_metadata {
                ruby.qtrue().as_value()
            } else {
                ruby.qfalse().as_value()
            },
        )?;
        set_hash_entry(ruby, &hash, "pdf_options", pdf_hash.into_value_with(ruby))?;
    }

    if let Some(images) = config.images {
        let images_hash = ruby.hash_new();
        set_hash_entry(
            ruby,
            &images_hash,
            "extract_images",
            if images.extract_images {
                ruby.qtrue().as_value()
            } else {
                ruby.qfalse().as_value()
            },
        )?;
        set_hash_entry(
            ruby,
            &images_hash,
            "target_dpi",
            ruby.integer_from_i64(images.target_dpi as i64).into_value_with(ruby),
        )?;
        set_hash_entry(
            ruby,
            &images_hash,
            "max_image_dimension",
            ruby.integer_from_i64(images.max_image_dimension as i64)
                .into_value_with(ruby),
        )?;
        set_hash_entry(
            ruby,
            &images_hash,
            "auto_adjust_dpi",
            if images.auto_adjust_dpi {
                ruby.qtrue().as_value()
            } else {
                ruby.qfalse().as_value()
            },
        )?;
        set_hash_entry(
            ruby,
            &images_hash,
            "min_dpi",
            ruby.integer_from_i64(images.min_dpi as i64).into_value_with(ruby),
        )?;
        set_hash_entry(
            ruby,
            &images_hash,
            "max_dpi",
            ruby.integer_from_i64(images.max_dpi as i64).into_value_with(ruby),
        )?;
        set_hash_entry(ruby, &hash, "image_extraction", images_hash.into_value_with(ruby))?;
    }

    if let Some(postprocessor) = config.postprocessor {
        let pp_hash = ruby.hash_new();
        set_hash_entry(
            ruby,
            &pp_hash,
            "enabled",
            if postprocessor.enabled {
                ruby.qtrue().as_value()
            } else {
                ruby.qfalse().as_value()
            },
        )?;
        if let Some(enabled_processors) = postprocessor.enabled_processors {
            let enabled_array = ruby.ary_from_vec(enabled_processors);
            set_hash_entry(
                ruby,
                &pp_hash,
                "enabled_processors",
                enabled_array.into_value_with(ruby),
            )?;
        }
        if let Some(disabled_processors) = postprocessor.disabled_processors {
            let disabled_array = ruby.ary_from_vec(disabled_processors);
            set_hash_entry(
                ruby,
                &pp_hash,
                "disabled_processors",
                disabled_array.into_value_with(ruby),
            )?;
        }
        set_hash_entry(ruby, &hash, "postprocessor", pp_hash.into_value_with(ruby))?;
    }

    if let Some(token_reduction) = config.token_reduction {
        let tr_hash = ruby.hash_new();
        set_hash_entry(
            ruby,
            &tr_hash,
            "mode",
            ruby.str_new(&token_reduction.mode).into_value_with(ruby),
        )?;
        set_hash_entry(
            ruby,
            &tr_hash,
            "preserve_important_words",
            if token_reduction.preserve_important_words {
                ruby.qtrue().as_value()
            } else {
                ruby.qfalse().as_value()
            },
        )?;
        set_hash_entry(ruby, &hash, "token_reduction", tr_hash.into_value_with(ruby))?;
    }

    if let Some(keywords) = config.keywords {
        let keywords_hash = keyword_config_to_ruby_hash(ruby, &keywords)?;
        set_hash_entry(ruby, &hash, "keywords", keywords_hash.into_value_with(ruby))?;
    }

    if let Some(html_options) = config.html_options {
        let html_hash = html_options_to_ruby_hash(ruby, &html_options)?;
        set_hash_entry(ruby, &hash, "html_options", html_hash.into_value_with(ruby))?;
    }

    if let Some(max_concurrent) = config.max_concurrent_extractions {
        set_hash_entry(
            ruby,
            &hash,
            "max_concurrent_extractions",
            ruby.integer_from_u64(max_concurrent as u64).into_value_with(ruby),
        )?;
    }

    Ok(hash)
}

/// Load extraction configuration from a file.
///
/// Detects the file format from the extension (.toml, .yaml, .json)
/// and loads the configuration accordingly. Returns a hash to be used by Ruby.
///
/// @param path [String] Path to the configuration file
/// @return [Hash] Configuration hash
///
/// @example Load from TOML
///   hash = Kreuzberg._config_from_file_native("config.toml")
///
/// @example Load from YAML
///   hash = Kreuzberg._config_from_file_native("config.yaml")
///
fn config_from_file(path: String) -> Result<RHash, Error> {
    let ruby = Ruby::get().expect("Ruby not initialized");
    let file_path = Path::new(&path);

    let extension = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| runtime_error("File path must have an extension (.toml, .yaml, or .json)"))?;

    let config = match extension {
        "toml" => ExtractionConfig::from_toml_file(file_path).map_err(kreuzberg_error)?,
        "yaml" => ExtractionConfig::from_yaml_file(file_path).map_err(kreuzberg_error)?,
        "json" => ExtractionConfig::from_json_file(file_path).map_err(kreuzberg_error)?,
        _ => {
            return Err(runtime_error(format!(
                "Unsupported file extension '{}'. Supported: .toml, .yaml, .json",
                extension
            )));
        }
    };

    extraction_config_to_ruby_hash(&ruby, config)
}

/// Discover configuration file in current or parent directories.
///
/// Searches for kreuzberg.toml, kreuzberg.yaml, or kreuzberg.json in the current
/// directory and parent directories. Returns nil if no config file is found.
///
/// @return [Hash, nil] Configuration hash or nil if not found
///
/// @example
///   hash = Kreuzberg._config_discover_native
///   # => {...config hash...} or nil
///
fn config_discover() -> Result<Value, Error> {
    let ruby = Ruby::get().expect("Ruby not initialized");

    let maybe_config = ExtractionConfig::discover().map_err(kreuzberg_error)?;

    match maybe_config {
        Some(config) => {
            let hash = extraction_config_to_ruby_hash(&ruby, config)?;
            Ok(hash.as_value())
        }
        None => Ok(ruby.qnil().as_value()),
    }
}

/// Convert Rust ExtractionResult to Ruby Hash
fn extraction_result_to_ruby(ruby: &Ruby, result: RustExtractionResult) -> Result<RHash, Error> {
    let hash = ruby.hash_new();

    let content_value = ruby.str_new(result.content.as_str()).into_value_with(ruby);
    set_hash_entry(ruby, &hash, "content", content_value)?;

    let mime_value = ruby.str_new(result.mime_type.as_str()).into_value_with(ruby);
    set_hash_entry(ruby, &hash, "mime_type", mime_value)?;

    let metadata_json = serde_json::to_string(&result.metadata)
        .map_err(|e| runtime_error(format!("Failed to serialize metadata: {}", e)))?;
    let metadata_json_value = ruby.str_new(&metadata_json).into_value_with(ruby);
    set_hash_entry(ruby, &hash, "metadata_json", metadata_json_value)?;
    let metadata_value = serde_json::to_value(&result.metadata)
        .map_err(|e| runtime_error(format!("Failed to serialize metadata: {}", e)))?;
    let metadata_hash = json_value_to_ruby(ruby, &metadata_value)?;
    set_hash_entry(ruby, &hash, "metadata", metadata_hash)?;

    let tables_array = ruby.ary_new();
    for table in result.tables {
        let table_hash = ruby.hash_new();

        let cells_array = ruby.ary_new();
        for row in table.cells {
            let row_array = ruby.ary_from_vec(row);
            cells_array.push(row_array)?;
        }
        table_hash.aset("cells", cells_array)?;

        table_hash.aset("markdown", table.markdown)?;

        table_hash.aset("page_number", table.page_number)?;

        tables_array.push(table_hash)?;
    }
    let tables_value = tables_array.into_value_with(ruby);
    set_hash_entry(ruby, &hash, "tables", tables_value)?;

    if let Some(langs) = result.detected_languages {
        let langs_array = ruby.ary_from_vec(langs);
        let langs_value = langs_array.into_value_with(ruby);
        set_hash_entry(ruby, &hash, "detected_languages", langs_value)?;
    } else {
        set_hash_entry(ruby, &hash, "detected_languages", ruby.qnil().as_value())?;
    }

    if let Some(chunks) = result.chunks {
        let chunks_array = ruby.ary_new();
        for chunk in chunks {
            let chunk_hash = ruby.hash_new();
            chunk_hash.aset("content", chunk.content)?;
            chunk_hash.aset("char_start", chunk.metadata.char_start)?;
            chunk_hash.aset("char_end", chunk.metadata.char_end)?;
            if let Some(token_count) = chunk.metadata.token_count {
                chunk_hash.aset("token_count", token_count)?;
            } else {
                chunk_hash.aset("token_count", ruby.qnil().as_value())?;
            }
            chunk_hash.aset("chunk_index", chunk.metadata.chunk_index)?;
            chunk_hash.aset("total_chunks", chunk.metadata.total_chunks)?;
            if let Some(embedding) = chunk.embedding {
                let embedding_array = ruby.ary_new();
                for value in embedding {
                    embedding_array.push(ruby.float_from_f64(value as f64).into_value_with(ruby))?;
                }
                chunk_hash.aset("embedding", embedding_array)?;
            } else {
                chunk_hash.aset("embedding", ruby.qnil().as_value())?;
            }
            chunks_array.push(chunk_hash)?;
        }
        let chunks_value = chunks_array.into_value_with(ruby);
        set_hash_entry(ruby, &hash, "chunks", chunks_value)?;
    } else {
        set_hash_entry(ruby, &hash, "chunks", ruby.qnil().as_value())?;
    }

    if let Some(images) = result.images {
        let images_array = ruby.ary_new();
        for image in images {
            let image_hash = ruby.hash_new();
            let data_value = ruby.str_from_slice(&image.data).into_value_with(ruby);
            image_hash.aset("data", data_value)?;
            image_hash.aset("format", image.format)?;
            image_hash.aset("image_index", image.image_index as i64)?;
            if let Some(page) = image.page_number {
                image_hash.aset("page_number", page as i64)?;
            } else {
                image_hash.aset("page_number", ruby.qnil().as_value())?;
            }
            if let Some(width) = image.width {
                image_hash.aset("width", width as i64)?;
            } else {
                image_hash.aset("width", ruby.qnil().as_value())?;
            }
            if let Some(height) = image.height {
                image_hash.aset("height", height as i64)?;
            } else {
                image_hash.aset("height", ruby.qnil().as_value())?;
            }
            if let Some(colorspace) = image.colorspace {
                image_hash.aset("colorspace", colorspace)?;
            } else {
                image_hash.aset("colorspace", ruby.qnil().as_value())?;
            }
            if let Some(bits) = image.bits_per_component {
                image_hash.aset("bits_per_component", bits as i64)?;
            } else {
                image_hash.aset("bits_per_component", ruby.qnil().as_value())?;
            }
            image_hash.aset(
                "is_mask",
                if image.is_mask {
                    ruby.qtrue().as_value()
                } else {
                    ruby.qfalse().as_value()
                },
            )?;
            if let Some(description) = image.description {
                image_hash.aset("description", description)?;
            } else {
                image_hash.aset("description", ruby.qnil().as_value())?;
            }
            if let Some(ocr_result) = image.ocr_result {
                let nested = extraction_result_to_ruby(ruby, *ocr_result)?;
                image_hash.aset("ocr_result", nested.into_value_with(ruby))?;
            } else {
                image_hash.aset("ocr_result", ruby.qnil().as_value())?;
            }
            images_array.push(image_hash)?;
        }
        set_hash_entry(ruby, &hash, "images", images_array.into_value_with(ruby))?;
    } else {
        set_hash_entry(ruby, &hash, "images", ruby.qnil().as_value())?;
    }

    Ok(hash)
}

/// Extract content from a file (synchronous).
///
/// @param path [String] Path to the file
/// @param mime_type [String, nil] Optional MIME type hint
/// @param options [Hash] Extraction configuration
/// @return [Hash] Extraction result with :content, :mime_type, :metadata, :tables, etc.
///
/// @example Basic usage
///   result = Kreuzberg.extract_file_sync("document.pdf")
///   puts result[:content]
///
/// @example With OCR
///   result = Kreuzberg.extract_file_sync("scanned.pdf", nil, force_ocr: true)
///
fn extract_file_sync(args: &[Value]) -> Result<RHash, Error> {
    let ruby = Ruby::get().expect("Ruby not initialized");
    let args = scan_args::<(String,), (Option<String>,), (), (), RHash, ()>(args)?;
    let (path,) = args.required;
    let (mime_type,) = args.optional;
    let opts = Some(args.keywords);

    let config = parse_extraction_config(&ruby, opts)?;

    let result = kreuzberg::extract_file_sync(&path, mime_type.as_deref(), &config).map_err(kreuzberg_error)?;

    extraction_result_to_ruby(&ruby, result)
}

/// Extract content from bytes (synchronous).
///
/// @param data [String] Binary data to extract
/// @param mime_type [String] MIME type of the data
/// @param options [Hash] Extraction configuration
/// @return [Hash] Extraction result
///
/// @example
///   data = File.binread("document.pdf")
///   result = Kreuzberg.extract_bytes_sync(data, "application/pdf")
///
fn extract_bytes_sync(args: &[Value]) -> Result<RHash, Error> {
    let ruby = Ruby::get().expect("Ruby not initialized");
    let args = scan_args::<(String, String), (), (), (), RHash, ()>(args)?;
    let (data, mime_type) = args.required;
    let opts = Some(args.keywords);

    let config = parse_extraction_config(&ruby, opts)?;

    let result = kreuzberg::extract_bytes_sync(data.as_bytes(), &mime_type, &config).map_err(kreuzberg_error)?;

    extraction_result_to_ruby(&ruby, result)
}

/// Batch extract content from multiple files (synchronous).
///
/// @param paths [Array<String>] List of file paths
/// @param options [Hash] Extraction configuration
/// @return [Array<Hash>] Array of extraction results
///
/// @example
///   paths = ["doc1.pdf", "doc2.docx", "doc3.xlsx"]
///   results = Kreuzberg.batch_extract_files_sync(paths)
///   results.each { |r| puts r[:content] }
///
fn batch_extract_files_sync(args: &[Value]) -> Result<RArray, Error> {
    let ruby = Ruby::get().expect("Ruby not initialized");
    let args = scan_args::<(RArray,), (), (), (), RHash, ()>(args)?;
    let (paths_array,) = args.required;
    let opts = Some(args.keywords);

    let config = parse_extraction_config(&ruby, opts)?;

    let paths: Vec<String> = paths_array.to_vec::<String>()?;

    let results = kreuzberg::batch_extract_file_sync(paths, &config).map_err(kreuzberg_error)?;

    let results_array = ruby.ary_new();
    for result in results {
        results_array.push(extraction_result_to_ruby(&ruby, result)?)?;
    }

    Ok(results_array)
}

/// Extract content from a file (asynchronous).
///
/// Note: Ruby doesn't have native async/await, so this uses a blocking Tokio runtime.
/// For true async behavior, use the synchronous version in a background thread.
///
/// @param path [String] Path to the file
/// @param mime_type [String, nil] Optional MIME type hint
/// @param options [Hash] Extraction configuration
/// @return [Hash] Extraction result
///
fn extract_file(args: &[Value]) -> Result<RHash, Error> {
    let ruby = Ruby::get().expect("Ruby not initialized");
    let args = scan_args::<(String,), (Option<String>,), (), (), RHash, ()>(args)?;
    let (path,) = args.required;
    let (mime_type,) = args.optional;
    let opts = Some(args.keywords);

    let config = parse_extraction_config(&ruby, opts)?;

    let runtime =
        tokio::runtime::Runtime::new().map_err(|e| runtime_error(format!("Failed to create Tokio runtime: {}", e)))?;

    let result = runtime
        .block_on(async { kreuzberg::extract_file(&path, mime_type.as_deref(), &config).await })
        .map_err(kreuzberg_error)?;

    extraction_result_to_ruby(&ruby, result)
}

/// Extract content from bytes (asynchronous).
///
/// @param data [String] Binary data
/// @param mime_type [String] MIME type
/// @param options [Hash] Extraction configuration
/// @return [Hash] Extraction result
///
fn extract_bytes(args: &[Value]) -> Result<RHash, Error> {
    let ruby = Ruby::get().expect("Ruby not initialized");
    let args = scan_args::<(String, String), (), (), (), RHash, ()>(args)?;
    let (data, mime_type) = args.required;
    let opts = Some(args.keywords);

    let config = parse_extraction_config(&ruby, opts)?;

    let runtime =
        tokio::runtime::Runtime::new().map_err(|e| runtime_error(format!("Failed to create Tokio runtime: {}", e)))?;

    let result = runtime
        .block_on(async { kreuzberg::extract_bytes(data.as_bytes(), &mime_type, &config).await })
        .map_err(kreuzberg_error)?;

    extraction_result_to_ruby(&ruby, result)
}

/// Batch extract content from multiple files (asynchronous).
///
/// @param paths [Array<String>] List of file paths
/// @param options [Hash] Extraction configuration
/// @return [Array<Hash>] Array of extraction results
///
fn batch_extract_files(args: &[Value]) -> Result<RArray, Error> {
    let ruby = Ruby::get().expect("Ruby not initialized");
    let args = scan_args::<(RArray,), (), (), (), RHash, ()>(args)?;
    let (paths_array,) = args.required;
    let opts = Some(args.keywords);

    let config = parse_extraction_config(&ruby, opts)?;

    let paths: Vec<String> = paths_array.to_vec::<String>()?;

    let runtime =
        tokio::runtime::Runtime::new().map_err(|e| runtime_error(format!("Failed to create Tokio runtime: {}", e)))?;

    let results = runtime
        .block_on(async { kreuzberg::batch_extract_file(paths, &config).await })
        .map_err(kreuzberg_error)?;

    let results_array = ruby.ary_new();
    for result in results {
        results_array.push(extraction_result_to_ruby(&ruby, result)?)?;
    }

    Ok(results_array)
}

/// Batch extract content from multiple byte arrays (synchronous).
///
/// @param bytes_array [Array<String>] List of binary data strings
/// @param mime_types [Array<String>] List of MIME types corresponding to each byte array
/// @param options [Hash] Extraction configuration
/// @return [Array<Hash>] Array of extraction results
///
/// @example
///   data1 = File.binread("document.pdf")
///   data2 = File.binread("invoice.docx")
///   results = Kreuzberg.batch_extract_bytes_sync([data1, data2], ["application/pdf", "application/vnd.openxmlformats-officedocument.wordprocessingml.document"])
///
fn batch_extract_bytes_sync(args: &[Value]) -> Result<RArray, Error> {
    let ruby = Ruby::get().expect("Ruby not initialized");
    let args = scan_args::<(RArray, RArray), (), (), (), RHash, ()>(args)?;
    let (bytes_array, mime_types_array) = args.required;
    let opts = Some(args.keywords);

    let config = parse_extraction_config(&ruby, opts)?;

    let bytes_vec: Vec<String> = bytes_array.to_vec::<String>()?;
    let mime_types: Vec<String> = mime_types_array.to_vec::<String>()?;

    if bytes_vec.len() != mime_types.len() {
        return Err(runtime_error(format!(
            "bytes_array and mime_types must have the same length: {} vs {}",
            bytes_vec.len(),
            mime_types.len()
        )));
    }

    let contents: Vec<(&[u8], &str)> = bytes_vec
        .iter()
        .zip(mime_types.iter())
        .map(|(bytes, mime)| (bytes.as_bytes(), mime.as_str()))
        .collect();

    let results = kreuzberg::batch_extract_bytes_sync(contents, &config).map_err(kreuzberg_error)?;

    let results_array = ruby.ary_new();
    for result in results {
        results_array.push(extraction_result_to_ruby(&ruby, result)?)?;
    }

    Ok(results_array)
}

/// Batch extract content from multiple byte arrays (asynchronous).
///
/// @param bytes_array [Array<String>] List of binary data strings
/// @param mime_types [Array<String>] List of MIME types corresponding to each byte array
/// @param options [Hash] Extraction configuration
/// @return [Array<Hash>] Array of extraction results
///
fn batch_extract_bytes(args: &[Value]) -> Result<RArray, Error> {
    let ruby = Ruby::get().expect("Ruby not initialized");
    let args = scan_args::<(RArray, RArray), (), (), (), RHash, ()>(args)?;
    let (bytes_array, mime_types_array) = args.required;
    let opts = Some(args.keywords);

    let config = parse_extraction_config(&ruby, opts)?;

    let bytes_vec: Vec<String> = bytes_array.to_vec::<String>()?;
    let mime_types: Vec<String> = mime_types_array.to_vec::<String>()?;

    if bytes_vec.len() != mime_types.len() {
        return Err(runtime_error(format!(
            "bytes_array and mime_types must have the same length: {} vs {}",
            bytes_vec.len(),
            mime_types.len()
        )));
    }

    let contents: Vec<(&[u8], &str)> = bytes_vec
        .iter()
        .zip(mime_types.iter())
        .map(|(bytes, mime)| (bytes.as_bytes(), mime.as_str()))
        .collect();

    let runtime =
        tokio::runtime::Runtime::new().map_err(|e| runtime_error(format!("Failed to create Tokio runtime: {}", e)))?;

    let results = runtime
        .block_on(async { kreuzberg::batch_extract_bytes(contents, &config).await })
        .map_err(kreuzberg_error)?;

    let results_array = ruby.ary_new();
    for result in results {
        results_array.push(extraction_result_to_ruby(&ruby, result)?)?;
    }

    Ok(results_array)
}

/// Clear all cache entries.
///
/// @return [void]
///
/// @example
///   Kreuzberg.clear_cache
///
fn ruby_clear_cache() -> Result<(), Error> {
    let cache_root = cache_root_dir()?;
    if !cache_root.exists() {
        return Ok(());
    }

    for dir in cache_directories(&cache_root)? {
        let Some(dir_str) = dir.to_str() else {
            return Err(runtime_error("Cache directory path contains non-UTF8 characters"));
        };

        // OSError/RuntimeError must bubble up - system errors need user reports ~keep
        kreuzberg::cache::clear_cache_directory(dir_str).map_err(kreuzberg_error)?;
    }

    Ok(())
}

/// Get cache statistics.
///
/// @return [Hash] Cache statistics with :total_entries and :total_size_bytes
///
/// @example
///   stats = Kreuzberg.cache_stats
///   puts "Cache entries: #{stats[:total_entries]}"
///   puts "Cache size: #{stats[:total_size_bytes]} bytes"
///
fn ruby_cache_stats() -> Result<RHash, Error> {
    let ruby = Ruby::get().expect("Ruby not initialized");

    let hash = ruby.hash_new();
    let cache_root = cache_root_dir()?;

    if !cache_root.exists() {
        hash.aset("total_entries", 0)?;
        hash.aset("total_size_bytes", 0)?;
        return Ok(hash);
    }

    let mut total_entries: usize = 0;
    let mut total_bytes: f64 = 0.0;

    for dir in cache_directories(&cache_root)? {
        let Some(dir_str) = dir.to_str() else {
            return Err(runtime_error("Cache directory path contains non-UTF8 characters"));
        };

        // OSError/RuntimeError must bubble up - system errors need user reports ~keep
        let stats = kreuzberg::cache::get_cache_metadata(dir_str).map_err(kreuzberg_error)?;
        total_entries += stats.total_files;
        total_bytes += stats.total_size_mb * 1024.0 * 1024.0;
    }

    set_hash_entry(
        &ruby,
        &hash,
        "total_entries",
        ruby.integer_from_u64(total_entries as u64).into_value_with(&ruby),
    )?;
    set_hash_entry(
        &ruby,
        &hash,
        "total_size_bytes",
        ruby.integer_from_u64(total_bytes.round() as u64).into_value_with(&ruby),
    )?;

    Ok(hash)
}

/// Register a post-processor plugin.
///
/// @param name [String] Unique identifier for the post-processor
/// @param processor [Proc] Ruby Proc/lambda that processes extraction results
/// @param priority [Integer] Execution priority (default: 50, higher = runs first)
/// @return [nil]
///
/// # Example
/// ```text
/// Kreuzberg.register_post_processor("uppercase", ->(result) {
///   result[:content] = result[:content].upcase
///   result
/// }, 100)
/// ```
fn register_post_processor(args: &[Value]) -> Result<(), Error> {
    let _ruby = Ruby::get().expect("Ruby not initialized");
    let args = scan_args::<(String, Value), (Option<i32>,), (), (), (), ()>(args)?;
    let (name, processor) = args.required;
    let (priority,) = args.optional;
    let priority = priority.unwrap_or(50);

    if !processor.respond_to("call", true)? {
        return Err(runtime_error("Post-processor must be a Proc or respond to 'call'"));
    }

    use async_trait::async_trait;
    use kreuzberg::plugins::{Plugin, PostProcessor, ProcessingStage};
    use std::sync::Arc;

    struct RubyPostProcessor {
        name: String,
        processor: GcGuardedValue,
    }

    unsafe impl Send for RubyPostProcessor {}
    unsafe impl Sync for RubyPostProcessor {}

    impl Plugin for RubyPostProcessor {
        fn name(&self) -> &str {
            &self.name
        }

        fn version(&self) -> String {
            "1.0.0".to_string()
        }

        fn initialize(&self) -> kreuzberg::Result<()> {
            Ok(())
        }

        fn shutdown(&self) -> kreuzberg::Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl PostProcessor for RubyPostProcessor {
        async fn process(
            &self,
            result: &mut kreuzberg::ExtractionResult,
            _config: &kreuzberg::ExtractionConfig,
        ) -> kreuzberg::Result<()> {
            let processor_name = self.name.clone();
            let processor = self.processor.value();
            let result_clone = result.clone();

            // Use block_in_place to avoid GVL deadlocks (same pattern as Python PostProcessor)
            // See crates/kreuzberg-py/README.md:151-158 for explanation
            // CRITICAL: spawn_blocking causes GVL deadlocks, must use block_in_place
            let updated_result = tokio::task::block_in_place(|| {
                let ruby = Ruby::get().expect("Ruby not initialized");
                let result_hash = extraction_result_to_ruby(&ruby, result_clone.clone()).map_err(|e| {
                    kreuzberg::KreuzbergError::Plugin {
                        message: format!("Failed to convert result to Ruby: {}", e),
                        plugin_name: processor_name.clone(),
                    }
                })?;

                let modified = processor
                    .funcall::<_, _, magnus::Value>("call", (result_hash,))
                    .map_err(|e| kreuzberg::KreuzbergError::Plugin {
                        message: format!("Ruby post-processor failed: {}", e),
                        plugin_name: processor_name.clone(),
                    })?;

                let modified_hash =
                    magnus::RHash::try_convert(modified).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                        message: format!("Post-processor must return a Hash: {}", e),
                        plugin_name: processor_name.clone(),
                    })?;

                let mut updated_result = result_clone;

                if let Some(content_val) = get_kw(&ruby, modified_hash, "content") {
                    let new_content =
                        String::try_convert(content_val).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                            message: format!("Failed to convert content: {}", e),
                            plugin_name: processor_name.clone(),
                        })?;
                    updated_result.content = new_content;
                }

                if let Some(mime_val) = get_kw(&ruby, modified_hash, "mime_type") {
                    let new_mime = String::try_convert(mime_val).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                        message: format!("Failed to convert mime_type: {}", e),
                        plugin_name: processor_name.clone(),
                    })?;
                    updated_result.mime_type = new_mime;
                }

                if let Some(metadata_val) = get_kw(&ruby, modified_hash, "metadata") {
                    if metadata_val.is_nil() {
                        updated_result.metadata = kreuzberg::types::Metadata::default();
                    } else {
                        let metadata_json =
                            ruby_value_to_json(metadata_val).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                                message: format!("Metadata must be JSON-serializable: {}", e),
                                plugin_name: processor_name.clone(),
                            })?;
                        let metadata: kreuzberg::types::Metadata =
                            serde_json::from_value(metadata_json).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                                message: format!("Failed to deserialize metadata: {}", e),
                                plugin_name: processor_name.clone(),
                            })?;
                        updated_result.metadata = metadata;
                    }
                }

                if let Some(tables_val) = get_kw(&ruby, modified_hash, "tables") {
                    let tables_json =
                        ruby_value_to_json(tables_val).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                            message: format!("Tables must be JSON-serializable: {}", e),
                            plugin_name: processor_name.clone(),
                        })?;
                    if tables_json.is_null() {
                        updated_result.tables.clear();
                    } else {
                        let tables: Vec<kreuzberg::types::Table> =
                            serde_json::from_value(tables_json).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                                message: format!("Failed to deserialize tables: {}", e),
                                plugin_name: processor_name.clone(),
                            })?;
                        updated_result.tables = tables;
                    }
                }

                if let Some(languages_val) = get_kw(&ruby, modified_hash, "detected_languages") {
                    if languages_val.is_nil() {
                        updated_result.detected_languages = None;
                    } else {
                        let langs_json =
                            ruby_value_to_json(languages_val).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                                message: format!("detected_languages must be JSON-serializable: {}", e),
                                plugin_name: processor_name.clone(),
                            })?;
                        let languages: Vec<String> =
                            serde_json::from_value(langs_json).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                                message: format!("Failed to deserialize detected_languages: {}", e),
                                plugin_name: processor_name.clone(),
                            })?;
                        updated_result.detected_languages = Some(languages);
                    }
                }

                if let Some(chunks_val) = get_kw(&ruby, modified_hash, "chunks") {
                    if chunks_val.is_nil() {
                        updated_result.chunks = None;
                    } else {
                        let chunks_json =
                            ruby_value_to_json(chunks_val).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                                message: format!("Chunks must be JSON-serializable: {}", e),
                                plugin_name: processor_name.clone(),
                            })?;
                        let chunks: Vec<kreuzberg::types::Chunk> =
                            serde_json::from_value(chunks_json).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                                message: format!("Failed to deserialize chunks: {}", e),
                                plugin_name: processor_name.clone(),
                            })?;
                        updated_result.chunks = Some(chunks);
                    }
                }

                Ok::<kreuzberg::ExtractionResult, kreuzberg::KreuzbergError>(updated_result)
            })?;

            *result = updated_result;
            Ok(())
        }

        fn processing_stage(&self) -> ProcessingStage {
            ProcessingStage::Late
        }
    }

    let processor_impl = Arc::new(RubyPostProcessor {
        name: name.clone(),
        processor: GcGuardedValue::new(processor),
    });

    let registry = kreuzberg::get_post_processor_registry();
    registry
        .write()
        .map_err(|e| runtime_error(format!("Failed to acquire registry lock: {}", e)))?
        .register(processor_impl, priority)
        .map_err(kreuzberg_error)?;

    Ok(())
}

/// Register a validator plugin.
///
/// @param name [String] Unique identifier for the validator
/// @param validator [Proc] Ruby Proc/lambda that validates extraction results
/// @param priority [Integer] Execution priority (default: 50, higher = runs first)
/// @return [nil]
///
/// # Example
/// ```text
/// Kreuzberg.register_validator("min_length", ->(result) {
///   raise "Content too short" if result[:content].length < 100
/// }, 100)
/// ```
fn register_validator(args: &[Value]) -> Result<(), Error> {
    let _ruby = Ruby::get().expect("Ruby not initialized");
    let args = scan_args::<(String, Value), (Option<i32>,), (), (), (), ()>(args)?;
    let (name, validator) = args.required;
    let (priority,) = args.optional;
    let priority = priority.unwrap_or(50);

    if !validator.respond_to("call", true)? {
        return Err(runtime_error("Validator must be a Proc or respond to 'call'"));
    }

    use async_trait::async_trait;
    use kreuzberg::plugins::{Plugin, Validator};
    use std::sync::Arc;

    struct RubyValidator {
        name: String,
        validator: GcGuardedValue,
        priority: i32,
    }

    unsafe impl Send for RubyValidator {}
    unsafe impl Sync for RubyValidator {}

    impl Plugin for RubyValidator {
        fn name(&self) -> &str {
            &self.name
        }

        fn version(&self) -> String {
            "1.0.0".to_string()
        }

        fn initialize(&self) -> kreuzberg::Result<()> {
            Ok(())
        }

        fn shutdown(&self) -> kreuzberg::Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl Validator for RubyValidator {
        async fn validate(
            &self,
            result: &kreuzberg::ExtractionResult,
            _config: &kreuzberg::ExtractionConfig,
        ) -> kreuzberg::Result<()> {
            let validator_name = self.name.clone();
            let validator = self.validator.value();
            let result_clone = result.clone();

            // Use block_in_place to avoid GVL deadlocks (same pattern as Python Validator)
            // See crates/kreuzberg-py/README.md:151-158 for explanation
            // CRITICAL: spawn_blocking causes GVL deadlocks, must use block_in_place
            tokio::task::block_in_place(|| {
                let ruby = Ruby::get().expect("Ruby not initialized");
                let result_hash =
                    extraction_result_to_ruby(&ruby, result_clone).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                        message: format!("Failed to convert result to Ruby: {}", e),
                        plugin_name: validator_name.clone(),
                    })?;

                validator
                    .funcall::<_, _, magnus::Value>("call", (result_hash,))
                    .map_err(|e| kreuzberg::KreuzbergError::Validation {
                        message: format!("Validation failed: {}", e),
                        source: None,
                    })?;

                Ok(())
            })
        }

        fn priority(&self) -> i32 {
            self.priority
        }
    }

    let validator_impl = Arc::new(RubyValidator {
        name: name.clone(),
        validator: GcGuardedValue::new(validator),
        priority,
    });

    let registry = kreuzberg::get_validator_registry();
    registry
        .write()
        .map_err(|e| runtime_error(format!("Failed to acquire registry lock: {}", e)))?
        .register(validator_impl)
        .map_err(kreuzberg_error)?;

    Ok(())
}

/// Register an OCR backend plugin.
///
/// @param name [String] Unique identifier for the OCR backend
/// @param backend [Object] Ruby object implementing OCR backend interface
/// @return [nil]
///
/// # Example
/// ```text
/// class CustomOcr
///   def process_image(image_bytes, language)
///     # Return extracted text
///     "Extracted text"
///   end
///
///   def supports_language?(lang)
///     %w[eng deu fra].include?(lang)
///   end
/// end
///
/// Kreuzberg.register_ocr_backend("custom", CustomOcr.new)
/// ```
fn register_ocr_backend(name: String, backend: Value) -> Result<(), Error> {
    if !backend.respond_to("name", true)? {
        return Err(runtime_error("OCR backend must respond to 'name'"));
    }
    if !backend.respond_to("process_image", true)? {
        return Err(runtime_error("OCR backend must respond to 'process_image'"));
    }

    use async_trait::async_trait;
    use kreuzberg::plugins::{OcrBackend, OcrBackendType, Plugin};
    use std::sync::Arc;

    struct RubyOcrBackend {
        name: String,
        backend: GcGuardedValue,
    }

    unsafe impl Send for RubyOcrBackend {}
    unsafe impl Sync for RubyOcrBackend {}

    impl Plugin for RubyOcrBackend {
        fn name(&self) -> &str {
            &self.name
        }

        fn version(&self) -> String {
            "1.0.0".to_string()
        }

        fn initialize(&self) -> kreuzberg::Result<()> {
            Ok(())
        }

        fn shutdown(&self) -> kreuzberg::Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl OcrBackend for RubyOcrBackend {
        async fn process_image(
            &self,
            image_bytes: &[u8],
            config: &kreuzberg::OcrConfig,
        ) -> kreuzberg::Result<kreuzberg::ExtractionResult> {
            let ruby = Ruby::get().expect("Ruby not initialized");
            let image_str = ruby.str_from_slice(image_bytes);

            let config_hash = ocr_config_to_ruby_hash(&ruby, config).map_err(|e| kreuzberg::KreuzbergError::Ocr {
                message: format!("Failed to convert OCR config: {}", e),
                source: None,
            })?;

            let response = self
                .backend
                .value()
                .funcall::<_, _, Value>("process_image", (image_str, config_hash.into_value_with(&ruby)))
                .map_err(|e| kreuzberg::KreuzbergError::Ocr {
                    message: format!("Ruby OCR backend failed: {}", e),
                    source: None,
                })?;

            let text = String::try_convert(response).map_err(|e| kreuzberg::KreuzbergError::Ocr {
                message: format!("OCR backend must return a String: {}", e),
                source: None,
            })?;

            Ok(kreuzberg::ExtractionResult {
                content: text,
                mime_type: "text/plain".to_string(),
                metadata: kreuzberg::types::Metadata::default(),
                tables: vec![],
                detected_languages: None,
                chunks: None,
                images: None,
            })
        }

        fn supports_language(&self, lang: &str) -> bool {
            match self.backend.value().respond_to("supports_language?", true) {
                Ok(true) => self
                    .backend
                    .value()
                    .funcall::<_, _, bool>("supports_language?", (lang,))
                    .unwrap_or(true),
                _ => true,
            }
        }

        fn backend_type(&self) -> OcrBackendType {
            OcrBackendType::Custom
        }
    }

    let backend_impl = Arc::new(RubyOcrBackend {
        name: name.clone(),
        backend: GcGuardedValue::new(backend),
    });

    let registry = kreuzberg::get_ocr_backend_registry();
    registry
        .write()
        .map_err(|e| runtime_error(format!("Failed to acquire registry lock: {}", e)))?
        .register(backend_impl)
        .map_err(kreuzberg_error)?;

    Ok(())
}

/// Unregister a post-processor plugin.
///
/// @param name [String] Name of the post-processor to remove
/// @return [nil]
///
fn unregister_post_processor(name: String) -> Result<(), Error> {
    let registry = kreuzberg::get_post_processor_registry();
    registry
        .write()
        .map_err(|e| runtime_error(format!("Failed to acquire registry lock: {}", e)))?
        .remove(&name)
        .map_err(kreuzberg_error)?;
    Ok(())
}

/// Unregister a validator plugin.
///
/// @param name [String] Name of the validator to remove
/// @return [nil]
///
fn unregister_validator(name: String) -> Result<(), Error> {
    let registry = kreuzberg::get_validator_registry();
    registry
        .write()
        .map_err(|e| runtime_error(format!("Failed to acquire registry lock: {}", e)))?
        .remove(&name)
        .map_err(kreuzberg_error)?;
    Ok(())
}

/// Clear all registered post-processors.
///
/// @return [nil]
///
fn clear_post_processors() -> Result<(), Error> {
    let registry = kreuzberg::get_post_processor_registry();
    registry
        .write()
        .map_err(|e| runtime_error(format!("Failed to acquire registry lock: {}", e)))?
        .shutdown_all()
        .map_err(kreuzberg_error)?;
    Ok(())
}

/// Clear all registered validators.
///
/// @return [nil]
///
fn clear_validators() -> Result<(), Error> {
    let registry = kreuzberg::get_validator_registry();
    registry
        .write()
        .map_err(|e| runtime_error(format!("Failed to acquire registry lock: {}", e)))?
        .shutdown_all()
        .map_err(kreuzberg_error)?;
    Ok(())
}

/// List all registered validators.
///
/// @return [Array<String>] Array of validator names
///
fn list_validators() -> Result<Vec<String>, Error> {
    let registry = kreuzberg::get_validator_registry();
    let validators = registry
        .read()
        .map_err(|e| runtime_error(format!("Failed to acquire registry lock: {}", e)))?
        .list();
    Ok(validators)
}

/// List all registered post-processors.
///
/// @return [Array<String>] Array of post-processor names
///
fn list_post_processors() -> Result<Vec<String>, Error> {
    let registry = kreuzberg::get_post_processor_registry();
    let processors = registry
        .read()
        .map_err(|e| runtime_error(format!("Failed to acquire registry lock: {}", e)))?
        .list();
    Ok(processors)
}

/// Unregister an OCR backend by name.
///
/// Removes a previously registered OCR backend from the global registry.
///
/// @param name [String] Backend name to unregister
/// @return [void]
///
/// @example
///   Kreuzberg.unregister_ocr_backend("my_ocr")
///
fn unregister_ocr_backend(name: String) -> Result<(), Error> {
    kreuzberg::plugins::unregister_ocr_backend(&name).map_err(|e| runtime_error(e.to_string()))
}

/// List all registered OCR backend names.
///
/// Returns an array of all OCR backend names currently registered in the global registry.
///
/// @return [Array<String>] Array of OCR backend names
///
/// @example
///   backends = Kreuzberg.list_ocr_backends
///   #=> ["tesseract", "my_custom_ocr"]
///
fn list_ocr_backends() -> Result<Vec<String>, Error> {
    kreuzberg::plugins::list_ocr_backends().map_err(|e| runtime_error(e.to_string()))
}

/// Clear all registered OCR backends.
///
/// Removes all OCR backends from the global registry and calls their shutdown methods.
///
/// @return [void]
///
/// @example
///   Kreuzberg.clear_ocr_backends
///
fn clear_ocr_backends() -> Result<(), Error> {
    kreuzberg::plugins::clear_ocr_backends().map_err(|e| runtime_error(e.to_string()))
}

/// List all registered document extractor names.
///
/// Returns an array of all document extractor names currently registered in the global registry.
///
/// @return [Array<String>] Array of document extractor names
///
/// @example
///   extractors = Kreuzberg.list_document_extractors
///   #=> ["pdf", "docx", "txt"]
///
fn list_document_extractors() -> Result<Vec<String>, Error> {
    kreuzberg::plugins::list_extractors().map_err(|e| runtime_error(e.to_string()))
}

/// Unregister a document extractor by name.
///
/// Removes a previously registered document extractor from the global registry.
///
/// @param name [String] Extractor name to unregister
/// @return [void]
///
/// @example
///   Kreuzberg.unregister_document_extractor("my_extractor")
///
fn unregister_document_extractor(name: String) -> Result<(), Error> {
    kreuzberg::plugins::unregister_extractor(&name).map_err(|e| runtime_error(e.to_string()))
}

/// Clear all registered document extractors.
///
/// Removes all document extractors from the global registry and calls their shutdown methods.
///
/// @return [void]
///
/// @example
///   Kreuzberg.clear_document_extractors
///
fn clear_document_extractors() -> Result<(), Error> {
    kreuzberg::plugins::clear_extractors().map_err(|e| runtime_error(e.to_string()))
}

/// Validate that a MIME type is supported.
///
/// @param mime_type [String] The MIME type to validate
/// @return [String] The validated MIME type (may be normalized)
///
/// @example
///   validated = Kreuzberg.validate_mime_type("application/pdf")
///   #=> "application/pdf"
///
/// @example Validate image MIME type
///   validated = Kreuzberg.validate_mime_type("image/jpeg")
///   #=> "image/jpeg"
///
fn validate_mime_type_native(mime_type: String) -> Result<String, Error> {
    kreuzberg::validate_mime_type(&mime_type).map_err(kreuzberg_error)
}

/// Detect MIME type from byte content.
///
/// Uses magic byte detection to determine the MIME type of content.
///
/// @param bytes [String] The byte content to analyze
/// @return [String] Detected MIME type
///
/// @example
///   pdf_bytes = "%PDF-1.4\n"
///   mime = Kreuzberg.detect_mime_type(pdf_bytes)
///   #=> "application/pdf"
///
fn detect_mime_type_from_bytes(bytes: String) -> Result<String, Error> {
    let mime_type = kreuzberg::detect_mime_type_from_bytes(bytes.as_bytes()).map_err(kreuzberg_error)?;
    Ok(mime_type)
}

/// Detect MIME type from a file path.
///
/// Detects MIME type by reading the file's magic bytes.
///
/// @param path [String] Path to the file
/// @return [String] Detected MIME type
///
/// @example
///   mime = Kreuzberg.detect_mime_type_from_path("document.pdf")
///   #=> "application/pdf"
///
fn detect_mime_type_from_path_native(path: String) -> Result<String, Error> {
    let content = fs::read(&path).map_err(KreuzbergError::Io).map_err(kreuzberg_error)?;
    let mime_type = kreuzberg::detect_mime_type_from_bytes(&content).map_err(kreuzberg_error)?;
    Ok(mime_type)
}

/// Get file extensions for a given MIME type.
///
/// Returns an array of file extensions commonly associated with the MIME type.
///
/// @param mime_type [String] The MIME type
/// @return [Array<String>] Array of file extensions (without dots)
///
/// @example
///   exts = Kreuzberg.get_extensions_for_mime("application/pdf")
///   #=> ["pdf"]
///
/// @example
///   exts = Kreuzberg.get_extensions_for_mime("image/jpeg")
///   #=> ["jpg", "jpeg"]
///
fn get_extensions_for_mime_native(mime_type: String) -> Result<Vec<String>, Error> {
    kreuzberg::get_extensions_for_mime(&mime_type).map_err(kreuzberg_error)
}

/// List all available embedding preset names.
///
/// Returns an array of preset names that can be used with get_embedding_preset.
///
/// # Returns
///
/// Array of 4 preset names: ["fast", "balanced", "quality", "multilingual"]
///
/// # Example
///
/// ```ruby
/// require 'kreuzberg'
///
/// presets = Kreuzberg.list_embedding_presets
/// puts presets  # => ["fast", "balanced", "quality", "multilingual"]
/// ```
fn list_embedding_presets(ruby: &Ruby) -> Result<RArray, Error> {
    let presets = kreuzberg::embeddings::list_presets();
    let array = ruby.ary_new();
    for name in presets {
        array.push(name)?;
    }
    Ok(array)
}

/// Get a specific embedding preset by name.
///
/// Returns a preset configuration hash, or nil if the preset name is not found.
///
/// # Arguments
///
/// * `name` - The preset name (case-sensitive)
///
/// # Returns
///
/// Hash with preset configuration or nil if not found
///
/// Available presets:
/// - "fast": AllMiniLML6V2Q (384 dimensions) - Quick prototyping, low-latency
/// - "balanced": BGEBaseENV15 (768 dimensions) - General-purpose RAG
/// - "quality": BGELargeENV15 (1024 dimensions) - High-quality embeddings
/// - "multilingual": MultilingualE5Base (768 dimensions) - Multi-language support
///
/// # Example
///
/// ```ruby
/// require 'kreuzberg'
///
/// preset = Kreuzberg.get_embedding_preset("balanced")
/// if preset
///   puts "Model: #{preset[:model_name]}, Dims: #{preset[:dimensions]}"
///   # => Model: BGEBaseENV15, Dims: 768
/// end
/// ```
fn get_embedding_preset(ruby: &Ruby, name: String) -> Result<Value, Error> {
    let preset = kreuzberg::embeddings::get_preset(&name);

    match preset {
        Some(preset) => {
            let hash = ruby.hash_new();

            set_hash_entry(ruby, &hash, "name", ruby.str_new(preset.name).as_value())?;
            set_hash_entry(ruby, &hash, "chunk_size", preset.chunk_size.into_value_with(ruby))?;
            set_hash_entry(ruby, &hash, "overlap", preset.overlap.into_value_with(ruby))?;

            // Note: When embeddings feature is enabled in kreuzberg, the model field is EmbeddingModel
            // Since Ruby bindings typically build with all features, we use the model field and format it.
            let model_name = format!("{:?}", preset.model);

            set_hash_entry(ruby, &hash, "model_name", ruby.str_new(&model_name).as_value())?;
            set_hash_entry(ruby, &hash, "dimensions", preset.dimensions.into_value_with(ruby))?;
            set_hash_entry(ruby, &hash, "description", ruby.str_new(preset.description).as_value())?;

            Ok(hash.as_value())
        }
        None => Ok(ruby.qnil().as_value()),
    }
}

/// Initialize the Kreuzberg Ruby module
#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("Kreuzberg")?;

    module.define_module_function("extract_file_sync", function!(extract_file_sync, -1))?;
    module.define_module_function("extract_bytes_sync", function!(extract_bytes_sync, -1))?;
    module.define_module_function("batch_extract_files_sync", function!(batch_extract_files_sync, -1))?;
    module.define_module_function("batch_extract_bytes_sync", function!(batch_extract_bytes_sync, -1))?;

    module.define_module_function("extract_file", function!(extract_file, -1))?;
    module.define_module_function("extract_bytes", function!(extract_bytes, -1))?;
    module.define_module_function("batch_extract_files", function!(batch_extract_files, -1))?;
    module.define_module_function("batch_extract_bytes", function!(batch_extract_bytes, -1))?;

    module.define_module_function("clear_cache", function!(ruby_clear_cache, 0))?;
    module.define_module_function("cache_stats", function!(ruby_cache_stats, 0))?;

    module.define_module_function("register_post_processor", function!(register_post_processor, -1))?;
    module.define_module_function("register_validator", function!(register_validator, -1))?;
    module.define_module_function("register_ocr_backend", function!(register_ocr_backend, 2))?;
    module.define_module_function("unregister_post_processor", function!(unregister_post_processor, 1))?;
    module.define_module_function("unregister_validator", function!(unregister_validator, 1))?;
    module.define_module_function("clear_post_processors", function!(clear_post_processors, 0))?;
    module.define_module_function("clear_validators", function!(clear_validators, 0))?;
    module.define_module_function("list_post_processors", function!(list_post_processors, 0))?;
    module.define_module_function("list_validators", function!(list_validators, 0))?;
    module.define_module_function("unregister_ocr_backend", function!(unregister_ocr_backend, 1))?;
    module.define_module_function("list_ocr_backends", function!(list_ocr_backends, 0))?;
    module.define_module_function("clear_ocr_backends", function!(clear_ocr_backends, 0))?;
    module.define_module_function("list_document_extractors", function!(list_document_extractors, 0))?;
    module.define_module_function(
        "unregister_document_extractor",
        function!(unregister_document_extractor, 1),
    )?;
    module.define_module_function("clear_document_extractors", function!(clear_document_extractors, 0))?;

    module.define_module_function("_config_from_file_native", function!(config_from_file, 1))?;
    module.define_module_function("_config_discover_native", function!(config_discover, 0))?;

    module.define_module_function("detect_mime_type", function!(detect_mime_type_from_bytes, 1))?;
    module.define_module_function(
        "detect_mime_type_from_path",
        function!(detect_mime_type_from_path_native, 1),
    )?;
    module.define_module_function("get_extensions_for_mime", function!(get_extensions_for_mime_native, 1))?;
    module.define_module_function("validate_mime_type", function!(validate_mime_type_native, 1))?;

    module.define_module_function("list_embedding_presets", function!(list_embedding_presets, 0))?;
    module.define_module_function("get_embedding_preset", function!(get_embedding_preset, 1))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ruby_clear_cache_clears_directory() {
        use std::fs;
        use std::path::PathBuf;

        let thread_id = std::thread::current().id();
        let cache_dir = PathBuf::from(format!("/tmp/kreuzberg_test_clear_{:?}", thread_id));

        let _ = fs::remove_dir_all(&cache_dir);

        fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");

        let test_file = cache_dir.join("test_cache.msgpack");
        fs::write(&test_file, b"test data").expect("Failed to write test file");

        assert!(test_file.exists(), "Test file should exist before clear");

        let cache_dir_str = cache_dir.to_str().expect("Cache dir must be valid UTF-8");
        let result = kreuzberg::cache::clear_cache_directory(cache_dir_str);

        assert!(result.is_ok(), "Cache clear should succeed");
        let (removed, _) = result.unwrap();
        assert_eq!(removed, 1, "Should remove one file");

        assert!(!test_file.exists(), "Test file should be removed after clear");

        let _ = fs::remove_dir_all(&cache_dir);
    }

    #[test]
    fn test_ruby_cache_stats_returns_correct_structure() {
        use std::fs;
        use std::path::PathBuf;

        let thread_id = std::thread::current().id();
        let cache_dir = PathBuf::from(format!("/tmp/kreuzberg_test_stats_{:?}", thread_id));

        let _ = fs::remove_dir_all(&cache_dir);

        fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");

        let test_file1 = cache_dir.join("test1.msgpack");
        let test_file2 = cache_dir.join("test2.msgpack");
        fs::write(&test_file1, b"test data 1").expect("Failed to write test file 1");
        fs::write(&test_file2, b"test data 2").expect("Failed to write test file 2");

        let cache_dir_str = cache_dir.to_str().expect("Cache dir must be valid UTF-8");
        let stats = kreuzberg::cache::get_cache_metadata(cache_dir_str);

        assert!(stats.is_ok(), "Cache stats should succeed");
        let stats = stats.unwrap();

        assert_eq!(stats.total_files, 2, "Should report 2 files");
        assert!(stats.total_size_mb > 0.0, "Total size should be greater than 0");
        assert!(
            stats.available_space_mb > 0.0,
            "Available space should be greater than 0"
        );

        let _ = fs::remove_dir_all(&cache_dir);
    }

    #[test]
    fn test_ruby_cache_stats_converts_mb_to_bytes() {
        let size_mb = 1.5;
        let size_bytes = (size_mb * 1024.0 * 1024.0) as u64;
        assert_eq!(size_bytes, 1_572_864, "Should convert MB to bytes correctly");
    }

    #[test]
    fn test_ruby_clear_cache_handles_empty_directory() {
        use std::fs;
        use std::path::PathBuf;

        let thread_id = std::thread::current().id();
        let cache_dir = PathBuf::from(format!("/tmp/kreuzberg_test_empty_{:?}", thread_id));

        let _ = fs::remove_dir_all(&cache_dir);

        fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");

        let cache_dir_str = cache_dir.to_str().expect("Cache dir must be valid UTF-8");
        let result = kreuzberg::cache::clear_cache_directory(cache_dir_str);

        assert!(result.is_ok(), "Should handle empty directory");
        let (removed, freed) = result.unwrap();
        assert_eq!(removed, 0, "Should remove 0 files from empty directory");
        assert_eq!(freed, 0.0, "Should free 0 MB from empty directory");

        let _ = fs::remove_dir_all(&cache_dir);
    }

    #[test]
    fn test_image_extraction_config_conversion() {
        let config = ImageExtractionConfig {
            extract_images: true,
            target_dpi: 300,
            max_image_dimension: 4096,
            auto_adjust_dpi: true,
            min_dpi: 72,
            max_dpi: 600,
        };

        assert!(config.extract_images);
        assert_eq!(config.target_dpi, 300);
        assert_eq!(config.max_image_dimension, 4096);
        assert!(config.auto_adjust_dpi);
        assert_eq!(config.min_dpi, 72);
        assert_eq!(config.max_dpi, 600);
    }

    #[test]
    fn test_image_preprocessing_config_conversion() {
        let config = ImagePreprocessingConfig {
            target_dpi: 300,
            auto_rotate: true,
            deskew: true,
            denoise: false,
            contrast_enhance: false,
            binarization_method: "otsu".to_string(),
            invert_colors: false,
        };

        assert_eq!(config.target_dpi, 300);
        assert!(config.auto_rotate);
        assert!(config.deskew);
        assert!(!config.denoise);
        assert!(!config.contrast_enhance);
        assert_eq!(config.binarization_method, "otsu");
        assert!(!config.invert_colors);
    }

    #[test]
    fn test_postprocessor_config_conversion() {
        let config = PostProcessorConfig {
            enabled: true,
            enabled_processors: Some(vec!["processor1".to_string(), "processor2".to_string()]),
            disabled_processors: None,
        };

        assert!(config.enabled);
        assert!(config.enabled_processors.is_some());
        assert_eq!(config.enabled_processors.unwrap().len(), 2);
        assert!(config.disabled_processors.is_none());
    }

    #[test]
    fn test_token_reduction_config_conversion() {
        let config = TokenReductionConfig {
            mode: "moderate".to_string(),
            preserve_important_words: true,
        };

        assert_eq!(config.mode, "moderate");
        assert!(config.preserve_important_words);
    }

    #[test]
    fn test_extraction_config_with_new_fields() {
        let config = ExtractionConfig {
            images: Some(ImageExtractionConfig {
                extract_images: true,
                target_dpi: 300,
                max_image_dimension: 4096,
                auto_adjust_dpi: true,
                min_dpi: 72,
                max_dpi: 600,
            }),
            postprocessor: Some(PostProcessorConfig {
                enabled: true,
                enabled_processors: None,
                disabled_processors: None,
            }),
            token_reduction: Some(TokenReductionConfig {
                mode: "light".to_string(),
                preserve_important_words: true,
            }),
            ..Default::default()
        };

        assert!(config.images.is_some());
        assert!(config.postprocessor.is_some());
        assert!(config.token_reduction.is_some());
    }
}
