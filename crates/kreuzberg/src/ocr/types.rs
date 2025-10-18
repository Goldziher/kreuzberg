use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Re-export ImagePreprocessingConfig from public types (same struct used internally)
pub use crate::types::ImagePreprocessingConfig;

/// Page Segmentation Mode for Tesseract OCR
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PSMMode {
    OsdOnly = 0,
    AutoOsd = 1,
    AutoOnly = 2,
    Auto = 3,
    SingleColumn = 4,
    SingleBlockVertical = 5,
    SingleBlock = 6,
    SingleLine = 7,
    SingleWord = 8,
    CircleWord = 9,
    SingleChar = 10,
}

impl PSMMode {
    pub fn from_u8(value: u8) -> Result<Self, String> {
        match value {
            0 => Ok(PSMMode::OsdOnly),
            1 => Ok(PSMMode::AutoOsd),
            2 => Ok(PSMMode::AutoOnly),
            3 => Ok(PSMMode::Auto),
            4 => Ok(PSMMode::SingleColumn),
            5 => Ok(PSMMode::SingleBlockVertical),
            6 => Ok(PSMMode::SingleBlock),
            7 => Ok(PSMMode::SingleLine),
            8 => Ok(PSMMode::SingleWord),
            9 => Ok(PSMMode::CircleWord),
            10 => Ok(PSMMode::SingleChar),
            _ => Err(format!("Invalid PSM mode value: {}", value)),
        }
    }

    pub fn as_u8(&self) -> u8 {
        *self as u8
    }
}

/// Configuration for Tesseract OCR (internal, efficient types).
///
/// This is the internal representation used by the OCR processor.
/// Public API uses i32 for PyO3 compatibility, converted to u8 here for efficiency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TesseractConfig {
    // Basic settings
    pub language: String,
    pub psm: u8,
    pub output_format: String,

    // NEW: OCR Engine Mode (0-3)
    pub oem: u8,

    // NEW: Minimum confidence threshold
    pub min_confidence: f64,

    // NEW: Image preprocessing
    pub preprocessing: Option<ImagePreprocessingConfig>,

    // Table detection
    pub enable_table_detection: bool,
    pub table_min_confidence: f64,
    pub table_column_threshold: u32,
    pub table_row_threshold_ratio: f64,

    // Tesseract engine tweaks
    pub use_cache: bool,
    pub classify_use_pre_adapted_templates: bool,
    pub language_model_ngram_on: bool,
    pub tessedit_dont_blkrej_good_wds: bool,
    pub tessedit_dont_rowrej_good_wds: bool,
    pub tessedit_enable_dict_correction: bool,
    pub tessedit_char_whitelist: String,
    pub tessedit_char_blacklist: String,
    pub tessedit_use_primary_params_model: bool,
    pub textord_space_size_is_variable: bool,
    pub thresholding_method: bool,
}

impl Default for TesseractConfig {
    fn default() -> Self {
        Self {
            language: "eng".to_string(),
            psm: 3,
            output_format: "markdown".to_string(),
            oem: 3, // Default (auto-select)
            min_confidence: 0.0,
            preprocessing: None,
            enable_table_detection: true,
            table_min_confidence: 0.0,
            table_column_threshold: 50,
            table_row_threshold_ratio: 0.5,
            use_cache: true,
            classify_use_pre_adapted_templates: true,
            language_model_ngram_on: false,
            tessedit_dont_blkrej_good_wds: true,
            tessedit_dont_rowrej_good_wds: true,
            tessedit_enable_dict_correction: true,
            tessedit_char_whitelist: String::new(),
            tessedit_char_blacklist: String::new(),
            tessedit_use_primary_params_model: true,
            textord_space_size_is_variable: true,
            thresholding_method: false,
        }
    }
}

impl TesseractConfig {
    pub fn validate(&self) -> Result<(), String> {
        match self.output_format.as_str() {
            "text" | "markdown" | "hocr" | "tsv" => Ok(()),
            _ => Err(format!(
                "Invalid output_format: '{}'. Must be one of: text, markdown, hocr, tsv",
                self.output_format
            )),
        }
    }
}

/// Convert from public API TesseractConfig to internal OCR TesseractConfig.
///
/// This conversion handles type differences (i32 → u8/u32) and clones
/// necessary fields. The public API uses i32 for PyO3 compatibility,
/// while the internal representation uses more efficient types.
impl From<&crate::types::TesseractConfig> for TesseractConfig {
    fn from(config: &crate::types::TesseractConfig) -> Self {
        Self {
            psm: config.psm as u8,
            language: config.language.clone(),
            output_format: config.output_format.clone(),
            oem: config.oem as u8,
            min_confidence: config.min_confidence,
            preprocessing: config.preprocessing.clone(),
            enable_table_detection: config.enable_table_detection,
            table_min_confidence: config.table_min_confidence,
            table_column_threshold: config.table_column_threshold as u32,
            table_row_threshold_ratio: config.table_row_threshold_ratio,
            use_cache: config.use_cache,
            classify_use_pre_adapted_templates: config.classify_use_pre_adapted_templates,
            language_model_ngram_on: config.language_model_ngram_on,
            tessedit_dont_blkrej_good_wds: config.tessedit_dont_blkrej_good_wds,
            tessedit_dont_rowrej_good_wds: config.tessedit_dont_rowrej_good_wds,
            tessedit_enable_dict_correction: config.tessedit_enable_dict_correction,
            tessedit_char_whitelist: config.tessedit_char_whitelist.clone(),
            tessedit_char_blacklist: config.tessedit_char_blacklist.clone(),
            tessedit_use_primary_params_model: config.tessedit_use_primary_params_model,
            textord_space_size_is_variable: config.textord_space_size_is_variable,
            thresholding_method: config.thresholding_method,
        }
    }
}

/// OCR extraction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub content: String,
    pub mime_type: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub tables: Vec<Table>,
}

/// Extracted table from OCR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub cells: Vec<Vec<String>>,
    pub markdown: String,
    pub page_number: i32,
}

/// Batch item result for processing multiple files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchItemResult {
    pub file_path: String,
    pub success: bool,
    pub result: Option<crate::types::OcrExtractionResult>,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_psm_mode_from_u8_valid() {
        let modes = [
            (0, PSMMode::OsdOnly),
            (1, PSMMode::AutoOsd),
            (2, PSMMode::AutoOnly),
            (3, PSMMode::Auto),
            (4, PSMMode::SingleColumn),
            (5, PSMMode::SingleBlockVertical),
            (6, PSMMode::SingleBlock),
            (7, PSMMode::SingleLine),
            (8, PSMMode::SingleWord),
            (9, PSMMode::CircleWord),
            (10, PSMMode::SingleChar),
        ];

        for (value, expected) in modes {
            let mode = PSMMode::from_u8(value).unwrap();
            assert_eq!(mode, expected);
        }
    }

    #[test]
    fn test_psm_mode_from_u8_invalid() {
        let invalid_values = [11, 12, 255, 100];

        for value in invalid_values {
            let result = PSMMode::from_u8(value);
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("Invalid PSM mode"));
        }
    }

    #[test]
    fn test_psm_mode_as_u8() {
        assert_eq!(PSMMode::Auto.as_u8(), 3);
        assert_eq!(PSMMode::SingleLine.as_u8(), 7);
        assert_eq!(PSMMode::SingleChar.as_u8(), 10);
    }

    #[test]
    fn test_tesseract_config_default() {
        let config = TesseractConfig::default();

        assert_eq!(config.language, "eng");
        assert_eq!(config.psm, 3);
        assert_eq!(config.output_format, "markdown");
        assert!(config.enable_table_detection);
        assert_eq!(config.table_min_confidence, 0.0);
        assert_eq!(config.table_column_threshold, 50);
        assert_eq!(config.table_row_threshold_ratio, 0.5);
        assert!(config.use_cache);
    }

    #[test]
    fn test_tesseract_config_validate_valid() {
        let valid_formats = ["text", "markdown", "hocr", "tsv"];

        for format in valid_formats {
            let config = TesseractConfig {
                output_format: format.to_string(),
                ..Default::default()
            };
            assert!(config.validate().is_ok());
        }
    }

    #[test]
    fn test_tesseract_config_validate_invalid() {
        let config = TesseractConfig {
            output_format: "invalid".to_string(),
            ..Default::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid output_format"));
    }

    #[test]
    fn test_extraction_result_creation() {
        let mut metadata = HashMap::new();
        metadata.insert("key".to_string(), serde_json::json!("value"));

        let table = Table {
            cells: vec![vec!["A".to_string(), "B".to_string()]],
            markdown: "| A | B |".to_string(),
            page_number: 0,
        };

        let result = ExtractionResult {
            content: "Test content".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: metadata.clone(),
            tables: vec![table],
        };

        assert_eq!(result.content, "Test content");
        assert_eq!(result.mime_type, "text/plain");
        assert_eq!(result.metadata.get("key").unwrap(), &serde_json::json!("value"));
        assert_eq!(result.tables.len(), 1);
    }

    #[test]
    fn test_table_creation() {
        let cells = vec![
            vec!["Header1".to_string(), "Header2".to_string()],
            vec!["Value1".to_string(), "Value2".to_string()],
        ];

        let markdown = "| Header1 | Header2 |\n| ------- | ------- |\n| Value1  | Value2  |".to_string();

        let table = Table {
            cells: cells.clone(),
            markdown: markdown.clone(),
            page_number: 1,
        };

        assert_eq!(table.cells.len(), 2);
        assert_eq!(table.cells[0].len(), 2);
        assert_eq!(table.markdown, markdown);
        assert_eq!(table.page_number, 1);
    }

    #[test]
    fn test_batch_item_result_success() {
        let result = crate::types::OcrExtractionResult {
            content: "content".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: vec![],
        };

        let batch_result = BatchItemResult {
            file_path: "/path/to/file.png".to_string(),
            success: true,
            result: Some(result),
            error: None,
        };

        assert_eq!(batch_result.file_path, "/path/to/file.png");
        assert!(batch_result.success);
        assert!(batch_result.result.is_some());
        assert!(batch_result.error.is_none());
    }

    #[test]
    fn test_batch_item_result_failure() {
        let batch_result = BatchItemResult {
            file_path: "/path/to/file.png".to_string(),
            success: false,
            result: None,
            error: Some("File not found".to_string()),
        };

        assert_eq!(batch_result.file_path, "/path/to/file.png");
        assert!(!batch_result.success);
        assert!(batch_result.result.is_none());
        assert_eq!(batch_result.error.as_ref().unwrap(), "File not found");
    }

    #[test]
    fn test_tesseract_config_from_public_api() {
        let public_config = crate::types::TesseractConfig {
            language: "deu".to_string(),
            psm: 6, // i32 in public API
            output_format: "text".to_string(),
            oem: 1, // i32 in public API
            min_confidence: 70.0,
            preprocessing: Some(crate::types::ImagePreprocessingConfig::default()),
            enable_table_detection: false,
            table_min_confidence: 50.0,
            table_column_threshold: 100, // i32 in public API
            table_row_threshold_ratio: 0.8,
            use_cache: false,
            classify_use_pre_adapted_templates: false,
            language_model_ngram_on: true,
            tessedit_dont_blkrej_good_wds: false,
            tessedit_dont_rowrej_good_wds: false,
            tessedit_enable_dict_correction: false,
            tessedit_char_whitelist: "0123456789".to_string(),
            tessedit_char_blacklist: "!@#$".to_string(),
            tessedit_use_primary_params_model: false,
            textord_space_size_is_variable: false,
            thresholding_method: true,
        };

        let internal_config: TesseractConfig = (&public_config).into();

        // Verify all fields converted correctly
        assert_eq!(internal_config.language, "deu");
        assert_eq!(internal_config.psm, 6); // Converted to u8
        assert_eq!(internal_config.output_format, "text");
        assert_eq!(internal_config.oem, 1); // Converted to u8
        assert_eq!(internal_config.min_confidence, 70.0);
        assert!(internal_config.preprocessing.is_some());
        assert!(!internal_config.enable_table_detection);
        assert_eq!(internal_config.table_min_confidence, 50.0);
        assert_eq!(internal_config.table_column_threshold, 100); // Converted to u32
        assert_eq!(internal_config.table_row_threshold_ratio, 0.8);
        assert!(!internal_config.use_cache);
        assert!(!internal_config.classify_use_pre_adapted_templates);
        assert!(internal_config.language_model_ngram_on);
        assert!(!internal_config.tessedit_dont_blkrej_good_wds);
        assert!(!internal_config.tessedit_dont_rowrej_good_wds);
        assert!(!internal_config.tessedit_enable_dict_correction);
        assert_eq!(internal_config.tessedit_char_whitelist, "0123456789");
        assert_eq!(internal_config.tessedit_char_blacklist, "!@#$");
        assert!(!internal_config.tessedit_use_primary_params_model);
        assert!(!internal_config.textord_space_size_is_variable);
        assert!(internal_config.thresholding_method);
    }
}
