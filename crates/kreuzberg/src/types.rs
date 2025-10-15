use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// General extraction result used by the core extraction API.
///
/// This is the main result type returned by all extraction functions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub content: String,
    pub mime_type: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub tables: Vec<Table>,
}

/// Extracted table structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub cells: Vec<Vec<String>>,
    pub markdown: String,
    pub page_number: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelWorkbook {
    pub sheets: Vec<ExcelSheet>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelSheet {
    pub name: String,
    pub markdown: String,
    pub row_count: usize,
    pub col_count: usize,
    pub cell_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XmlExtractionResult {
    pub content: String,
    pub element_count: usize,
    pub unique_elements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextExtractionResult {
    pub content: String,
    pub line_count: usize,
    pub word_count: usize,
    pub character_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Vec<(String, String)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_blocks: Option<Vec<(String, String)>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PptxExtractionResult {
    pub content: String,
    pub metadata: PptxMetadata,
    pub slide_count: usize,
    pub image_count: usize,
    pub table_count: usize,
    pub images: Vec<ExtractedImage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PptxMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub summary: Option<String>,
    pub fonts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedImage {
    pub data: Vec<u8>,
    pub format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slide_number: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailExtractionResult {
    pub subject: Option<String>,
    pub from_email: Option<String>,
    pub to_emails: Vec<String>,
    pub cc_emails: Vec<String>,
    pub bcc_emails: Vec<String>,
    pub date: Option<String>,
    pub message_id: Option<String>,
    pub plain_text: Option<String>,
    pub html_content: Option<String>,
    pub cleaned_text: String,
    pub attachments: Vec<EmailAttachment>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAttachment {
    pub name: Option<String>,
    pub filename: Option<String>,
    pub mime_type: Option<String>,
    pub size: Option<usize>,
    pub is_image: bool,
    pub data: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrExtractionResult {
    pub content: String,
    pub mime_type: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub tables: Vec<OcrTable>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrTable {
    pub cells: Vec<Vec<String>>,
    pub markdown: String,
    pub page_number: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TesseractConfig {
    pub language: String,
    pub psm: i32,
    pub output_format: String,
    pub enable_table_detection: bool,
    pub table_min_confidence: f64,
    pub table_column_threshold: i32,
    pub table_row_threshold_ratio: f64,
    pub use_cache: bool,
    pub classify_use_pre_adapted_templates: bool,
    pub language_model_ngram_on: bool,
    pub tessedit_dont_blkrej_good_wds: bool,
    pub tessedit_dont_rowrej_good_wds: bool,
    pub tessedit_enable_dict_correction: bool,
    pub tessedit_char_whitelist: String,
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
            tessedit_use_primary_params_model: true,
            textord_space_size_is_variable: true,
            thresholding_method: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImagePreprocessingMetadata {
    pub original_dimensions: (usize, usize),
    pub original_dpi: (f64, f64),
    pub target_dpi: i32,
    pub scale_factor: f64,
    pub auto_adjusted: bool,
    pub final_dpi: i32,
    pub new_dimensions: Option<(usize, usize)>,
    pub resample_method: String,
    pub dimension_clamped: bool,
    pub calculated_dpi: Option<i32>,
    pub skipped_resize: bool,
    pub resize_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionConfig {
    pub target_dpi: i32,
    pub max_image_dimension: i32,
    pub auto_adjust_dpi: bool,
    pub min_dpi: i32,
    pub max_dpi: i32,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            target_dpi: 300,
            max_image_dimension: 4096,
            auto_adjust_dpi: true,
            min_dpi: 72,
            max_dpi: 600,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_files: usize,
    pub total_size_mb: f64,
    pub available_space_mb: f64,
    pub oldest_file_age_days: f64,
    pub newest_file_age_days: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PandocExtractionResult {
    pub content: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibreOfficeConversionResult {
    pub converted_bytes: Vec<u8>,
    pub original_format: String,
    pub target_format: String,
}
