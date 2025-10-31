#![deny(clippy::all)]

use kreuzberg::plugins::registry::{get_post_processor_registry, get_validator_registry};
use kreuzberg::{
    ChunkingConfig as RustChunkingConfig, EmbeddingConfig as RustEmbeddingConfig,
    EmbeddingModelType as RustEmbeddingModelType, ExtractionConfig, ExtractionResult as RustExtractionResult,
    ImageExtractionConfig as RustImageExtractionConfig, LanguageDetectionConfig as RustLanguageDetectionConfig,
    OcrConfig as RustOcrConfig, PdfConfig as RustPdfConfig, PostProcessorConfig as RustPostProcessorConfig,
    TesseractConfig as RustTesseractConfig, TokenReductionConfig as RustTokenReductionConfig,
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
        // IO errors - system-level file access issues
        KreuzbergError::Io(e) => Error::new(Status::GenericFailure, format!("IO error: {}", e)),

        // Parsing errors - invalid document format, corrupt files
        KreuzbergError::Parsing { message, .. } => {
            Error::new(Status::InvalidArg, format!("Parsing error: {}", message))
        }

        // OCR errors - OCR processing failures
        KreuzbergError::Ocr { message, .. } => Error::new(Status::GenericFailure, format!("OCR error: {}", message)),

        // Validation errors - invalid configuration or parameters
        KreuzbergError::Validation { message, .. } => {
            Error::new(Status::InvalidArg, format!("Validation error: {}", message))
        }

        // Cache errors - non-fatal cache operations
        KreuzbergError::Cache { message, .. } => {
            Error::new(Status::GenericFailure, format!("Cache error: {}", message))
        }

        // Image processing errors
        KreuzbergError::ImageProcessing { message, .. } => {
            Error::new(Status::GenericFailure, format!("Image processing error: {}", message))
        }

        // Serialization errors - JSON/MessagePack failures
        KreuzbergError::Serialization { message, .. } => {
            Error::new(Status::InvalidArg, format!("Serialization error: {}", message))
        }

        // Missing dependency errors - missing system tools (tesseract, pandoc, etc.)
        KreuzbergError::MissingDependency(dep) => {
            Error::new(Status::GenericFailure, format!("Missing dependency: {}", dep))
        }

        // Plugin errors - plugin-specific failures
        KreuzbergError::Plugin { message, plugin_name } => Error::new(
            Status::GenericFailure,
            format!("Plugin error in '{}': {}", plugin_name, message),
        ),

        // Lock poisoned errors - should not happen in normal operation
        KreuzbergError::LockPoisoned(msg) => Error::new(Status::GenericFailure, format!("Lock poisoned: {}", msg)),

        // Unsupported format errors - invalid or unsupported MIME types
        KreuzbergError::UnsupportedFormat(format) => {
            Error::new(Status::InvalidArg, format!("Unsupported format: {}", format))
        }

        // Other errors - catch-all
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
    pub max_concurrent_extractions: Option<u32>,
}

impl From<JsExtractionConfig> for ExtractionConfig {
    fn from(val: JsExtractionConfig) -> Self {
        ExtractionConfig {
            use_cache: val.use_cache.unwrap_or(true),
            enable_quality_processing: val.enable_quality_processing.unwrap_or(true),
            ocr: val.ocr.map(Into::into),
            force_ocr: val.force_ocr.unwrap_or(false),
            chunking: val.chunking.map(Into::into),
            images: val.images.map(Into::into),
            pdf_options: val.pdf_options.map(Into::into),
            token_reduction: val.token_reduction.map(Into::into),
            language_detection: val.language_detection.map(Into::into),
            keywords: None,
            postprocessor: val.postprocessor.map(Into::into),
            html_options: None,
            max_concurrent_extractions: val.max_concurrent_extractions.map(|v| v as usize),
        }
    }
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
#[derive(serde::Serialize, serde::Deserialize)]
pub struct JsExtractionResult {
    pub content: String,
    pub mime_type: String,
    #[napi(ts_type = "Metadata")]
    pub metadata: serde_json::Value,
    pub tables: Vec<JsTable>,
    pub detected_languages: Option<Vec<String>>,
    pub chunks: Option<Vec<String>>,
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
            chunks: val
                .chunks
                .map(|chunks| chunks.into_iter().map(|chunk| chunk.content).collect()),
            images,
        })
    }
}

impl TryFrom<JsExtractionResult> for RustExtractionResult {
    type Error = napi::Error;

    fn try_from(val: JsExtractionResult) -> Result<Self> {
        let metadata = serde_json::from_value(val.metadata)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to deserialize metadata: {}", e)))?;

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
                let total_chunks = chunks.len();
                chunks
                    .into_iter()
                    .enumerate()
                    .map(|(index, content)| kreuzberg::Chunk {
                        content: content.clone(),
                        embedding: None,
                        metadata: kreuzberg::ChunkMetadata {
                            char_start: 0,
                            char_end: content.len(),
                            token_count: None,
                            chunk_index: index,
                            total_chunks,
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
    let rust_config = config.map(Into::into).unwrap_or_default();

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
    let rust_config = config.map(Into::into).unwrap_or_default();

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
    let rust_config = config.map(Into::into).unwrap_or_default();

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
    let rust_config: kreuzberg::ExtractionConfig = config.map(Into::into).unwrap_or_default();
    let owned_data = data.to_vec();

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
    let rust_config = config.map(Into::into).unwrap_or_default();

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
    let rust_config = config.map(Into::into).unwrap_or_default();

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
    let rust_config = config.map(Into::into).unwrap_or_default();

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
    let rust_config = config.map(Into::into).unwrap_or_default();

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

// JavaScript PostProcessor bridge implementation using ThreadsafeFunction

use async_trait::async_trait;
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

// SAFETY: ThreadsafeFunction from napi-rs is designed to be Send + Sync.
// - ThreadsafeFunction uses internal synchronization primitives to safely call JavaScript from any thread
// - NAPI-RS guarantees thread-safe execution of callbacks by marshaling all JavaScript interactions
//   through the Node.js event loop via a callback queue
// - The JavaScript function reference is managed by the Node.js runtime and protected by NAPI-RS
// - No raw JavaScript values (napi_value) are stored directly; only the ThreadsafeFunction wrapper
// - The Arc<ThreadsafeFunction<...>> wrapper provides shared ownership with atomic reference counting
// - All cross-thread access goes through ThreadsafeFunction::call_async which handles synchronization
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
        // Convert Rust ExtractionResult to JS format and serialize to JSON
        let js_result =
            JsExtractionResult::try_from(result.clone()).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                message: format!("Failed to convert result for JavaScript PostProcessor: {}", e),
                plugin_name: self.name.clone(),
            })?;
        let json_input = serde_json::to_string(&js_result).map_err(|e| kreuzberg::KreuzbergError::Plugin {
            message: format!("Failed to serialize result for JavaScript PostProcessor: {}", e),
            plugin_name: self.name.clone(),
        })?;

        // Call JavaScript process function with JSON string
        // We're already in an async context, so we can directly await the ThreadsafeFunction
        // No spawn_blocking needed - this allows the Node.js event loop to process the callback naturally
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

        // Deserialize the JSON result
        let updated: JsExtractionResult =
            serde_json::from_str(&json_output).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                message: format!(
                    "Failed to deserialize result from JavaScript PostProcessor '{}': {}",
                    self.name, e
                ),
                plugin_name: self.name.clone(),
            })?;

        // Update the result in-place by converting from JS format
        let rust_result =
            kreuzberg::ExtractionResult::try_from(updated).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                message: format!("Failed to convert result from JavaScript PostProcessor: {}", e),
                plugin_name: self.name.clone(),
            })?;
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
    // Validate required methods exist
    validate_plugin_object(&processor, "PostProcessor", &["name", "process"])?;

    // Get processor name
    let name_fn: Function<(), String> = processor.get_named_property("name")?;
    let name: String = name_fn.call(())?;

    if name.is_empty() {
        return Err(Error::new(
            Status::InvalidArg,
            "Processor name cannot be empty".to_string(),
        ));
    }

    // Get processing stage (optional, defaults to Middle)
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

    // Get process function and create ThreadsafeFunction
    // The JS function is async and returns Promise<String>
    let process_fn: Function<String, Promise<String>> = processor.get_named_property("process")?;

    // Build ThreadsafeFunction with callback to properly pass arguments to JS
    // The JS function is async and returns a Promise, so we use build_callback
    // to transform the argument passing (wrapping in an array)
    let tsfn = process_fn.build_threadsafe_function().build_callback(|ctx| {
        // Return the value wrapped in a vec so JS receives it as ...args
        Ok(vec![ctx.value])
    })?;

    // Create the Rust wrapper
    let js_processor = JsPostProcessor {
        name: name.clone(),
        process_fn: Arc::new(tsfn),
        stage,
    };

    // Register with the Rust registry
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

    // Remove processor from the internal HashMap
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

    // Clear all processors from the internal HashMap
    *registry = Default::default();
    Ok(())
}

// JavaScript Validator bridge implementation using ThreadsafeFunction

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

// SAFETY: ThreadsafeFunction from napi-rs is designed to be Send + Sync.
// - ThreadsafeFunction uses internal synchronization primitives to safely call JavaScript from any thread
// - NAPI-RS guarantees thread-safe execution of callbacks by marshaling all JavaScript interactions
//   through the Node.js event loop via a callback queue
// - The JavaScript function reference is managed by the Node.js runtime and protected by NAPI-RS
// - No raw JavaScript values (napi_value) are stored directly; only the ThreadsafeFunction wrapper
// - The Arc<ThreadsafeFunction<...>> wrapper provides shared ownership with atomic reference counting
// - All cross-thread access goes through ThreadsafeFunction::call_async which handles synchronization
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
        // Convert Rust ExtractionResult to JS format and serialize to JSON
        let js_result =
            JsExtractionResult::try_from(result.clone()).map_err(|e| kreuzberg::KreuzbergError::Plugin {
                message: format!("Failed to convert result for JavaScript Validator: {}", e),
                plugin_name: self.name.clone(),
            })?;
        let json_input = serde_json::to_string(&js_result).map_err(|e| kreuzberg::KreuzbergError::Plugin {
            message: format!("Failed to serialize result for JavaScript Validator: {}", e),
            plugin_name: self.name.clone(),
        })?;

        // Call JavaScript validate function with JSON string
        // We're already in an async context, so we can directly await the ThreadsafeFunction
        // No spawn_blocking needed - this allows the Node.js event loop to process the callback naturally
        self.validate_fn
            .call_async(json_input)
            .await
            .map_err(|e| {
                let err_msg = e.to_string();
                // Check if the error message contains "ValidationError" to determine error type
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
                // Check if the error message contains "ValidationError" to determine error type
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

        // Validation passed (empty string returned)
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
    // Validate required methods exist
    validate_plugin_object(&validator, "Validator", &["name", "validate"])?;

    // Get validator name
    let name_fn: Function<(), String> = validator.get_named_property("name")?;
    let name: String = name_fn.call(())?;

    if name.is_empty() {
        return Err(Error::new(
            Status::InvalidArg,
            "Validator name cannot be empty".to_string(),
        ));
    }

    // Get priority (optional, defaults to 50)
    let priority = if let Ok(priority_fn) = validator.get_named_property::<Function<(), i32>>("priority") {
        priority_fn.call(())?
    } else {
        50
    };

    // Get validate function and create ThreadsafeFunction
    // The JS function is async and returns Promise<String>
    let validate_fn: Function<String, Promise<String>> = validator.get_named_property("validate")?;

    // Build ThreadsafeFunction with callback to properly pass arguments to JS
    // The JS function is async and returns a Promise, so we use build_callback
    // to transform the argument passing (wrapping in an array)
    let tsfn = validate_fn.build_threadsafe_function().build_callback(|ctx| {
        // Return the value wrapped in a vec so JS receives it as ...args
        Ok(vec![ctx.value])
    })?;

    // Create the Rust wrapper
    let js_validator = JsValidator {
        name: name.clone(),
        validate_fn: Arc::new(tsfn),
        priority,
    };

    // Register with the Rust registry
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

    // Remove validator from the internal HashMap
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

    // Clear all validators from the internal HashMap
    *registry = Default::default();
    Ok(())
}

// JavaScript OCR Backend bridge implementation using ThreadsafeFunction

use kreuzberg::plugins::registry::get_ocr_backend_registry;
use kreuzberg::plugins::{OcrBackend as RustOcrBackend, OcrBackendType};

/// Wrapper that makes a JavaScript OCR backend usable from Rust.
///
/// Uses JSON serialization to pass data between Rust and JavaScript due to NAPI limitations
/// with complex object types across ThreadsafeFunction boundaries.
///
/// Wrapper that holds the ThreadsafeFunction to call JavaScript from Rust.
/// The processImage_fn is an async JavaScript function that:
/// - Takes: (Buffer, String) - image bytes and language code
/// - Returns: Promise<String> (JSON-serialized ExtractionResult)
///
/// Type parameters:
/// - Input: (Buffer, String)
/// - Return: Promise<String>
/// - CallJsBackArgs: Vec<(Buffer, String)> (because build_callback returns vec![value])
/// - ErrorStatus: napi::Status
/// - CalleeHandled: false (default with build_callback)
type ProcessImageFn =
    Arc<ThreadsafeFunction<(Buffer, String), Promise<String>, Vec<(Buffer, String)>, napi::Status, false>>;

struct JsOcrBackend {
    name: String,
    supported_languages: Vec<String>,
    process_image_fn: ProcessImageFn,
}

// SAFETY: ThreadsafeFunction from napi-rs is designed to be Send + Sync.
// - ThreadsafeFunction uses internal synchronization primitives to safely call JavaScript from any thread
// - NAPI-RS guarantees thread-safe execution of callbacks by marshaling all JavaScript interactions
//   through the Node.js event loop via a callback queue
// - The JavaScript function reference is managed by the Node.js runtime and protected by NAPI-RS
// - No raw JavaScript values (napi_value) are stored directly; only the ThreadsafeFunction wrapper
// - The Arc<ThreadsafeFunction<...>> wrapper provides shared ownership with atomic reference counting
// - All cross-thread access goes through ThreadsafeFunction::call_async which handles synchronization
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
        // Convert image bytes to Buffer and language to String
        let buffer = Buffer::from(image_bytes);
        let language = config.language.clone();
        let backend_name = self.name.clone();

        // Call JavaScript processImage function with Buffer and language string
        // Double await: first for the ThreadsafeFunction call, second for the Promise returned by JS
        let json_output = self
            .process_image_fn
            .call_async((buffer, language))
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

        // Deserialize the JSON result
        let wire_result: serde_json::Value =
            serde_json::from_str(&json_output).map_err(|e| kreuzberg::KreuzbergError::Ocr {
                message: format!(
                    "Failed to deserialize result from JavaScript OCR backend '{}': {}",
                    backend_name, e
                ),
                source: Some(Box::new(e)),
            })?;

        // Convert wire format to ExtractionResult
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
///   - `processImage(imageBytes: Buffer, language: string): Promise<result>` - Process image and return extraction result
///
/// # Implementation Notes
///
/// Due to NAPI ThreadsafeFunction limitations, the processImage function receives:
/// - `imageBytes` as Buffer (first argument)
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
///     // Perform OCR on imageBytes
///     const text = await myOcrLibrary.process(imageBytes, language);
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
    // Validate required methods exist
    validate_plugin_object(&backend, "OCR Backend", &["name", "supportedLanguages", "processImage"])?;

    // Get backend name
    let name_fn: Function<(), String> = backend.get_named_property("name")?;
    let name: String = name_fn.call(())?;

    if name.is_empty() {
        return Err(Error::new(
            Status::InvalidArg,
            "OCR backend name cannot be empty".to_string(),
        ));
    }

    // Get supported languages
    let supported_languages_fn: Function<(), Vec<String>> = backend.get_named_property("supportedLanguages")?;
    let supported_languages: Vec<String> = supported_languages_fn.call(())?;

    if supported_languages.is_empty() {
        return Err(Error::new(
            Status::InvalidArg,
            "OCR backend must support at least one language".to_string(),
        ));
    }

    // Get processImage function and create ThreadsafeFunction
    // The JS function is async and returns Promise<String>
    let process_image_fn: Function<(Buffer, String), Promise<String>> = backend.get_named_property("processImage")?;

    // Build ThreadsafeFunction with callback to properly pass arguments to JS
    // The JS function is async and returns a Promise, so we use build_callback
    // to transform the argument passing (wrapping in a vec)
    let tsfn = process_image_fn.build_threadsafe_function().build_callback(|ctx| {
        // Return the value wrapped in a vec so JS receives it as separate arguments
        Ok(vec![ctx.value])
    })?;

    // Create the Rust wrapper
    let js_ocr_backend = JsOcrBackend {
        name: name.clone(),
        supported_languages,
        process_image_fn: Arc::new(tsfn),
    };

    // Register with the Rust registry
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

// #[cfg(all(
// #[global_allocator]
