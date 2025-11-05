//! Kreuzberg Ruby Bindings (Magnus)
//!
//! High-performance document intelligence framework bindings for Ruby.
//! Provides extraction, OCR, chunking, and language detection for 30+ file formats.

use kreuzberg::{
    ChunkingConfig, ExtractionConfig, ExtractionResult as RustExtractionResult, KreuzbergError,
    LanguageDetectionConfig, OcrConfig, PdfConfig,
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

// Cache management functions are not yet implemented in the Rust API

/// Initialize the Kreuzberg Ruby module
#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("Kreuzberg")?;

    // Synchronous extraction functions
    module.define_module_function("extract_file_sync", function!(extract_file_sync, -1))?;
    module.define_module_function("extract_bytes_sync", function!(extract_bytes_sync, -1))?;
    module.define_module_function("batch_extract_files_sync", function!(batch_extract_files_sync, -1))?;

    // Asynchronous extraction functions (use Tokio runtime internally)
    module.define_module_function("extract_file", function!(extract_file, -1))?;
    module.define_module_function("extract_bytes", function!(extract_bytes, -1))?;
    module.define_module_function("batch_extract_files", function!(batch_extract_files, -1))?;

    Ok(())
}
