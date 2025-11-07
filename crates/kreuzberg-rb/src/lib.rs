//! Kreuzberg Ruby Bindings (Magnus)
//!
//! High-performance document intelligence framework bindings for Ruby.
//! Provides extraction, OCR, chunking, and language detection for 30+ file formats.

use kreuzberg::{
    ChunkingConfig, ExtractionConfig, ExtractionResult as RustExtractionResult, ImageExtractionConfig,
    ImagePreprocessingConfig, KreuzbergError, LanguageDetectionConfig, OcrConfig, PdfConfig, PostProcessorConfig,
    TokenReductionConfig,
};
use magnus::prelude::*;
use magnus::{Error, RArray, RHash, Ruby, Symbol, TryConvert, Value, function, scan_args::scan_args};

/// Convert Kreuzberg errors to Ruby exceptions
fn kreuzberg_error(err: KreuzbergError) -> Error {
    let ruby = Ruby::get().expect("Ruby not initialized");
    match err {
        KreuzbergError::Validation { message, .. } => Error::new(ruby.exception_arg_error(), message),
        KreuzbergError::Parsing { message, .. } => {
            Error::new(ruby.exception_runtime_error(), format!("ParsingError: {}", message))
        }
        KreuzbergError::Ocr { message, .. } => {
            Error::new(ruby.exception_runtime_error(), format!("OCRError: {}", message))
        }
        KreuzbergError::MissingDependency(message) => Error::new(
            ruby.exception_runtime_error(),
            format!("MissingDependencyError: {}", message),
        ),
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
    let sym = ruby.intern(name);
    hash.get(sym).or_else(|| hash.get(name))
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

    let config = OcrConfig {
        backend,
        language,
        tesseract_config: None,
    };

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

    let config = ChunkingConfig {
        max_chars,
        max_overlap,
        embedding: None, // TODO: Support embedding config from Ruby
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

/// Parse ExtractionConfig from Ruby Hash
fn parse_extraction_config(ruby: &Ruby, opts: Option<RHash>) -> Result<ExtractionConfig, Error> {
    let mut config = ExtractionConfig::default();

    if let Some(hash) = opts {
        // use_cache
        if let Some(val) = get_kw(ruby, hash, "use_cache") {
            config.use_cache = bool::try_convert(val)?;
        }

        // enable_quality_processing
        if let Some(val) = get_kw(ruby, hash, "enable_quality_processing") {
            config.enable_quality_processing = bool::try_convert(val)?;
        }

        // force_ocr
        if let Some(val) = get_kw(ruby, hash, "force_ocr") {
            config.force_ocr = bool::try_convert(val)?;
        }

        // ocr (Hash)
        if let Some(val) = get_kw(ruby, hash, "ocr")
            && !val.is_nil()
        {
            let ocr_hash = RHash::try_convert(val)?;
            config.ocr = Some(parse_ocr_config(ruby, ocr_hash)?);
        }

        // chunking (Hash)
        if let Some(val) = get_kw(ruby, hash, "chunking")
            && !val.is_nil()
        {
            let chunking_hash = RHash::try_convert(val)?;
            config.chunking = Some(parse_chunking_config(ruby, chunking_hash)?);
        }

        // language_detection (Hash)
        if let Some(val) = get_kw(ruby, hash, "language_detection")
            && !val.is_nil()
        {
            let lang_hash = RHash::try_convert(val)?;
            config.language_detection = Some(parse_language_detection_config(ruby, lang_hash)?);
        }

        // pdf_options (Hash)
        if let Some(val) = get_kw(ruby, hash, "pdf_options")
            && !val.is_nil()
        {
            let pdf_hash = RHash::try_convert(val)?;
            config.pdf_options = Some(parse_pdf_config(ruby, pdf_hash)?);
        }

        // images (Hash)
        if let Some(val) = get_kw(ruby, hash, "images")
            && !val.is_nil()
        {
            let images_hash = RHash::try_convert(val)?;
            config.images = Some(parse_image_extraction_config(ruby, images_hash)?);
        }

        // image_preprocessing (Hash)
        // Note: This field doesn't exist in ExtractionConfig, but would be part of OcrConfig
        // Skipping for now as it's not in the top-level config

        // postprocessor (Hash)
        if let Some(val) = get_kw(ruby, hash, "postprocessor")
            && !val.is_nil()
        {
            let postprocessor_hash = RHash::try_convert(val)?;
            config.postprocessor = Some(parse_postprocessor_config(ruby, postprocessor_hash)?);
        }

        // token_reduction (Hash)
        if let Some(val) = get_kw(ruby, hash, "token_reduction")
            && !val.is_nil()
        {
            let token_reduction_hash = RHash::try_convert(val)?;
            config.token_reduction = Some(parse_token_reduction_config(ruby, token_reduction_hash)?);
        }
    }

    Ok(config)
}

/// Convert Rust ExtractionResult to Ruby Hash
fn extraction_result_to_ruby(ruby: &Ruby, result: RustExtractionResult) -> Result<RHash, Error> {
    let hash = ruby.hash_new();

    // content (String)
    hash.aset(ruby.intern("content"), result.content)?;

    // mime_type (String)
    hash.aset(ruby.intern("mime_type"), result.mime_type)?;

    // metadata (Hash) - serialize to JSON string for simplicity
    let metadata_json = serde_json::to_string(&result.metadata)
        .map_err(|e| runtime_error(format!("Failed to serialize metadata: {}", e)))?;
    hash.aset(ruby.intern("metadata_json"), metadata_json)?;

    // tables (Array of Hashes)
    let tables_array = ruby.ary_new();
    for table in result.tables {
        let table_hash = ruby.hash_new();

        // cells (Array of Arrays of Strings)
        let cells_array = ruby.ary_new();
        for row in table.cells {
            let row_array = ruby.ary_from_vec(row);
            cells_array.push(row_array)?;
        }
        table_hash.aset(ruby.intern("cells"), cells_array)?;

        // markdown (String)
        table_hash.aset(ruby.intern("markdown"), table.markdown)?;

        // page_number (Integer)
        table_hash.aset(ruby.intern("page_number"), table.page_number)?;

        tables_array.push(table_hash)?;
    }
    hash.aset(ruby.intern("tables"), tables_array)?;

    // detected_languages (Array of Strings or nil)
    if let Some(langs) = result.detected_languages {
        let langs_array = ruby.ary_from_vec(langs);
        hash.aset(ruby.intern("detected_languages"), langs_array)?;
    } else {
        hash.aset(ruby.intern("detected_languages"), ruby.qnil())?;
    }

    // chunks (Array of Hashes or nil)
    if let Some(chunks) = result.chunks {
        let chunks_array = ruby.ary_new();
        for chunk in chunks {
            let chunk_hash = ruby.hash_new();
            chunk_hash.aset(ruby.intern("content"), chunk.content)?;
            chunk_hash.aset(ruby.intern("char_start"), chunk.metadata.char_start)?;
            chunk_hash.aset(ruby.intern("char_end"), chunk.metadata.char_end)?;
            if let Some(token_count) = chunk.metadata.token_count {
                chunk_hash.aset(ruby.intern("token_count"), token_count)?;
            }
            chunks_array.push(chunk_hash)?;
        }
        hash.aset(ruby.intern("chunks"), chunks_array)?;
    } else {
        hash.aset(ruby.intern("chunks"), ruby.qnil())?;
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

    // Convert Ruby array to Vec<String>
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

    // Use Tokio runtime to block on async function
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

    // Convert Ruby arrays to Vecs
    let bytes_vec: Vec<String> = bytes_array.to_vec::<String>()?;
    let mime_types: Vec<String> = mime_types_array.to_vec::<String>()?;

    // Validate that arrays have the same length
    if bytes_vec.len() != mime_types.len() {
        return Err(runtime_error(format!(
            "bytes_array and mime_types must have the same length: {} vs {}",
            bytes_vec.len(),
            mime_types.len()
        )));
    }

    // Convert to Vec<(&[u8], &str)> format expected by kreuzberg
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

    // Convert Ruby arrays to Vecs
    let bytes_vec: Vec<String> = bytes_array.to_vec::<String>()?;
    let mime_types: Vec<String> = mime_types_array.to_vec::<String>()?;

    // Validate that arrays have the same length
    if bytes_vec.len() != mime_types.len() {
        return Err(runtime_error(format!(
            "bytes_array and mime_types must have the same length: {} vs {}",
            bytes_vec.len(),
            mime_types.len()
        )));
    }

    // Convert to Vec<(&[u8], &str)> format expected by kreuzberg
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
    let cache_dir = std::env::current_dir()
        .map_err(|e| runtime_error(format!("Failed to get current directory: {}", e)))?
        .join(".kreuzberg");

    let cache_dir_str = cache_dir
        .to_str()
        .ok_or_else(|| runtime_error("Cache directory path contains non-UTF8 characters"))?;

    // OSError/RuntimeError must bubble up - system errors need user reports ~keep
    kreuzberg::cache::clear_cache_directory(cache_dir_str).map_err(kreuzberg_error)?;

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

    let cache_dir = std::env::current_dir()
        .map_err(|e| runtime_error(format!("Failed to get current directory: {}", e)))?
        .join(".kreuzberg");

    let cache_dir_str = cache_dir
        .to_str()
        .ok_or_else(|| runtime_error("Cache directory path contains non-UTF8 characters"))?;

    // OSError/RuntimeError must bubble up - system errors need user reports ~keep
    let stats = kreuzberg::cache::get_cache_metadata(cache_dir_str).map_err(kreuzberg_error)?;

    let hash = ruby.hash_new();
    hash.aset(ruby.intern("total_entries"), stats.total_files)?;
    // Convert MB to bytes for consistency with Ruby API
    // Note: Loses sub-byte precision, but acceptable since stats are approximate
    let total_size_bytes = (stats.total_size_mb * 1024.0 * 1024.0) as u64;
    hash.aset(ruby.intern("total_size_bytes"), total_size_bytes)?;

    Ok(hash)
}

// Plugin system functions

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

    // Validate that processor is callable
    if !processor.respond_to("call", true)? {
        return Err(runtime_error("Post-processor must be a Proc or respond to 'call'"));
    }

    // Create Ruby-backed PostProcessor wrapper
    use async_trait::async_trait;
    use kreuzberg::plugins::{Plugin, PostProcessor, ProcessingStage};
    use std::sync::Arc;

    // SAFETY: We mark this as Send+Sync because Ruby Global VM Lock (GVL)
    // ensures thread safety. Magnus::Value is thread-safe under GVL.
    struct RubyPostProcessor {
        name: String,
        processor: magnus::Value,
    }

    // SAFETY: Ruby operations are protected by the Global VM Lock
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
            // Convert Rust result to Ruby hash
            let ruby = Ruby::get().expect("Ruby not initialized");
            let result_hash =
                extraction_result_to_ruby(&ruby, result.clone()).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                    message: format!("Failed to convert result to Ruby: {}", e),
                    plugin_name: self.name.clone(),
                })?;

            // Call Ruby Proc with result hash
            let modified = self
                .processor
                .funcall::<_, _, magnus::Value>("call", (result_hash,))
                .map_err(|e| kreuzberg::KreuzbergError::Plugin {
                    message: format!("Ruby post-processor failed: {}", e),
                    plugin_name: self.name.clone(),
                })?;

            // Convert modified hash back to Rust result
            let modified_hash =
                magnus::RHash::try_convert(modified).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                    message: format!("Post-processor must return a Hash: {}", e),
                    plugin_name: self.name.clone(),
                })?;

            // Update result content (only field that processors typically modify)
            if let Some(content_val) = get_kw(&ruby, modified_hash, "content") {
                let new_content = String::try_convert(content_val).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                    message: format!("Failed to convert content: {}", e),
                    plugin_name: self.name.clone(),
                })?;
                result.content = new_content;
            }

            Ok(())
        }

        fn processing_stage(&self) -> ProcessingStage {
            ProcessingStage::Late // Custom Ruby processors run last
        }
    }

    // Register with Rust core
    let processor_impl = Arc::new(RubyPostProcessor {
        name: name.clone(),
        processor,
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

    // Validate that validator is callable
    if !validator.respond_to("call", true)? {
        return Err(runtime_error("Validator must be a Proc or respond to 'call'"));
    }

    // Create Ruby-backed Validator wrapper
    use async_trait::async_trait;
    use kreuzberg::plugins::{Plugin, Validator};
    use std::sync::Arc;

    // SAFETY: We mark this as Send+Sync because Ruby Global VM Lock (GVL)
    // ensures thread safety. Magnus::Value is thread-safe under GVL.
    struct RubyValidator {
        name: String,
        validator: magnus::Value,
        priority: i32,
    }

    // SAFETY: Ruby operations are protected by the Global VM Lock
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
            // Convert Rust result to Ruby hash
            let ruby = Ruby::get().expect("Ruby not initialized");
            let result_hash =
                extraction_result_to_ruby(&ruby, result.clone()).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                    message: format!("Failed to convert result to Ruby: {}", e),
                    plugin_name: self.name.clone(),
                })?;

            // Call Ruby Proc with result hash
            self.validator
                .funcall::<_, _, magnus::Value>("call", (result_hash,))
                .map_err(|e| kreuzberg::KreuzbergError::Validation {
                    message: format!("Validation failed: {}", e),
                    source: None,
                })?;

            Ok(())
        }

        fn priority(&self) -> i32 {
            self.priority
        }
    }

    // Register with Rust core
    let validator_impl = Arc::new(RubyValidator {
        name: name.clone(),
        validator,
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
    // Validate that backend has required methods
    if !backend.respond_to("process_image", true)? {
        return Err(runtime_error("OCR backend must respond to 'process_image'"));
    }
    if !backend.respond_to("supports_language?", true)? {
        return Err(runtime_error("OCR backend must respond to 'supports_language?'"));
    }

    // Create Ruby-backed OcrBackend wrapper
    use async_trait::async_trait;
    use kreuzberg::plugins::{OcrBackend, OcrBackendType, Plugin};
    use std::sync::Arc;

    // SAFETY: We mark this as Send+Sync because Ruby Global VM Lock (GVL)
    // ensures thread safety. Magnus::Value is thread-safe under GVL.
    struct RubyOcrBackend {
        name: String,
        backend: magnus::Value,
    }

    // SAFETY: Ruby operations are protected by the Global VM Lock
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
            // Convert image bytes to Ruby string
            let ruby = Ruby::get().expect("Ruby not initialized");
            let image_str = ruby.str_from_slice(image_bytes);

            // Call Ruby backend's process_image method
            let text = self
                .backend
                .funcall::<_, _, String>("process_image", (image_str, config.language.clone()))
                .map_err(|e| kreuzberg::KreuzbergError::Ocr {
                    message: format!("Ruby OCR backend failed: {}", e),
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
            self.backend
                .funcall::<_, _, bool>("supports_language?", (lang,))
                .unwrap_or(false)
        }

        fn backend_type(&self) -> OcrBackendType {
            OcrBackendType::Custom
        }
    }

    // Register with Rust core
    let backend_impl = Arc::new(RubyOcrBackend {
        name: name.clone(),
        backend,
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

/// Initialize the Kreuzberg Ruby module
#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("Kreuzberg")?;

    // Synchronous extraction functions
    module.define_module_function("extract_file_sync", function!(extract_file_sync, -1))?;
    module.define_module_function("extract_bytes_sync", function!(extract_bytes_sync, -1))?;
    module.define_module_function("batch_extract_files_sync", function!(batch_extract_files_sync, -1))?;
    module.define_module_function("batch_extract_bytes_sync", function!(batch_extract_bytes_sync, -1))?;

    // Asynchronous extraction functions (use Tokio runtime internally)
    module.define_module_function("extract_file", function!(extract_file, -1))?;
    module.define_module_function("extract_bytes", function!(extract_bytes, -1))?;
    module.define_module_function("batch_extract_files", function!(batch_extract_files, -1))?;
    module.define_module_function("batch_extract_bytes", function!(batch_extract_bytes, -1))?;

    // Cache management functions
    module.define_module_function("clear_cache", function!(ruby_clear_cache, 0))?;
    module.define_module_function("cache_stats", function!(ruby_cache_stats, 0))?;

    // Plugin system functions
    module.define_module_function("register_post_processor", function!(register_post_processor, -1))?;
    module.define_module_function("register_validator", function!(register_validator, -1))?;
    module.define_module_function("register_ocr_backend", function!(register_ocr_backend, 2))?;
    module.define_module_function("unregister_post_processor", function!(unregister_post_processor, 1))?;
    module.define_module_function("unregister_validator", function!(unregister_validator, 1))?;
    module.define_module_function("clear_post_processors", function!(clear_post_processors, 0))?;
    module.define_module_function("clear_validators", function!(clear_validators, 0))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ruby_clear_cache_clears_directory() {
        use std::fs;
        use std::path::PathBuf;

        // Create unique test cache directory using thread ID
        let thread_id = std::thread::current().id();
        let cache_dir = PathBuf::from(format!("/tmp/kreuzberg_test_clear_{:?}", thread_id));

        // Clean up any existing test directory
        let _ = fs::remove_dir_all(&cache_dir);

        // Ensure directory exists
        fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");

        // Create a test cache file
        let test_file = cache_dir.join("test_cache.msgpack");
        fs::write(&test_file, b"test data").expect("Failed to write test file");

        // Verify file exists
        assert!(test_file.exists(), "Test file should exist before clear");

        // Clear cache using the function logic (without Ruby runtime)
        let cache_dir_str = cache_dir.to_str().expect("Cache dir must be valid UTF-8");
        let result = kreuzberg::cache::clear_cache_directory(cache_dir_str);

        assert!(result.is_ok(), "Cache clear should succeed");
        let (removed, _) = result.unwrap();
        assert_eq!(removed, 1, "Should remove one file");

        // Verify file is removed
        assert!(!test_file.exists(), "Test file should be removed after clear");

        // Clean up
        let _ = fs::remove_dir_all(&cache_dir);
    }

    #[test]
    fn test_ruby_cache_stats_returns_correct_structure() {
        use std::fs;
        use std::path::PathBuf;

        // Create unique test cache directory using thread ID
        let thread_id = std::thread::current().id();
        let cache_dir = PathBuf::from(format!("/tmp/kreuzberg_test_stats_{:?}", thread_id));

        // Clean up any existing test directory
        let _ = fs::remove_dir_all(&cache_dir);

        // Ensure directory exists
        fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");

        // Create test cache files
        let test_file1 = cache_dir.join("test1.msgpack");
        let test_file2 = cache_dir.join("test2.msgpack");
        fs::write(&test_file1, b"test data 1").expect("Failed to write test file 1");
        fs::write(&test_file2, b"test data 2").expect("Failed to write test file 2");

        // Get stats using the function logic (without Ruby runtime)
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

        // Clean up
        let _ = fs::remove_dir_all(&cache_dir);
    }

    #[test]
    fn test_ruby_cache_stats_converts_mb_to_bytes() {
        // Test the conversion logic
        let size_mb = 1.5; // 1.5 MB
        let size_bytes = (size_mb * 1024.0 * 1024.0) as u64;
        assert_eq!(size_bytes, 1_572_864, "Should convert MB to bytes correctly");
    }

    #[test]
    fn test_ruby_clear_cache_handles_empty_directory() {
        use std::fs;
        use std::path::PathBuf;

        // Create unique test cache directory using thread ID
        let thread_id = std::thread::current().id();
        let cache_dir = PathBuf::from(format!("/tmp/kreuzberg_test_empty_{:?}", thread_id));

        // Clean up any existing test directory
        let _ = fs::remove_dir_all(&cache_dir);

        // Ensure directory exists but is empty
        fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");

        // Clear empty cache
        let cache_dir_str = cache_dir.to_str().expect("Cache dir must be valid UTF-8");
        let result = kreuzberg::cache::clear_cache_directory(cache_dir_str);

        assert!(result.is_ok(), "Should handle empty directory");
        let (removed, freed) = result.unwrap();
        assert_eq!(removed, 0, "Should remove 0 files from empty directory");
        assert_eq!(freed, 0.0, "Should free 0 MB from empty directory");

        // Clean up
        let _ = fs::remove_dir_all(&cache_dir);
    }

    #[test]
    fn test_image_extraction_config_conversion() {
        // Test that ImageExtractionConfig struct has the expected fields
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
        // Test that ImagePreprocessingConfig struct has the expected fields
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
        // Test that PostProcessorConfig struct has the expected fields
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
        // Test that TokenReductionConfig struct has the expected fields
        let config = TokenReductionConfig {
            mode: "moderate".to_string(),
            preserve_important_words: true,
        };

        assert_eq!(config.mode, "moderate");
        assert!(config.preserve_important_words);
    }

    #[test]
    fn test_extraction_config_with_new_fields() {
        // Test that ExtractionConfig can hold the new configuration fields
        let mut config = ExtractionConfig::default();

        config.images = Some(ImageExtractionConfig {
            extract_images: true,
            target_dpi: 300,
            max_image_dimension: 4096,
            auto_adjust_dpi: true,
            min_dpi: 72,
            max_dpi: 600,
        });

        config.postprocessor = Some(PostProcessorConfig {
            enabled: true,
            enabled_processors: None,
            disabled_processors: None,
        });

        config.token_reduction = Some(TokenReductionConfig {
            mode: "light".to_string(),
            preserve_important_words: true,
        });

        assert!(config.images.is_some());
        assert!(config.postprocessor.is_some());
        assert!(config.token_reduction.is_some());
    }
}
