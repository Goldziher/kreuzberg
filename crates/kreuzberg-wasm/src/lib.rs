use kreuzberg::{
    ChunkingConfig as RustChunkingConfig, ExtractionConfig, ExtractionResult as RustExtractionResult,
    ImageExtractionConfig as RustImageExtractionConfig, LanguageDetectionConfig as RustLanguageDetectionConfig,
    OcrConfig as RustOcrConfig, PdfConfig as RustPdfConfig, PostProcessorConfig as RustPostProcessorConfig,
    TesseractConfig as RustTesseractConfig, TokenReductionConfig as RustTokenReductionConfig,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Initialize panic hook for better error messages in the browser
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

// ============================================================================
// Configuration Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WasmOcrConfig {
    pub backend: String,
    pub language: Option<String>,
    pub tesseract_config: Option<WasmTesseractConfig>,
}

impl From<WasmOcrConfig> for RustOcrConfig {
    fn from(val: WasmOcrConfig) -> Self {
        RustOcrConfig {
            backend: val.backend,
            language: val.language.unwrap_or_else(|| "eng".to_string()),
            tesseract_config: val.tesseract_config.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WasmTesseractConfig {
    pub psm: Option<i32>,
    pub enable_table_detection: Option<bool>,
    pub tessedit_char_whitelist: Option<String>,
}

impl From<WasmTesseractConfig> for RustTesseractConfig {
    fn from(val: WasmTesseractConfig) -> Self {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WasmChunkingConfig {
    pub max_chars: Option<usize>,
    pub max_overlap: Option<usize>,
}

impl From<WasmChunkingConfig> for RustChunkingConfig {
    fn from(val: WasmChunkingConfig) -> Self {
        RustChunkingConfig {
            max_chars: val.max_chars.unwrap_or(1000),
            max_overlap: val.max_overlap.unwrap_or(200),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WasmLanguageDetectionConfig {
    pub enabled: Option<bool>,
    pub min_confidence: Option<f64>,
    pub detect_multiple: Option<bool>,
}

impl From<WasmLanguageDetectionConfig> for RustLanguageDetectionConfig {
    fn from(val: WasmLanguageDetectionConfig) -> Self {
        RustLanguageDetectionConfig {
            enabled: val.enabled.unwrap_or(true),
            min_confidence: val.min_confidence.unwrap_or(0.8),
            detect_multiple: val.detect_multiple.unwrap_or(false),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WasmTokenReductionConfig {
    pub mode: Option<String>,
    pub preserve_important_words: Option<bool>,
}

impl From<WasmTokenReductionConfig> for RustTokenReductionConfig {
    fn from(val: WasmTokenReductionConfig) -> Self {
        RustTokenReductionConfig {
            mode: val.mode.unwrap_or_else(|| "off".to_string()),
            preserve_important_words: val.preserve_important_words.unwrap_or(true),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WasmPdfConfig {
    pub extract_images: Option<bool>,
    pub passwords: Option<Vec<String>>,
    pub extract_metadata: Option<bool>,
}

impl From<WasmPdfConfig> for RustPdfConfig {
    fn from(val: WasmPdfConfig) -> Self {
        RustPdfConfig {
            extract_images: val.extract_images.unwrap_or(false),
            passwords: val.passwords,
            extract_metadata: val.extract_metadata.unwrap_or(true),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WasmImageExtractionConfig {
    pub extract_images: Option<bool>,
    pub target_dpi: Option<i32>,
    pub max_image_dimension: Option<i32>,
    pub auto_adjust_dpi: Option<bool>,
    pub min_dpi: Option<i32>,
    pub max_dpi: Option<i32>,
}

impl From<WasmImageExtractionConfig> for RustImageExtractionConfig {
    fn from(val: WasmImageExtractionConfig) -> Self {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WasmPostProcessorConfig {
    pub enabled: Option<bool>,
    pub enabled_processors: Option<Vec<String>>,
    pub disabled_processors: Option<Vec<String>>,
}

impl From<WasmPostProcessorConfig> for RustPostProcessorConfig {
    fn from(val: WasmPostProcessorConfig) -> Self {
        RustPostProcessorConfig {
            enabled: val.enabled.unwrap_or(true),
            enabled_processors: val.enabled_processors,
            disabled_processors: val.disabled_processors,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WasmExtractionConfig {
    pub use_cache: Option<bool>,
    pub enable_quality_processing: Option<bool>,
    pub ocr: Option<WasmOcrConfig>,
    pub force_ocr: Option<bool>,
    pub chunking: Option<WasmChunkingConfig>,
    pub images: Option<WasmImageExtractionConfig>,
    pub pdf_options: Option<WasmPdfConfig>,
    pub token_reduction: Option<WasmTokenReductionConfig>,
    pub language_detection: Option<WasmLanguageDetectionConfig>,
    pub postprocessor: Option<WasmPostProcessorConfig>,
    pub max_concurrent_extractions: Option<usize>,
}

impl From<WasmExtractionConfig> for ExtractionConfig {
    fn from(val: WasmExtractionConfig) -> Self {
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
            max_concurrent_extractions: val.max_concurrent_extractions,
        }
    }
}

// ============================================================================
// Result Types
// ============================================================================

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WasmTable {
    pub cells: Vec<Vec<String>>,
    pub markdown: String,
    pub page_number: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WasmExtractionResult {
    pub content: String,
    pub mime_type: String,
    pub metadata: serde_json::Value,
    pub tables: Vec<WasmTable>,
    pub detected_languages: Option<Vec<String>>,
    pub chunks: Option<Vec<String>>,
}

impl From<RustExtractionResult> for WasmExtractionResult {
    fn from(val: RustExtractionResult) -> Self {
        let metadata = serde_json::to_value(&val.metadata).unwrap_or_default();

        WasmExtractionResult {
            content: val.content,
            mime_type: val.mime_type,
            metadata,
            tables: val
                .tables
                .into_iter()
                .map(|t| WasmTable {
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

/// Extract content from bytes (asynchronous)
///
/// # Arguments
///
/// * `data` - File content as Uint8Array
/// * `mime_type` - MIME type of the data
/// * `options` - Optional extraction configuration (as a JavaScript object)
///
/// # Example
///
/// ```javascript
/// import { extractBytes } from 'kreuzberg-wasm';
///
/// const bytes = await fetch('document.pdf').then(r => r.arrayBuffer());
/// const result = await extractBytes(
///   new Uint8Array(bytes),
///   'application/pdf',
///   { ocr: { backend: 'tesseract', language: 'eng' } }
/// );
/// console.log(result.content);
/// ```
#[wasm_bindgen(js_name = extractBytes)]
pub async fn extract_bytes(data: Vec<u8>, mime_type: String, options: JsValue) -> Result<JsValue, JsValue> {
    let config: Option<WasmExtractionConfig> = if options.is_undefined() || options.is_null() {
        None
    } else {
        serde_wasm_bindgen::from_value(options)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse config: {}", e)))?
    };

    let rust_config = config.map(Into::into).unwrap_or_default();

    let result = kreuzberg::extract_bytes(&data, &mime_type, &rust_config)
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let wasm_result: WasmExtractionResult = result.into();
    serde_wasm_bindgen::to_value(&wasm_result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Batch extract from multiple byte arrays (asynchronous)
///
/// # Arguments
///
/// * `data_list` - Array of file contents as Uint8Arrays
/// * `mime_types` - Array of MIME types (one per data item)
/// * `options` - Optional extraction configuration
#[wasm_bindgen(js_name = batchExtractBytes)]
pub async fn batch_extract_bytes(
    data_list: Vec<JsValue>,
    mime_types: Vec<String>,
    options: JsValue,
) -> Result<JsValue, JsValue> {
    let config: Option<WasmExtractionConfig> = if options.is_undefined() || options.is_null() {
        None
    } else {
        serde_wasm_bindgen::from_value(options)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse config: {}", e)))?
    };

    let rust_config = config.map(Into::into).unwrap_or_default();

    // Convert JsValue array to Vec<Vec<u8>>
    let data: Vec<Vec<u8>> = data_list
        .iter()
        .map(|val| js_sys::Uint8Array::new(val).to_vec())
        .collect();

    // Create Vec<(&[u8], &str)> as required by the Rust API
    let contents: Vec<(&[u8], &str)> = data
        .iter()
        .zip(mime_types.iter())
        .map(|(d, m)| (d.as_slice(), m.as_str()))
        .collect();

    let results = kreuzberg::batch_extract_bytes(contents, &rust_config)
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let wasm_results: Vec<WasmExtractionResult> = results.into_iter().map(Into::into).collect();
    serde_wasm_bindgen::to_value(&wasm_results).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_config_conversion() {
        let config = WasmExtractionConfig {
            use_cache: Some(true),
            enable_quality_processing: Some(false),
            ocr: Some(WasmOcrConfig {
                backend: "tesseract".to_string(),
                language: Some("eng".to_string()),
                tesseract_config: None,
            }),
            force_ocr: Some(false),
            chunking: None,
            images: None,
            pdf_options: None,
            token_reduction: None,
            language_detection: None,
            postprocessor: None,
            max_concurrent_extractions: None,
        };

        let rust_config: ExtractionConfig = config.into();
        assert!(rust_config.ocr.is_some());
        assert_eq!(rust_config.ocr.unwrap().backend, "tesseract");
        assert!(rust_config.use_cache);
        assert!(!rust_config.enable_quality_processing);
    }

    /// Test sequential extractions (10 iterations) to verify no memory corruption
    /// This test mimics the NAPI-RS bug scenario where ~5 extractions cause SIGILL
    #[wasm_bindgen_test]
    async fn test_sequential_extractions_no_crash() {
        // Simple JSON test data
        let test_data = b"{\"test\": \"data\"}";
        let mime_type = "application/json";

        // Run 10 sequential extractions - NAPI crashes at ~5
        for i in 0..10 {
            let result = kreuzberg::extract_bytes(test_data, mime_type, &ExtractionConfig::default()).await;

            assert!(result.is_ok(), "Extraction {} failed: {:?}", i + 1, result.err());

            let extraction = result.unwrap();
            assert!(extraction.content.contains("test"));
            assert_eq!(extraction.mime_type, mime_type);
        }

        // If we get here without SIGILL, WASM doesn't have the bug!
    }

    /// Test with slightly larger data (markdown) - 10 sequential extractions
    #[wasm_bindgen_test]
    async fn test_sequential_markdown_extractions() {
        let test_data = b"# Test\n\nThis is a test document with some content.";
        let mime_type = "text/markdown";

        for i in 0..10 {
            let result = kreuzberg::extract_bytes(test_data, mime_type, &ExtractionConfig::default()).await;

            assert!(result.is_ok(), "Markdown extraction {} failed", i + 1);
            assert!(result.unwrap().content.len() > 0);
        }
    }
}
