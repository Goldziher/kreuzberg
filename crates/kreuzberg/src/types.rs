use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "pdf")]
use crate::pdf::metadata::PdfMetadata;

// ============================================================================
// Extraction Result
// ============================================================================

/// General extraction result used by the core extraction API.
///
/// This is the main result type returned by all extraction functions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub content: String,
    pub mime_type: String,
    pub metadata: Metadata,
    pub tables: Vec<Table>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detected_languages: Option<Vec<String>>,
}

// ============================================================================
// Metadata Types
// ============================================================================

/// Strongly-typed metadata for extraction results.
///
/// This struct provides compile-time type safety for metadata fields
/// while remaining flexible through the `additional` HashMap for
/// custom fields (e.g., from Python postprocessors).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Metadata {
    // Common fields (used across multiple formats)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    // Format-specific metadata
    #[cfg(feature = "pdf")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf: Option<PdfMetadata>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub excel: Option<ExcelMetadata>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<EmailMetadata>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pptx: Option<PptxMetadata>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive: Option<ArchiveMetadata>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<ImageMetadata>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub xml: Option<XmlMetadata>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<TextMetadata>,

    // Processing metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ocr: Option<OcrMetadata>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_preprocessing: Option<ImagePreprocessingMetadata>,

    // Structured data metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_schema: Option<serde_json::Value>,

    // Error metadata (for batch operations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorMetadata>,

    /// Additional custom fields.
    ///
    /// This flattened HashMap allows Python postprocessors (entity extraction,
    /// keyword extraction, etc.) to add arbitrary fields. Fields in this map
    /// are merged at the root level during serialization.
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

/// Excel/spreadsheet metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelMetadata {
    pub sheet_count: usize,
    pub sheet_names: Vec<String>,
}

/// Email metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_email: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_name: Option<String>,

    pub to_emails: Vec<String>,
    pub cc_emails: Vec<String>,
    pub bcc_emails: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,

    pub attachments: Vec<String>,
}

/// Archive (ZIP/TAR) metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveMetadata {
    pub format: String,
    pub file_count: usize,
    pub file_list: Vec<String>,
    pub total_size: usize,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub compressed_size: Option<usize>,
}

/// Image metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMetadata {
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub exif: HashMap<String, String>,
}

/// XML metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XmlMetadata {
    pub element_count: usize,
    pub unique_elements: Vec<String>,
}

/// Text/Markdown metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextMetadata {
    pub line_count: usize,
    pub word_count: usize,
    pub character_count: usize,

    // Markdown-specific fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Vec<(String, String)>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_blocks: Option<Vec<(String, String)>>,
}

/// OCR processing metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrMetadata {
    pub language: String,
    pub psm: i32,
    pub output_format: String,
    pub table_count: usize,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_rows: Option<usize>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_cols: Option<usize>,
}

/// Error metadata (for batch operations).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMetadata {
    pub error_type: String,
    pub message: String,
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
#[serde(default)]
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
