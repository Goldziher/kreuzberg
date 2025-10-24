#![deny(clippy::all)]

use kreuzberg::plugins::registry::{get_post_processor_registry, get_validator_registry};
use kreuzberg::{
    ChunkingConfig as RustChunkingConfig, ExtractionConfig, ExtractionResult as RustExtractionResult,
    ImageExtractionConfig as RustImageExtractionConfig, LanguageDetectionConfig as RustLanguageDetectionConfig,
    OcrConfig as RustOcrConfig, PdfConfig as RustPdfConfig, PostProcessorConfig as RustPostProcessorConfig,
    TesseractConfig as RustTesseractConfig, TokenReductionConfig as RustTokenReductionConfig,
};
use napi::bindgen_prelude::*;
use napi_derive::napi;

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
            keywords: None,
            postprocessor: val.postprocessor.map(Into::into),
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
#[derive(serde::Serialize, serde::Deserialize)]
pub struct JsExtractionResult {
    pub content: String,
    pub mime_type: String,
    #[napi(ts_type = "Metadata")]
    pub metadata: serde_json::Value,
    pub tables: Vec<JsTable>,
    pub detected_languages: Option<Vec<String>>,
    pub chunks: Option<Vec<String>>,
}

impl From<RustExtractionResult> for JsExtractionResult {
    fn from(val: RustExtractionResult) -> Self {
        let metadata = serde_json::to_value(&val.metadata).unwrap_or(serde_json::json!({}));

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

impl From<JsExtractionResult> for RustExtractionResult {
    fn from(val: JsExtractionResult) -> Self {
        let metadata = serde_json::from_value(val.metadata).unwrap_or_default();

        RustExtractionResult {
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
            chunks: val.chunks,
        }
    }
}

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

    let owned_data = data.to_vec();

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

    tokio::task::spawn_blocking(move || kreuzberg::batch_extract_file_sync(paths, &rust_config))
        .await
        .map_err(|e| Error::new(Status::GenericFailure, format!("Task join error: {}", e)))?
        .map(|results| results.into_iter().map(Into::into).collect())
        .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
}

/// Batch extract from multiple byte arrays (synchronous)
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

    let owned_data: Vec<Vec<u8>> = data_list.iter().map(|b| b.to_vec()).collect();

    tokio::task::spawn_blocking(move || {
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

// SAFETY: ThreadsafeFunction is explicitly designed to be called from any thread.
// It uses internal synchronization and the Node.js event loop to safely execute
// JavaScript. The Arc wrapper ensures the function lives long enough.
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
        let js_result: JsExtractionResult = result.clone().into();
        let json_input = serde_json::to_string(&js_result).map_err(|e| kreuzberg::KreuzbergError::Plugin {
            message: format!("Failed to serialize result for JavaScript PostProcessor: {}", e),
            plugin_name: self.name.clone(),
        })?;

        // Call JavaScript process function with JSON string and await result
        // Double await: first for the ThreadsafeFunction call, second for the Promise returned by JS
        let json_output = self
            .process_fn
            .call_async(json_input)
            .await
            .map_err(|e| kreuzberg::KreuzbergError::Plugin {
                message: format!("JavaScript PostProcessor '{}' failed: {}", self.name, e),
                plugin_name: self.name.clone(),
            })?
            .await
            .map_err(|e| kreuzberg::KreuzbergError::Plugin {
                message: format!("JavaScript PostProcessor '{}' failed: {}", self.name, e),
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
        let rust_result: kreuzberg::ExtractionResult = updated.into();
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
    let _ = registry.remove(&name);
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

// SAFETY: ThreadsafeFunction is explicitly designed to be called from any thread.
// It uses internal synchronization and the Node.js event loop to safely execute
// JavaScript. The Arc wrapper ensures the function lives long enough.
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
        let js_result: JsExtractionResult = result.clone().into();
        let json_input = serde_json::to_string(&js_result).map_err(|e| kreuzberg::KreuzbergError::Plugin {
            message: format!("Failed to serialize result for JavaScript Validator: {}", e),
            plugin_name: self.name.clone(),
        })?;

        // Call JavaScript validate function with JSON string and await result
        // Double await: first for the ThreadsafeFunction call, second for the Promise returned by JS
        let _empty_result = self
            .validate_fn
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
                        message: format!("JavaScript Validator '{}' failed: {}", self.name, err_msg),
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
                        message: format!("JavaScript Validator '{}' failed: {}", self.name, err_msg),
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
    let _ = registry.remove(&name);
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

// TODO: JavaScript OCR Backend bridge implementation
// Full callback bridge requires advanced NAPI-RS ThreadsafeFunction handling
// For now, we validate input and return descriptive error

/// Register a custom OCR backend
///
/// Registers a JavaScript OCR backend that can process images and extract text.
///
/// # Arguments
///
/// * `backend` - JavaScript object with the following interface:
///   - `name(): string` - Unique backend name
///   - `supportedLanguages(): string[]` - Array of supported ISO 639-2/3 language codes
///   - `processImage(imageBytes: Buffer, language: string): result` - Process image and return extraction result
///
/// # Example
///
/// ```typescript
/// import { registerOcrBackend } from '@kreuzberg/node';
///
/// registerOcrBackend({
///   name: () => "my-ocr",
///   supportedLanguages: () => ["eng", "deu", "fra"],
///   processImage: (imageBytes, language) => {
///     // Perform OCR on imageBytes
///     return {
///       content: "extracted text",
///       mime_type: "text/plain",
///       metadata: { confidence: 0.95 },
///       tables: [],
///       detected_languages: null,
///       chunks: null
///     };
///   }
/// });
/// ```
#[napi]
pub fn register_ocr_backend(_env: Env, backend: Object) -> Result<()> {
    // Validate backend has required methods
    backend.get_named_property::<Function>("name").map_err(|e| {
        Error::new(
            Status::InvalidArg,
            format!("OCR backend must have 'name' method: {}", e),
        )
    })?;

    backend
        .get_named_property::<Function>("supportedLanguages")
        .map_err(|e| {
            Error::new(
                Status::InvalidArg,
                format!("OCR backend must have 'supportedLanguages' method: {}", e),
            )
        })?;

    backend.get_named_property::<Function>("processImage").map_err(|e| {
        Error::new(
            Status::InvalidArg,
            format!("OCR backend must have 'processImage' method: {}", e),
        )
    })?;

    // JavaScript callback support not yet implemented
    Err(Error::new(
        Status::GenericFailure,
        "JavaScript OCR backend registration is not yet implemented. \
        This requires advanced NAPI-RS ThreadsafeFunction bridging to support \
        calling JavaScript functions from Rust async contexts. \
        Python OCR backends (EasyOCR, PaddleOCR) are fully supported via the Python bindings."
            .to_string(),
    ))
}

// #[cfg(all(
// #[global_allocator]
