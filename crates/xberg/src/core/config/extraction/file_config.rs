//! Per-file extraction configuration overrides for batch processing.
//!
//! This module contains [`FileExtractionConfig`], a subset of [`super::ExtractionConfig`]
//! where every field is optional. When used with batch extraction functions, each file
//! can specify overrides that are merged with the batch-level default config.
//!
//! Fields that are batch-level concerns (concurrency, caching, acceleration, security)
//! are intentionally excluded and can only be set on the batch-level [`super::ExtractionConfig`].

use serde::{Deserialize, Serialize};

use super::super::formats::OutputFormat;
use super::super::ocr::{OcrConfig, OcrStrategy};
use super::super::page::PageConfig;
use super::super::processing::{ChunkingConfig, PostProcessorConfig};
use super::types::{
    ImageExtractionConfig, LanguageDetectionConfig, MimeDetectionPolicy, TokenReductionOptions, UrlExtractionConfig,
};

/// Per-file extraction configuration overrides for batch processing.
///
/// All fields are `Option<T>` — `None` means "use the batch-level default."
/// This type is used by [`crate::ExtractInput::config`] and [`crate::extract_batch`]
/// to allow heterogeneous extraction settings within a single batch.
///
/// # Excluded Fields
///
/// The following [`super::ExtractionConfig`] fields are batch-level only and
/// cannot be overridden per file:
/// - `max_concurrent_extractions` — controls batch parallelism
/// - `use_cache` — global caching policy
/// - `acceleration` — shared ONNX execution provider
/// - `security_limits` — global archive security policy
///
/// # Example
///
/// ```rust
/// use xberg::FileExtractionConfig;
///
/// // Override just OCR forcing for a specific file
/// let config = FileExtractionConfig {
///     force_ocr: Some(true),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FileExtractionConfig {
    /// ~keep: Override MIME inference policy for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "1.1.0"))]
    pub mime_detection_policy: Option<MimeDetectionPolicy>,

    /// Override quality post-processing for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_quality_processing: Option<bool>,

    /// Override OCR configuration for this file (None in the Option = use batch default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ocr: Option<OcrConfig>,

    /// Override force OCR for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_ocr: Option<bool>,

    /// Override the OCR page-selection strategy for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ocr_strategy: Option<OcrStrategy>,

    /// Override force OCR pages for this file (1-indexed page numbers).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_ocr_pages: Option<Vec<u32>>,

    /// Override disable OCR for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_ocr: Option<bool>,

    /// Override chunking configuration for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunking: Option<ChunkingConfig>,

    /// Override content filtering configuration for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_filter: Option<super::super::content_filter::ContentFilterConfig>,

    /// Override image extraction configuration for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<ImageExtractionConfig>,

    /// Override PDF options for this file.
    #[cfg(feature = "pdf")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_options: Option<super::super::pdf::PdfConfig>,

    /// Override token reduction for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_reduction: Option<TokenReductionOptions>,

    /// Override language detection for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_detection: Option<LanguageDetectionConfig>,

    /// Override page extraction for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<PageConfig>,

    /// Override keyword extraction for this file.
    #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<crate::keywords::KeywordConfig>,

    /// Override post-processor for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postprocessor: Option<PostProcessorConfig>,

    /// Override HTML conversion options for this file.
    #[cfg(feature = "html")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_options: Option<html_to_markdown_rs::ConversionOptions>,

    /// Override styled HTML output configuration for this file.
    #[cfg(feature = "html")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_output: Option<super::super::html_output::HtmlOutputConfig>,

    /// Override result format for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_format: Option<crate::types::ResultFormat>,

    /// Override output content format for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<OutputFormat>,

    /// Override document structure output for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_document_structure: Option<bool>,

    /// Override strict parsing for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict_parse: Option<bool>,

    /// Override layout detection for this file.
    #[cfg(feature = "layout-types")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<super::super::layout::LayoutDetectionConfig>,

    /// Transcription configuration (see ExtractionConfig for docs).
    #[cfg(feature = "transcription-types")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcription: Option<super::super::transcription::TranscriptionConfig>,

    /// Override per-file extraction timeout in seconds.
    ///
    /// When set, the extraction for this file will be canceled after the
    /// specified duration. A timed-out file produces an error result without
    /// affecting other files in the batch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,

    /// Override tree-sitter configuration for this file.
    #[cfg(feature = "tree-sitter")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tree_sitter: Option<super::super::tree_sitter::TreeSitterConfig>,

    /// Override structured extraction configuration for this file.
    ///
    /// When set, enables LLM-based structured extraction with a JSON schema
    /// for this specific file. The extracted content is sent to a VLM/LLM
    /// and the response is parsed according to the provided schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_extraction: Option<super::super::llm::StructuredExtractionConfig>,

    /// Override URL ingestion and crawl configuration for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<UrlExtractionConfig>,

    /// Override named-entity recognition configuration for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ner: Option<super::super::ner::NerConfig>,

    /// Override redaction configuration for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redaction: Option<super::super::redaction::RedactionConfig>,

    /// Override summarization configuration for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summarization: Option<super::super::summarization::SummarizationConfig>,

    /// Override translation configuration for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translation: Option<super::super::translation::TranslationConfig>,

    /// Override per-page classification configuration for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_classification: Option<super::super::classification::PageClassificationConfig>,

    /// Override per-chunk classification configuration for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_classification: Option<super::super::chunk_classification::ChunkClassificationConfig>,

    /// Override VLM captioning configuration for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captioning: Option<super::super::captioning::CaptioningConfig>,

    /// Override QR-code detection for this file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qr_codes: Option<bool>,
}
