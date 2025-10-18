#![deny(clippy::all)]

use kreuzberg::{
    ChunkingConfig as RustChunkingConfig, ExtractionConfig, ExtractionResult as RustExtractionResult,
    ImageExtractionConfig as RustImageExtractionConfig, LanguageDetectionConfig as RustLanguageDetectionConfig,
    OcrConfig as RustOcrConfig, PdfConfig as RustPdfConfig, PostProcessorConfig as RustPostProcessorConfig,
    TesseractConfig as RustTesseractConfig, TokenReductionConfig as RustTokenReductionConfig,
};
use napi::bindgen_prelude::*;
use napi_derive::napi;

// ============================================================================
// Configuration Types
// ============================================================================

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

#[napi(object)]
pub struct JsChunkingConfig {
    pub max_chars: Option<u32>,
    pub max_overlap: Option<u32>,
}

impl From<JsChunkingConfig> for RustChunkingConfig {
    fn from(val: JsChunkingConfig) -> Self {
        RustChunkingConfig {
            max_chars: val.max_chars.unwrap_or(1000) as usize,
            max_overlap: val.max_overlap.unwrap_or(200) as usize,
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
            keywords: None, // Keywords extraction config not exposed to TypeScript yet
            postprocessor: val.postprocessor.map(Into::into),
            max_concurrent_extractions: val.max_concurrent_extractions.map(|v| v as usize),
        }
    }
}

// ============================================================================
// Result Types
// ============================================================================

#[napi(object)]
pub struct JsTable {
    pub cells: Vec<Vec<String>>,
    pub markdown: String,
    pub page_number: u32,
}

#[napi(object)]
pub struct JsExtractionResult {
    pub content: String,
    pub mime_type: String,
    pub metadata: String,
    pub tables: Vec<JsTable>,
    pub detected_languages: Option<Vec<String>>,
    pub chunks: Option<Vec<String>>,
}

impl From<RustExtractionResult> for JsExtractionResult {
    fn from(val: RustExtractionResult) -> Self {
        // Serialize metadata to JSON string for JavaScript compatibility
        let metadata = serde_json::to_string(&val.metadata).unwrap_or_else(|_| "{}".to_string());

        JsExtractionResult {
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
            chunks: val.chunks,
        }
    }
}

// ============================================================================
// Extraction Functions
// ============================================================================

/// Extract content from a file (synchronous)
#[napi]
pub fn extract_file_sync(
    file_path: String,
    mime_type: Option<String>,
    config: Option<JsExtractionConfig>,
) -> Result<JsExtractionResult> {
    let rust_config = config.map(Into::into).unwrap_or_default();

    kreuzberg::extract_file_sync(&file_path, mime_type.as_deref(), &rust_config)
        .map(Into::into)
        .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
}

/// Extract content from a file (asynchronous)
#[napi]
pub async fn extract_file(
    file_path: String,
    mime_type: Option<String>,
    config: Option<JsExtractionConfig>,
) -> Result<JsExtractionResult> {
    let rust_config = config.map(Into::into).unwrap_or_default();

    // Use sync wrapper to avoid async runtime conflicts with NAPI-RS
    tokio::task::spawn_blocking(move || kreuzberg::extract_file_sync(&file_path, mime_type.as_deref(), &rust_config))
        .await
        .map_err(|e| Error::new(Status::GenericFailure, format!("Task join error: {}", e)))?
        .map(Into::into)
        .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
}

/// Extract content from bytes (synchronous)
#[napi]
pub fn extract_bytes_sync(
    data: Buffer,
    mime_type: String,
    config: Option<JsExtractionConfig>,
) -> Result<JsExtractionResult> {
    let rust_config = config.map(Into::into).unwrap_or_default();

    // Copy Buffer to owned Vec<u8> to avoid V8 GC lifetime issues
    let owned_data = data.to_vec();

    kreuzberg::extract_bytes_sync(&owned_data, &mime_type, &rust_config)
        .map(Into::into)
        .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
}

/// Extract content from bytes (asynchronous)
#[napi]
pub async fn extract_bytes(
    data: Buffer,
    mime_type: String,
    config: Option<JsExtractionConfig>,
) -> Result<JsExtractionResult> {
    let rust_config = config.map(Into::into).unwrap_or_default();

    // Copy Buffer to owned Vec<u8> to avoid V8 GC lifetime issues
    let owned_data = data.to_vec();

    // Use sync wrapper to avoid async runtime conflicts with NAPI-RS
    tokio::task::spawn_blocking(move || kreuzberg::extract_bytes_sync(&owned_data, &mime_type, &rust_config))
        .await
        .map_err(|e| Error::new(Status::GenericFailure, format!("Task join error: {}", e)))?
        .map(Into::into)
        .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
}

/// Batch extract from multiple files (synchronous)
#[napi]
pub fn batch_extract_files_sync(
    paths: Vec<String>,
    config: Option<JsExtractionConfig>,
) -> Result<Vec<JsExtractionResult>> {
    let rust_config = config.map(Into::into).unwrap_or_default();

    kreuzberg::batch_extract_file_sync(paths, &rust_config)
        .map(|results| results.into_iter().map(Into::into).collect())
        .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
}

/// Batch extract from multiple files (asynchronous)
#[napi]
pub async fn batch_extract_files(
    paths: Vec<String>,
    config: Option<JsExtractionConfig>,
) -> Result<Vec<JsExtractionResult>> {
    let rust_config = config.map(Into::into).unwrap_or_default();

    // Use sync wrapper to avoid async runtime conflicts with NAPI-RS
    tokio::task::spawn_blocking(move || kreuzberg::batch_extract_file_sync(paths, &rust_config))
        .await
        .map_err(|e| Error::new(Status::GenericFailure, format!("Task join error: {}", e)))?
        .map(|results| results.into_iter().map(Into::into).collect())
        .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
}

/// Batch extract from multiple byte arrays (asynchronous)
#[napi]
pub async fn batch_extract_bytes(
    data_list: Vec<Buffer>,
    mime_types: Vec<String>,
    config: Option<JsExtractionConfig>,
) -> Result<Vec<JsExtractionResult>> {
    let rust_config = config.map(Into::into).unwrap_or_default();

    // Copy all Buffers to owned Vec<u8> to avoid V8 GC lifetime issues
    let owned_data: Vec<Vec<u8>> = data_list.iter().map(|b| b.to_vec()).collect();

    // Use sync wrapper to avoid async runtime conflicts with NAPI-RS
    tokio::task::spawn_blocking(move || {
        // Create Vec<(&[u8], &str)> from owned data
        let contents: Vec<(&[u8], &str)> = owned_data
            .iter()
            .zip(mime_types.iter())
            .map(|(data, mime)| (data.as_slice(), mime.as_str()))
            .collect();

        kreuzberg::batch_extract_bytes_sync(contents, &rust_config)
    })
    .await
    .map_err(|e| Error::new(Status::GenericFailure, format!("Task join error: {}", e)))?
    .map(|results| results.into_iter().map(Into::into).collect())
    .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
}

// Mimalloc disabled - potential conflict with V8's allocator causing SIGILL crashes
// #[cfg(all(
//     any(windows, unix),
//     target_arch = "x86_64",
//     not(target_env = "musl"),
//     not(debug_assertions)
// ))]
// #[global_allocator]
// static ALLOC: mimalloc_rust::GlobalMiMalloc = mimalloc_rust::GlobalMiMalloc;
