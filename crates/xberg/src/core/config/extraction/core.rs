//! Main extraction configuration struct.
//!
//! This module contains the main `ExtractionConfig` struct that aggregates all
//! configuration options for the extraction process.

use serde::{Deserialize, Serialize};

use super::super::acceleration::AccelerationConfig;
use super::super::content_filter::ContentFilterConfig;
use super::super::formats::{JupyterCellRendering, OutputFormat};
use super::super::llm::LlmConfig;
use super::super::ocr::{OcrConfig, OcrStrategy};
use super::super::page::PageConfig;
use super::super::processing::{ChunkingConfig, PostProcessorConfig};
use super::file_config::FileExtractionConfig;
use super::types::{
    ImageExtractionConfig, LanguageDetectionConfig, MimeDetectionPolicy, TokenReductionOptions, UrlExtractionConfig,
};

/// Main extraction configuration.
///
/// This struct contains all configuration options for the extraction process.
/// It can be loaded from TOML, YAML, or JSON files, or created programmatically.
///
/// # Example
///
/// ```rust
/// use xberg::core::config::ExtractionConfig;
///
/// // Create with defaults
/// let config = ExtractionConfig::default();
///
/// // Load from TOML file
/// // let config = ExtractionConfig::from_toml_file("xberg.toml")?;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractionConfig {
    /// ~keep: Controls whether MIME inference prefers content, a supported extension, or content alone.
    #[serde(default)]
    #[cfg_attr(feature = "alef-meta", alef(since = "1.1.0"))]
    pub mime_detection_policy: MimeDetectionPolicy,

    /// Enable caching of extraction results
    #[serde(default = "default_true")]
    pub use_cache: bool,

    /// Enable quality post-processing
    #[serde(default = "default_true")]
    pub enable_quality_processing: bool,

    /// OCR configuration.
    ///
    /// `None` does not run OCR for documents that already have usable text. Under
    /// `OcrStrategy::Auto`, a PDF with no text layer at all (a scan) is still routed
    /// to OCR with default settings so it is not returned empty (#1338). Set
    /// [`Self::disable_ocr`] to hard-disable OCR regardless of the detected content.
    #[serde(default)]
    pub ocr: Option<OcrConfig>,

    /// Force OCR even for searchable PDFs
    #[serde(default)]
    pub force_ocr: bool,

    /// Which pages get OCR'd when neither `force_ocr` nor `force_ocr_pages` applies.
    ///
    /// Defaults to [`OcrStrategy::Auto`], which OCRs only pages whose native text
    /// fails a quality check. Only applies to PDF documents. Cannot be
    /// [`OcrStrategy::ScannedPages`] while `disable_ocr` is `true`.
    #[serde(default, deserialize_with = "super::super::processing::deserialize_null_default")]
    pub ocr_strategy: OcrStrategy,

    /// Force OCR on specific pages only (1-indexed page numbers, must be >= 1).
    ///
    /// When set, only the listed pages are OCR'd regardless of text layer quality.
    /// Unlisted pages use native text extraction. Ignored when `force_ocr` is `true`.
    /// Only applies to PDF documents. Duplicates are automatically deduplicated.
    /// An `ocr` config is recommended for backend/language selection; defaults are used if absent.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_ocr_pages: Option<Vec<u32>>,

    /// Disable OCR entirely, even for images.
    ///
    /// When `true`, OCR is skipped for all document types. Images return metadata
    /// only (dimensions, format, EXIF) without text extraction. PDFs use only
    /// native text extraction without OCR fallback.
    ///
    /// Cannot be `true` simultaneously with `force_ocr`.
    #[serde(default)]
    pub disable_ocr: bool,

    /// Whether a PDF whose native text layer is empty or near-empty falls back to OCR
    /// under [`OcrStrategy::Auto`] (GH#1752).
    ///
    /// `None` (the default) derives the answer from whether [`Self::ocr`] is set, which is
    /// exactly the behaviour shipped before this field existed: with an `ocr` block the
    /// fallback always runs, and without one it runs only when the page's native text is
    /// *completely* empty (#1338). That carve-out means a scanned page carrying a visible
    /// page label, Bates number or scanner stamp keeps only that label.
    ///
    /// `Some(true)` applies the near-empty fallback without requiring an `ocr` block: the
    /// text-quality gate decides, using [`OcrConfig::default`]'s thresholds when no block is
    /// present, and an automatic OCR backend must be registered.
    ///
    /// `Some(false)` suppresses the fallback even when an `ocr` block is present.
    #[serde(default)]
    pub ocr_near_empty_fallback: Option<bool>,

    /// Whether [`OcrStrategy::ScannedPages`] folds the per-page text-quality gate into its
    /// page selection, on top of the pages scan detection flagged (GH#1752).
    ///
    /// `None` (the default) derives the answer from whether [`Self::ocr`] is set, which is
    /// exactly the behaviour shipped before this field existed. Without an `ocr` block
    /// `ScannedPages` therefore degrades to detected-scans-only.
    ///
    /// `Some(true)` runs the gate without requiring an `ocr` block, using
    /// [`OcrConfig::default`]'s thresholds; an automatic OCR backend must be registered.
    ///
    /// `Some(false)` selects detected scans only even when an `ocr` block is present, which
    /// is the setting that avoids paying for recognition across a whole mixed document.
    #[serde(default)]
    pub ocr_scanned_page_quality_gate: Option<bool>,

    /// Whether images embedded in a container document (DOCX, PPTX, ODT, HTML, ...) are sent
    /// to OCR (GH#1752).
    ///
    /// `None` (the default) derives the answer from whether [`Self::ocr`] is set, which is
    /// exactly the behaviour shipped before this field existed. `Some(true)` recognises
    /// picture text without requiring an `ocr` block; `Some(false)` suppresses it even when
    /// a block is present.
    ///
    /// Orthogonal to [`ImageExtractionConfig::run_ocr_on_images`], which still has to be
    /// `true` (its own default) for embedded-image OCR to run. See
    /// [`Self::runs_ocr_on_embedded_images`].
    #[serde(default)]
    pub ocr_embedded_images: Option<bool>,

    /// Text chunking configuration (None = chunking disabled)
    #[serde(default)]
    pub chunking: Option<ChunkingConfig>,

    /// Content filtering configuration (None = use extractor defaults).
    ///
    /// Controls whether document "furniture" (headers, footers, watermarks,
    /// repeating text) is included in or stripped from extraction results.
    /// See [`ContentFilterConfig`] for per-field documentation.
    #[serde(default)]
    pub content_filter: Option<ContentFilterConfig>,

    /// Image extraction configuration (None = no image extraction)
    #[serde(default)]
    pub images: Option<ImageExtractionConfig>,

    /// PDF-specific options (None = use defaults)
    #[cfg(feature = "pdf")]
    #[serde(default)]
    pub pdf_options: Option<super::super::pdf::PdfConfig>,

    /// Token reduction configuration (None = no token reduction)
    #[serde(default)]
    pub token_reduction: Option<TokenReductionOptions>,

    /// Language detection configuration (None = no language detection)
    #[serde(default)]
    pub language_detection: Option<LanguageDetectionConfig>,

    /// Page extraction configuration (None = no page tracking)
    #[serde(default)]
    pub pages: Option<PageConfig>,

    /// Keyword extraction configuration (None = no keyword extraction)
    #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
    #[serde(default)]
    pub keywords: Option<crate::keywords::KeywordConfig>,

    /// Post-processor configuration (None = use defaults)
    #[serde(default)]
    pub postprocessor: Option<PostProcessorConfig>,

    /// HTML to Markdown conversion options (None = use defaults)
    ///
    /// Configure how HTML documents are converted to Markdown, including heading styles,
    /// list formatting, code block styles, and preprocessing options.
    #[cfg(feature = "html")]
    #[serde(default)]
    pub html_options: Option<html_to_markdown_rs::ConversionOptions>,

    /// Styled HTML output configuration.
    ///
    /// When set alongside `output_format = OutputFormat::Html`, the extraction
    /// pipeline uses [`StyledHtmlRenderer`](crate::rendering::StyledHtmlRenderer)
    /// which emits stable `kb-*` CSS class hooks on every structural element
    /// and optionally embeds theme CSS or user-supplied CSS in a `<style>` block.
    ///
    /// When `None`, the existing plain comrak-based HTML renderer is used.
    #[cfg(feature = "html")]
    #[serde(default)]
    pub html_output: Option<crate::core::config::html_output::HtmlOutputConfig>,

    /// Default per-file timeout in seconds for batch extraction.
    ///
    /// When set, each file in a batch will be canceled after this duration
    /// unless overridden by [`FileExtractionConfig::timeout_secs`].
    ///
    /// Defaults to `Some(600)` (10 minutes) to prevent pathological files
    /// (e.g. deeply nested archives, documents with millions of cells) from
    /// running indefinitely and exhausting caller resources, while still
    /// giving slow paths (VLM-based OCR, large scanned documents) enough
    /// headroom to finish. Set to `None` to disable the timeout for trusted
    /// input or long-running workloads.
    #[serde(default = "ExtractionConfig::default_extraction_timeout")]
    pub extraction_timeout_secs: Option<u64>,

    /// Maximum concurrent document extractions in batch operations.
    ///
    /// This is a ceiling within the configured total thread budget, not an
    /// independent pool size. When unset, the scheduler derives document and
    /// per-document concurrency from `ConcurrencyConfig::max_threads`.
    #[serde(default)]
    pub max_concurrent_extractions: Option<usize>,

    /// Result structure format
    ///
    /// Controls whether results are returned in unified format (default) with all
    /// content in the `content` field, or element-based format with semantic
    /// elements (for Unstructured-compatible output).
    #[serde(default)]
    pub result_format: crate::types::ResultFormat,

    /// Security limits for archive extraction.
    ///
    /// Controls maximum archive size, compression ratio, file count, and other
    /// security thresholds to prevent decompression bomb attacks. Also caps
    /// nesting depth, iteration count, entity / token length, total content size,
    /// decoded image allocation, and table cell count for every extraction path
    /// that ingests user-controlled bytes.
    /// When `None`, default limits are used.
    #[serde(default)]
    pub security_limits: Option<crate::extractors::security::SecurityLimits>,

    /// Maximum uncompressed size in bytes for a single embedded file before
    /// recursive extraction is attempted (default: 50 MiB).
    ///
    /// Applies to embedded objects inside OOXML containers (DOCX, PPTX) and
    /// to email attachments processed via recursive extraction. Files that
    /// exceed this limit are skipped with a `ProcessingWarning` rather than
    /// passed to the extraction pipeline, preventing a single oversized
    /// embedded object from consuming unbounded memory or time.
    ///
    /// Set to `None` to disable the per-embedded-file cap (falls back to
    /// `security_limits.max_archive_size` as the only guard).
    #[serde(default = "ExtractionConfig::default_max_embedded_file_bytes")]
    pub max_embedded_file_bytes: Option<u64>,

    /// Content text format (default: Plain).
    ///
    /// Controls the format of the extracted content:
    /// - `Plain`: Raw extracted text (default)
    /// - `Markdown`: Markdown formatted output
    /// - `Djot`: Djot markup format (requires djot feature)
    /// - `Html`: HTML formatted output
    ///
    /// When set to a structured format, extraction results will include
    /// formatted output. The `formatted_content` field may be populated
    /// when format conversion is applied.
    #[serde(default)]
    pub output_format: OutputFormat,

    /// Escape Markdown special characters in rendered prose (default: `true`).
    ///
    /// When `output_format` is `Markdown` or `Djot`, the renderer backslash-escapes
    /// CommonMark-significant leading characters (e.g. `-`, `#`) so that literal
    /// text such as `#06-18` or `- clause` round-trips safely through a CommonMark
    /// parser instead of being reinterpreted as a heading or list marker.
    ///
    /// Table cell text is never escaped, so escaped prose can look inconsistent
    /// with table cells containing the same characters. Set this to `false` to
    /// disable prose escaping and make `content`, `pages[].content`, and
    /// `chunks[].content` read identically to table cell text — useful for LLM
    /// prompts or search indexing where CommonMark round-tripping does not matter.
    ///
    /// Defaults to `true` to preserve existing behavior.
    #[serde(default = "default_true")]
    pub escape_markdown: bool,

    /// Emit an opt-in anchor marker before each table's rendered Markdown
    /// block (default: `false`).
    ///
    /// When `output_format` is `Markdown` (or `Djot`) and this is `true`, the
    /// renderer inserts a `[TABLE:{table_id}]` marker immediately before each
    /// table's Markdown in `content`, `pages[].content`, and
    /// `chunks[].content`, where `table_id` matches the corresponding
    /// entry's [`crate::types::Table::table_id`]. This lets a consumer
    /// reconcile a rendered Markdown table block with its structured
    /// `tables[]` entry.
    ///
    /// Defaults to `false` so existing output is byte-identical unless
    /// explicitly enabled.
    #[serde(default)]
    pub table_anchors: bool,

    /// Controls how Jupyter notebook (`.ipynb`) code cells are rendered.
    ///
    /// - `Both` (default): code source plus the notebook's saved outputs
    /// - `Source`: only the code source (fenced code blocks)
    /// - `Outputs`: only the saved outputs
    ///
    /// Cells are never executed; `Outputs`/`Both` surface only outputs already
    /// stored in the notebook.
    #[serde(default)]
    pub jupyter_cell_rendering: JupyterCellRendering,

    /// Apply Jupyter Book/MyST cell visibility tags while rendering notebooks.
    ///
    /// When enabled, `remove-cell`/`hide-cell`, `remove-input`/`hide-input`,
    /// and `remove-output`/`hide-output` suppress the corresponding saved
    /// source or output. Cells are never executed, and their metadata remains
    /// available even when their rendered content is suppressed.
    ///
    /// Defaults to `true`. Set this to `false` to preserve all saved notebook
    /// content regardless of cell tags. ~keep
    #[serde(default = "default_true")]
    #[cfg_attr(feature = "alef-meta", alef(since = "1.1.0"))]
    pub apply_notebook_cell_tags: bool,

    /// Layout detection configuration (None = layout detection disabled).
    ///
    /// When set, PDF pages and images are analyzed for document structure
    /// (headings, code, formulas, tables, figures, etc.) using RT-DETR models
    /// via ONNX Runtime. For PDFs, layout hints override paragraph classification
    /// in the markdown pipeline. For images, per-region OCR is performed with
    /// markdown formatting based on detected layout classes.
    /// Requires the `layout-detection` feature to run inference; the field is
    /// present whenever the `layout-types` feature is active (which includes
    /// `layout-detection` as well as the no-ORT target groups).
    #[cfg(feature = "layout-types")]
    #[serde(default)]
    pub layout: Option<super::super::layout::LayoutDetectionConfig>,

    /// Transcription (speech-to-text) configuration for audio/video files.
    ///
    /// When set and `enabled`, files with audio/video MIME types (mp3, mp4,
    /// m4a, wav, webm, etc.) are routed to the Whisper-based transcription
    /// pipeline. The actual heavy dependencies are only active under the
    /// `transcription` feature; the field is visible under `transcription-types`
    /// (including on WASM and Android targets that use the no-ORT preset).
    ///
    /// Default: `None` (transcription disabled). This is an additive,
    /// non-breaking change.
    #[cfg(feature = "transcription-types")]
    #[serde(default)]
    pub transcription: Option<super::super::transcription::TranscriptionConfig>,

    /// Run layout detection on the non-OCR PDF markdown path.
    ///
    /// When `true` and `layout` is `Some(_)`, layout regions inform reading
    /// order, region grouping, and table detection while native font/tag
    /// semantics remain authoritative for headings, lists, code, and formulas.
    /// OCR layout classification is unchanged. This improves structural output
    /// at the cost of inference latency (~150-300ms/page CPU, ~20-50ms/page
    /// GPU). Default: `false`. Requires the `layout-detection` feature.
    #[serde(default)]
    pub use_layout_for_markdown: bool,

    /// Enable structured document tree output.
    ///
    /// When true, populates the `document` field on `ExtractedDocument` with a
    /// hierarchical `DocumentStructure` containing heading-driven section nesting,
    /// table grids, content layer classification, and inline annotations.
    ///
    /// Independent of `result_format` — can be combined with Unified or ElementBased.
    #[serde(default)]
    pub include_document_structure: bool,

    /// Fail extraction when a text-like document cannot be parsed in its own format.
    ///
    /// Default `false` (lenient): when structured parsing of YAML, JSON, TOML or XML fails
    /// (for example a Helm template, which is not YAML until rendered), or when the XML
    /// entity-length or table-cell security limit trips on a text-like input, the raw content
    /// is extracted as plain text (truncated to `security_limits.max_content_size`) and the
    /// reason is reported in `processing_warnings`. When `true`, those cases return the
    /// original error. Limits aimed at hostile input (archive size, nesting, XML depth) are
    /// never relaxed by this option.
    #[serde(default)]
    pub strict_parse: bool,

    /// Hardware acceleration configuration for ONNX Runtime models.
    ///
    /// Controls execution provider selection for layout detection and embedding
    /// models. When `None`, uses platform defaults (CoreML on macOS, CUDA on
    /// Linux, CPU on Windows).
    #[serde(default)]
    pub acceleration: Option<AccelerationConfig>,

    /// Cache namespace for tenant isolation.
    ///
    /// When set, cache entries are stored under `{cache_dir}/{namespace}/`.
    /// Must be alphanumeric, hyphens, or underscores only (max 64 chars).
    /// Different namespaces have isolated cache spaces on the same filesystem.
    #[serde(default)]
    pub cache_namespace: Option<String>,

    /// Per-request cache TTL in seconds.
    ///
    /// Overrides the global `max_age_days` for this specific extraction.
    /// When `0`, caching is completely skipped (no read or write).
    /// When `None`, the global TTL applies.
    #[serde(default)]
    pub cache_ttl_secs: Option<u64>,

    /// Email extraction configuration (None = use defaults).
    ///
    /// Currently supports configuring the fallback codepage for MSG files
    /// that do not specify one. See [`crate::core::config::EmailConfig`] for details.
    #[serde(default)]
    pub email: Option<super::super::email::EmailConfig>,

    /// CSV/TSV extraction configuration (None = use defaults).
    ///
    /// Lets callers set an explicit delimiter and declare comment-line
    /// prefixes to skip, instead of relying solely on delimiter
    /// auto-detection. See [`crate::core::config::CsvConfig`] for details.
    #[serde(default)]
    #[cfg_attr(feature = "alef-meta", alef(since = "1.1.0"))]
    pub csv: Option<super::super::csv::CsvConfig>,

    /// GeoJSON extraction configuration (None = bounded summary).
    ///
    /// By default, GeoJSON coordinates are replaced by aggregate counts and bounds
    /// so large geometry arrays do not become unbounded rendered output. Set
    /// [`crate::core::config::GeoJsonExtractionConfig::include_full_coordinates`]
    /// explicitly to retain the legacy full-coordinate output.
    #[serde(default)]
    #[cfg_attr(feature = "alef-meta", alef(since = "1.1.0"))]
    pub geojson: Option<super::super::geojson::GeoJsonExtractionConfig>,

    /// Concurrency limits for constrained environments (None = use defaults).
    ///
    /// Controls Rayon thread pool size, ONNX Runtime intra-op threads, and the
    /// combined document/inner-task budget for batch extraction. See
    /// [`crate::core::config::ConcurrencyConfig`] for details.
    #[serde(default)]
    pub concurrency: Option<super::super::concurrency::ConcurrencyConfig>,

    /// URL ingestion and crawl configuration.
    #[serde(default)]
    pub url: UrlExtractionConfig,

    /// Maximum recursion depth for archive extraction (default: 3).
    /// Set to 0 to disable recursive extraction (legacy behavior).
    #[serde(default = "ExtractionConfig::default_archive_depth")]
    pub max_archive_depth: usize,

    /// Tree-sitter language pack configuration (None = tree-sitter disabled).
    ///
    /// When set, enables code file extraction using tree-sitter parsers.
    /// Controls grammar download behavior and code analysis options.
    #[cfg(feature = "tree-sitter")]
    #[serde(default)]
    pub tree_sitter: Option<super::super::tree_sitter::TreeSitterConfig>,

    /// Structured extraction via LLM (None = disabled).
    ///
    /// When set, the extracted document content is sent to an LLM with the
    /// provided JSON schema. The structured response is stored in
    /// `ExtractedDocument::structured_output`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structured_extraction: Option<super::super::llm::StructuredExtractionConfig>,

    /// Named-entity recognition configuration. When set, the NER post-processor runs at
    /// the Middle stage and populates `ExtractedDocument::entities`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "1.0.0"))]
    pub ner: Option<super::super::ner::NerConfig>,

    /// Redaction / anonymisation configuration. When set, the redaction post-processor
    /// runs at the Late stage and rewrites every textual field in `ExtractedDocument`,
    /// emitting an audit trail in `ExtractedDocument::redaction_report`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "1.0.0"))]
    pub redaction: Option<super::super::redaction::RedactionConfig>,

    /// Summarisation configuration. When set, the summarisation post-processor runs at
    /// the Middle stage and populates `ExtractedDocument::summary`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "1.0.0"))]
    pub summarization: Option<super::super::summarization::SummarizationConfig>,

    /// Translation configuration. When set, the translation post-processor runs at the
    /// Middle stage and populates `ExtractedDocument::translation`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "1.0.0"))]
    pub translation: Option<super::super::translation::TranslationConfig>,

    /// Per-page classification configuration. When set, the classification post-processor
    /// runs at the Middle stage and populates `ExtractedDocument::page_classifications`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "1.0.0"))]
    pub page_classification: Option<super::super::classification::PageClassificationConfig>,

    /// Per-chunk multi-label classification configuration. When set, the
    /// chunk-classification post-processor runs at the Middle stage (after
    /// chunking) and populates `ChunkMetadata::classifications` on every chunk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "1.0.0"))]
    pub chunk_classification: Option<super::super::chunk_classification::ChunkClassificationConfig>,

    /// VLM captioning configuration for extracted images. When set, the captioning
    /// post-processor runs at the Middle stage and writes a caption into each
    /// `ExtractedImage::caption`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "1.0.0"))]
    pub captioning: Option<super::super::captioning::CaptioningConfig>,

    /// Enable QR-code detection in extracted images. When `true`, the QR post-processor
    /// runs at the Middle stage and populates `ExtractedImage::qr_codes`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "alef-meta", alef(since = "1.0.0"))]
    pub qr_codes: Option<bool>,

    /// Cooperative cancellation handle (`None` = no caller-initiated cancellation).
    ///
    /// Rust callers may supply a [`crate::cancellation::CancellationToken`], retain a
    /// clone, and call [`crate::cancellation::CancellationToken::cancel`] to request
    /// that extraction stop at its next cancellation checkpoint. Cancellation is
    /// cooperative rather than immediate, so latency depends on the extractor and
    /// operation currently in progress.
    ///
    /// Xberg also installs and uses cancellation tokens internally for extraction
    /// timeouts and REST async jobs. This field remains excluded from serialization
    /// and Alef-generated language bindings.
    #[serde(skip)]
    #[cfg_attr(alef, alef(skip))]
    pub cancel_token: Option<crate::cancellation::CancellationToken>,

    /// Transient source-filename hint for extension-based language detection.
    ///
    /// Set internally during extraction (from `ExtractInput::filename` or a
    /// downloaded document's filename) so extractors such as the tree-sitter
    /// code extractor can fall back to extension-based detection when
    /// content-based detection (e.g. shebang) is inconclusive. Excluded from
    /// serialization and bindings — it is not a user-facing configuration value.
    ///
    /// `pub` (not `pub(crate)`) so binding crates can construct `ExtractionConfig`
    /// via struct-update syntax (`..Default::default()`); a single private field
    /// would make that construction illegal across crates (E0451), matching the
    /// existing `cancel_token` precedent.
    #[serde(skip)]
    #[cfg_attr(alef, alef(skip))]
    pub source_name: Option<String>,
}

impl ExtractionConfig {
    /// Default [`Self::max_archive_depth`]: 3 levels of nested archives.
    ///
    /// Public and on the type rather than a free private `fn` because generated bindings
    /// have to call it to reproduce the default, and a private one is out of their reach.
    pub fn default_archive_depth() -> usize {
        3
    }

    /// Default per-embedded-file cap: 50 MiB.
    ///
    /// A single embedded object larger than this can consume significant memory
    /// when the recursive extractor materialises it. 50 MiB is generous for
    /// real-world embedded documents while still bounding worst-case allocation.
    ///
    /// Public and on the type rather than a free private `fn` because generated bindings
    /// have to call it to reproduce the default, and a private one is out of their reach.
    pub fn default_max_embedded_file_bytes() -> Option<u64> {
        Some(50 * 1024 * 1024)
    }

    /// Default extraction timeout: 600 seconds (10 minutes).
    ///
    /// Pathological files (deeply nested archives, sheets with millions of cells,
    /// adversarial PDFs) can otherwise run indefinitely and exhaust caller
    /// resources. 600 s bounds the worst-case cost of a single untrusted input
    /// while giving legitimate but slow paths — VLM-based OCR, large scanned
    /// documents — enough headroom to finish instead of being cut off at the
    /// previous 60 s default.
    ///
    /// Public and on the type rather than a free private `fn` because generated bindings
    /// have to call it to reproduce the default, and a private one is out of their reach.
    pub fn default_extraction_timeout() -> Option<u64> {
        Some(600)
    }
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            mime_detection_policy: MimeDetectionPolicy::default(),
            use_cache: true,
            enable_quality_processing: true,
            ocr: None,
            force_ocr: false,
            ocr_strategy: OcrStrategy::Auto,
            force_ocr_pages: None,
            disable_ocr: false,
            ocr_near_empty_fallback: None,
            ocr_scanned_page_quality_gate: None,
            ocr_embedded_images: None,
            chunking: None,
            content_filter: None,
            images: None,
            #[cfg(feature = "pdf")]
            pdf_options: None,
            token_reduction: None,
            language_detection: None,
            pages: None,
            #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
            keywords: None,
            postprocessor: None,
            #[cfg(feature = "html")]
            html_options: None,
            #[cfg(feature = "html")]
            html_output: None,
            extraction_timeout_secs: ExtractionConfig::default_extraction_timeout(),
            max_concurrent_extractions: None,
            security_limits: None,
            max_embedded_file_bytes: ExtractionConfig::default_max_embedded_file_bytes(),
            #[cfg(feature = "layout-types")]
            layout: None,
            #[cfg(feature = "transcription-types")]
            transcription: None,
            use_layout_for_markdown: false,
            result_format: crate::types::ResultFormat::Unified,
            output_format: OutputFormat::Plain,
            escape_markdown: true,
            table_anchors: false,
            jupyter_cell_rendering: JupyterCellRendering::Both,
            apply_notebook_cell_tags: true,
            include_document_structure: false,
            strict_parse: false,
            acceleration: None,
            cache_namespace: None,
            cache_ttl_secs: None,
            email: None,
            csv: None,
            geojson: None,
            concurrency: None,
            url: UrlExtractionConfig::default(),
            max_archive_depth: ExtractionConfig::default_archive_depth(),
            #[cfg(feature = "tree-sitter")]
            tree_sitter: None,
            structured_extraction: None,
            ner: None,
            redaction: None,
            summarization: None,
            translation: None,
            page_classification: None,
            chunk_classification: None,
            captioning: None,
            qr_codes: None,
            cancel_token: None,
            source_name: None,
        }
    }
}

impl ExtractionConfig {
    /// Resolve layout acceleration, preferring an explicit nested setting.
    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    pub(crate) fn resolved_layout_acceleration(&self) -> Option<&AccelerationConfig> {
        self.layout
            .as_ref()
            .and_then(|layout| layout.acceleration.as_ref())
            .or(self.acceleration.as_ref())
    }

    /// Resolve layout configuration with global acceleration as its fallback.
    ///
    /// An explicit layout-specific acceleration setting always takes precedence.
    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    pub(crate) fn resolved_layout_config(
        &self,
    ) -> Option<std::borrow::Cow<'_, super::super::layout::LayoutDetectionConfig>> {
        let layout = self.layout.as_ref()?;
        let acceleration = self.resolved_layout_acceleration();
        if layout.acceleration.is_some() || acceleration.is_none() {
            return Some(std::borrow::Cow::Borrowed(layout));
        }

        let mut resolved = layout.clone();
        resolved.acceleration = acceleration.cloned();
        Some(std::borrow::Cow::Owned(resolved))
    }

    /// Install an internal cancellation token when a timeout is configured but no
    /// caller supplied one.
    ///
    /// `extraction_timeout_secs` fires `token.cancel()` at every timeout call site
    /// (`core/extractor/{file,bytes,batch}.rs`, `engine/extract_impl.rs`). Binding-
    /// driven and CLI-driven calls generally leave `cancel_token` as `None` unless
    /// a Rust caller explicitly supplies one. Without this fallback, `token.cancel()`
    /// has nothing to signal: the timeout stops *waiting* and returns
    /// `XbergError::Timeout`, but spawned extraction work can continue running to
    /// completion and consume a worker thread.
    ///
    /// A caller-supplied token is always left untouched, so Rust callers and the
    /// REST cancellation path continue observing the same shared token.
    ///
    /// No-op when `extraction_timeout_secs` is `None`: without an internal timeout
    /// path there is no need to allocate a fallback token. Callers may still supply
    /// their own token for explicit cancellation.
    ///
    /// Unconditional on target/feature: the token and its atomic operations are
    /// available on every target, including wasm32. Timeout call sites remain gated
    /// independently where runtime support requires it.
    pub(crate) fn ensure_cancel_token(&mut self) {
        if self.extraction_timeout_secs.is_some() && self.cancel_token.is_none() {
            self.cancel_token = Some(crate::cancellation::CancellationToken::default());
        }
    }

    /// Create a new `ExtractionConfig` by applying per-file overrides from a
    /// [`FileExtractionConfig`]. Fields that are `Some` in the override replace the
    /// corresponding field in `self`; `None` fields keep the original value.
    ///
    /// Batch-level fields (`max_concurrent_extractions`, `use_cache`, `acceleration`,
    /// `security_limits`) are never affected by overrides.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use xberg::{ExtractionConfig, FileExtractionConfig};
    ///
    /// let base = ExtractionConfig::default();
    /// let override_config = FileExtractionConfig {
    ///     force_ocr: Some(true),
    ///     ..Default::default()
    /// };
    /// let resolved = base.with_file_overrides(&override_config);
    /// assert!(resolved.force_ocr);
    /// ```
    pub(crate) fn with_file_overrides(&self, overrides: &FileExtractionConfig) -> Self {
        let FileExtractionConfig {
            ref mime_detection_policy,
            ref enable_quality_processing,
            ref ocr,
            ref force_ocr,
            ref ocr_strategy,
            ref force_ocr_pages,
            ref disable_ocr,
            ref chunking,
            ref content_filter,
            ref images,
            #[cfg(feature = "pdf")]
            ref pdf_options,
            ref token_reduction,
            ref language_detection,
            ref pages,
            #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
            ref keywords,
            ref postprocessor,
            #[cfg(feature = "html")]
            ref html_options,
            #[cfg(feature = "html")]
            ref html_output,
            ref result_format,
            ref output_format,
            ref include_document_structure,
            ref strict_parse,
            #[cfg(feature = "layout-types")]
            ref layout,
            #[cfg(feature = "transcription-types")]
            ref transcription,
            ref timeout_secs,
            #[cfg(feature = "tree-sitter")]
            ref tree_sitter,
            ref structured_extraction,
            ref url,
            ref ner,
            ref redaction,
            ref summarization,
            ref translation,
            ref page_classification,
            ref chunk_classification,
            ref captioning,
            ref qr_codes,
        } = *overrides;

        let mut config = self.clone();

        if let Some(v) = mime_detection_policy {
            config.mime_detection_policy = *v;
        }

        if let Some(v) = enable_quality_processing {
            config.enable_quality_processing = *v;
        }
        if let Some(v) = ocr {
            config.ocr = Some(v.clone());
        }
        if let Some(v) = force_ocr {
            config.force_ocr = *v;
        }
        if let Some(v) = ocr_strategy {
            config.ocr_strategy = v.clone();
        }
        if let Some(v) = force_ocr_pages {
            config.force_ocr_pages = Some(v.clone());
        }
        if let Some(v) = disable_ocr {
            config.disable_ocr = *v;
        }
        if let Some(v) = chunking {
            config.chunking = Some(v.clone());
        }
        if let Some(v) = content_filter {
            config.content_filter = Some(v.clone());
        }
        if let Some(v) = images {
            config.images = Some(v.clone());
        }
        #[cfg(feature = "pdf")]
        if let Some(v) = pdf_options {
            config.pdf_options = Some(v.clone());
        }
        if let Some(v) = token_reduction {
            config.token_reduction = Some(v.clone());
        }
        if let Some(v) = language_detection {
            config.language_detection = Some(v.clone());
        }
        if let Some(v) = pages {
            config.pages = Some(v.clone());
        }
        #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
        if let Some(v) = keywords {
            config.keywords = Some(v.clone());
        }
        if let Some(v) = postprocessor {
            config.postprocessor = Some(v.clone());
        }
        #[cfg(feature = "html")]
        if let Some(v) = html_options {
            config.html_options = Some(v.clone());
        }
        #[cfg(feature = "html")]
        if let Some(v) = html_output {
            config.html_output = Some(v.clone());
        }
        if let Some(v) = result_format {
            config.result_format = *v;
        }
        if let Some(v) = output_format {
            config.output_format = v.clone();
        }
        if let Some(v) = include_document_structure {
            config.include_document_structure = *v;
        }
        if let Some(v) = strict_parse {
            config.strict_parse = *v;
        }
        #[cfg(feature = "layout-types")]
        if let Some(v) = layout {
            config.layout = Some(v.clone());
        }
        #[cfg(feature = "transcription-types")]
        if let Some(v) = transcription {
            config.transcription = Some(v.clone());
        }
        if let Some(v) = timeout_secs {
            config.extraction_timeout_secs = Some(*v);
        }
        #[cfg(feature = "tree-sitter")]
        if let Some(v) = tree_sitter {
            config.tree_sitter = Some(v.clone());
        }
        if let Some(v) = structured_extraction {
            config.structured_extraction = Some(v.clone());
        }
        if let Some(v) = url {
            config.url = v.clone();
        }
        if let Some(v) = ner {
            config.ner = Some(v.clone());
        }
        if let Some(v) = redaction {
            config.redaction = Some(v.clone());
        }
        if let Some(v) = summarization {
            config.summarization = Some(v.clone());
        }
        if let Some(v) = translation {
            config.translation = Some(v.clone());
        }
        if let Some(v) = page_classification {
            config.page_classification = Some(v.clone());
        }
        if let Some(v) = chunk_classification {
            config.chunk_classification = Some(v.clone());
        }
        if let Some(v) = captioning {
            config.captioning = Some(v.clone());
        }
        if let Some(v) = qr_codes {
            config.qr_codes = Some(*v);
        }

        config
    }

    /// Normalize configuration for implicit requirements.
    ///
    /// Currently handles:
    /// - Auto-enabling `extract_pages` when `result_format` is `ElementBased`, because
    ///   the element transformation requires per-page data to assign correct page numbers.
    ///   Without this, all elements would incorrectly get `page_number=1`.
    /// - Auto-enabling `extract_pages` when chunking is configured, because the chunker
    ///   needs page boundaries to assign correct page numbers to chunks.
    pub(crate) fn normalized(&self) -> std::borrow::Cow<'_, Self> {
        let needs_pages = |cfg: &Self| -> bool {
            match &cfg.pages {
                Some(page_config) => !page_config.extract_pages,
                None => true,
            }
        };

        let needs_pages_for_elements =
            self.result_format == crate::types::ResultFormat::ElementBased && needs_pages(self);
        let needs_pages_for_chunking = self.chunking.is_some() && needs_pages(self);

        if needs_pages_for_elements || needs_pages_for_chunking {
            let mut config = self.clone();
            let page_config = config.pages.get_or_insert_with(super::super::page::PageConfig::default);
            page_config.extract_pages = true;
            return std::borrow::Cow::Owned(config);
        }
        std::borrow::Cow::Borrowed(self)
    }

    fn validate_nested_llm_configs(&self, validate: fn(&LlmConfig) -> crate::Result<()>) -> crate::Result<()> {
        self.validate_ocr_llm_configs(validate)?;
        self.validate_chunking_llm_config(validate)?;
        self.validate_direct_llm_configs(validate)
    }

    fn validate_ocr_llm_configs(&self, validate: fn(&LlmConfig) -> crate::Result<()>) -> crate::Result<()> {
        let Some(ocr) = self.ocr.as_ref() else {
            return Ok(());
        };
        if let Some(llm) = ocr.vlm_config.as_ref() {
            validate_nested_llm("ocr.vlm_config", llm, validate)?;
        }
        if let Some(pipeline) = ocr.pipeline.as_ref() {
            for stage in &pipeline.stages {
                if let Some(llm) = stage.vlm_config.as_ref() {
                    validate_nested_llm("ocr.pipeline[].vlm_config", llm, validate)?;
                }
            }
        }
        Ok(())
    }

    fn validate_chunking_llm_config(&self, validate: fn(&LlmConfig) -> crate::Result<()>) -> crate::Result<()> {
        let llm = self
            .chunking
            .as_ref()
            .and_then(|chunking| chunking.embedding.as_ref())
            .and_then(|embedding| match &embedding.model {
                super::super::processing::EmbeddingModelType::Llm { llm } => Some(llm.as_ref()),
                _ => None,
            });
        if let Some(llm) = llm {
            validate_nested_llm("chunking.embedding.model.llm", llm, validate)?;
        }
        Ok(())
    }

    fn validate_direct_llm_configs(&self, validate: fn(&LlmConfig) -> crate::Result<()>) -> crate::Result<()> {
        let configs = [
            (
                "structured_extraction.llm",
                self.structured_extraction.as_ref().map(|config| &config.llm),
            ),
            ("ner.llm", self.ner.as_ref().and_then(|config| config.llm.as_ref())),
            (
                "summarization.llm",
                self.summarization.as_ref().and_then(|config| config.llm.as_ref()),
            ),
            ("translation.llm", self.translation.as_ref().map(|config| &config.llm)),
            (
                "page_classification.llm",
                self.page_classification.as_ref().map(|config| &config.llm),
            ),
            (
                "chunk_classification.llm",
                self.chunk_classification.as_ref().map(|config| &config.llm),
            ),
            ("captioning.llm", self.captioning.as_ref().map(|config| &config.llm)),
        ];
        for (path, llm) in configs {
            if let Some(llm) = llm {
                validate_nested_llm(path, llm, validate)?;
            }
        }
        Ok(())
    }

    /// Validate the configuration, returning an error if any settings are invalid.
    ///
    /// Checks:
    /// - `ocr`: backend name, VLM backend/model requirements, language codes, and the
    ///   `vlm_fallback` quality threshold.
    /// - `chunking`: `max_characters` is non-zero and `overlap` is smaller than it.
    ///   `topic_threshold`, when set, is a finite `[0.0, 1.0]` value.
    /// - `token_reduction`: `mode` is one of the recognized reduction levels.
    /// - `images`: `target_dpi`, `min_dpi`, and `max_dpi` are all positive and within the
    ///   supported range.
    /// - `language_detection`: `min_confidence` is a `[0.0, 1.0]` value.
    /// - `csv`: `delimiter`, when set, is exactly one ASCII character.
    /// - `keywords`: the n-gram range contains positive, ordered bounds and `min_score` is a
    ///   finite `[0.0, 1.0]` value.
    /// - `layout`: `confidence_threshold`, when set, is a finite `[0.0, 1.0]` value.
    /// - `redaction`: custom terms, patterns, and external findings are valid.
    /// - `pdf_options`: hierarchy cluster counts and margin fractions are in their supported ranges.
    /// - every nested LLM config: sampling ranges and target-specific authentication support.
    ///
    /// Called automatically when a config is loaded from a file, built from a JSON override,
    /// or passed to the public `extract` and `extract_batch` entry points. Call this method
    /// explicitly before passing a typed config to lower-level processing APIs.
    ///
    /// # Errors
    ///
    /// Returns `XbergError::Validation` describing the first invalid setting found.
    pub fn validate(&self) -> Result<(), crate::XbergError> {
        use crate::core::config_validation::{
            validate_chunking_params, validate_confidence, validate_csv_delimiter, validate_dpi,
            validate_token_reduction_level,
        };

        if let Some(ref ocr) = self.ocr {
            ocr.validate()?;
        }
        self.ocr_strategy.validate()?;

        #[cfg(feature = "pdf")]
        if let Some(ref pdf_options) = self.pdf_options {
            pdf_options.validate()?;
        }

        if let Some(ref pages) = self.pages {
            pages.validate()?;
        }

        self.validate_nested_llm_configs(LlmConfig::validate)?;

        #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
        if let Some(ref keywords) = self.keywords {
            keywords.validate()?;
        }

        if let Some(ref chunking) = self.chunking {
            if let Some(topic_threshold) = chunking.topic_threshold {
                validate_unit_interval("chunking.topic_threshold", topic_threshold)?;
            }

            // Only meaningful when the raw fields are the ones that will be used. A `preset`
            // replaces both `max_characters` and `overlap` in `ChunkingConfig::resolve_preset`,
            // and both fields carry serde defaults, so validating them alongside a preset would
            // reject a config on values the preset discards before anything reads them. ~keep
            if chunking.preset.is_none() {
                validate_chunking_params(chunking.max_characters, chunking.overlap)?;
            }
        }

        if let Some(ref token_reduction) = self.token_reduction {
            validate_token_reduction_level(&token_reduction.mode)?;
        }

        if let Some(ref images) = self.images {
            validate_dpi(images.target_dpi)?;
            validate_dpi(images.min_dpi)?;
            validate_dpi(images.max_dpi)?;
        }

        if let Some(ref language_detection) = self.language_detection {
            validate_confidence(language_detection.min_confidence)?;
        }

        #[cfg(feature = "layout-types")]
        if let Some(ref layout) = self.layout
            && let Some(confidence_threshold) = layout.confidence_threshold
        {
            validate_unit_interval("layout.confidence_threshold", confidence_threshold)?;
        }

        if let Some(ref redaction) = self.redaction {
            redaction.validate()?;
        }

        if let Some(ref csv) = self.csv
            && let Some(ref delimiter) = csv.delimiter
        {
            validate_csv_delimiter(delimiter)?;
        }

        Ok(())
    }

    /// Returns the effective disable-OCR value, accounting for both the top-level
    /// `disable_ocr` flag and the `ocr.enabled` shorthand on [`OcrConfig`].
    ///
    /// Setting `ocr.enabled = false` in configuration is treated as equivalent to
    /// `disable_ocr = true`. This method is the single source of truth for whether
    /// OCR should be skipped.
    pub(crate) fn effective_disable_ocr(&self) -> bool {
        self.disable_ocr || self.ocr.as_ref().is_some_and(|o| !o.enabled)
    }

    /// Check if image processing is needed by examining OCR and image extraction settings.
    ///
    /// Returns `true` if either OCR is enabled or image extraction is configured,
    /// indicating that image decompression and processing should occur.
    /// Returns `false` if both are disabled, allowing optimization to skip unnecessary
    /// image decompression for text-only extraction workflows.
    ///
    /// # Optimization Impact
    /// For text-only extractions (no OCR, no image extraction), skipping image
    /// decompression can improve CPU utilization by 5-10% by avoiding wasteful
    /// image I/O and processing when results won't be used.
    /// Returns `true` when a CONTAINER document (DOCX, PPT, PPTX, HTML, ...) needs
    /// to read an embedded image's binary data out of its archive or source.
    ///
    /// True when `config.images.extract_images` is set, captioning is configured, QR-code
    /// detection is enabled, or embedded images get OCR'd. All four consume the bytes:
    /// captioning and QR-code detection regardless of whether the caller also requested
    /// image extraction, and embedded-image OCR because the OCR path reads the same bytes
    /// the container would otherwise skip (GH#1662): before the OCR disjunct existed, a
    /// container asked this question, was told no, and attached an image with an empty
    /// buffer, which OCR then ran on and reported `Could not determine image format`.
    ///
    /// This governs whether a CONTAINER reads bytes it does not otherwise have. A
    /// standalone image extractor already holds the whole file's bytes regardless of this
    /// answer, so it must not use this method to decide whether to put its OWN bytes in the
    /// result's `images` array; that is [`wants_own_bytes_in_result`](Self::wants_own_bytes_in_result).
    pub fn needs_image_data(&self) -> bool {
        self.images.as_ref().is_some_and(|i| i.extract_images)
            || self.runs_ocr_on_embedded_images()
            || self.captioning.is_some()
            || self.qr_codes == Some(true)
    }

    /// Whether embedded images get OCR'd.
    ///
    /// This is THE condition -- `core/pipeline/mod.rs` and
    /// `extraction::image_ocr::process_images_with_ocr` both call this method rather than
    /// re-deriving it, because when two copies of it drifted apart a container extractor
    /// asked `needs_image_data` and was told no, so it attached an image with an empty
    /// buffer, and the OCR path then ran on those zero bytes and reported `Could not
    /// determine image format` (GH#1662). A re-derived copy anywhere reopens that defect;
    /// `embedded_image_ocr_gate_has_no_second_copy` in `core/pipeline/tests.rs` fails if
    /// one appears. ~keep
    ///
    /// [`Self::ocr_embedded_images`] is the caller's explicit answer to the OCR half;
    /// `None` derives it from whether an `ocr` block is present, which is what this
    /// condition was before GH#1752 gave the behaviour a setting of its own.
    ///
    /// `disable_ocr` still wins regardless: it is documented as skipping OCR "for all
    /// document types", and `ocr_embedded_images` (like the plain presence of an `ocr`
    /// block before it) is an AUTOMATIC trigger, not an explicit request like `force_ocr` --
    /// see [`Self::effective_disable_ocr`]'s callers elsewhere (`needs_image_processing`,
    /// `extractors/image.rs`, `engine/extract_impl.rs`) for the same precedent.
    pub fn runs_ocr_on_embedded_images(&self) -> bool {
        !self.effective_disable_ocr()
            && self.ocr_embedded_images.unwrap_or(self.ocr.is_some())
            && self.images.as_ref().map(|i| i.run_ocr_on_images).unwrap_or(true)
    }

    /// Returns `true` when a standalone image extraction (the whole input document IS
    /// the image, not a container's embedded picture) should attach its own already-read
    /// bytes to the result's `images` array.
    ///
    /// Deliberately narrower than [`needs_image_data`](Self::needs_image_data): a standalone
    /// image's content is read in full before this question is ever asked, whether or not OCR
    /// runs, so OCR being configured is not by itself a reason to echo those bytes back in the
    /// public output. Using `needs_image_data` here (as `needs_image_data`'s own OCR disjunct
    /// briefly did) would silently start returning every OCR'd standalone image's raw bytes in
    /// `images`, a public output contract change with nothing asking for it. Same three
    /// conditions as before that disjunct existed: explicit image extraction, captioning, or
    /// QR-code detection.
    pub fn wants_own_bytes_in_result(&self) -> bool {
        self.images.as_ref().is_some_and(|i| i.extract_images)
            || self.captioning.is_some()
            || self.qr_codes == Some(true)
    }

    /// Returns `true` when any image processing is needed during extraction.
    ///
    /// # Optimization Impact
    ///
    /// For text-only extractions (no OCR, no image extraction, no captioning), skipping
    /// image decompression can improve CPU utilization by 5-10% by avoiding wasteful
    /// image I/O and processing when results won't be used.
    pub fn needs_image_processing(&self) -> bool {
        let ocr_enabled = !self.effective_disable_ocr() && (self.ocr.is_some() || self.force_ocr);

        #[cfg(feature = "layout-detection")]
        let layout_enabled = self.layout.is_some();
        #[cfg(not(feature = "layout-detection"))]
        let layout_enabled = false;

        ocr_enabled || self.needs_image_data() || layout_enabled
    }
}

fn validate_unit_interval(field: &str, value: f32) -> crate::Result<()> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        return Ok(());
    }

    Err(crate::XbergError::validation(format!(
        "{field} must be a finite value between 0.0 and 1.0, got {value}"
    )))
}

fn validate_nested_llm(
    path: &str,
    llm: &LlmConfig,
    validate: fn(&LlmConfig) -> crate::Result<()>,
) -> crate::Result<()> {
    validate(llm).map_err(|error| match error {
        crate::XbergError::Validation { message, source } => crate::XbergError::Validation {
            message: format!("{path}: {message}"),
            source,
        },
        other => other,
    })
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    /// Polyglot bindings serialize a zero-valued mirror struct with every field
    /// present, so `ocr_strategy` arrives as an explicit `null`. `#[serde(default)]`
    /// only covers a *missing* key; an internally-tagged enum rejects `null`.
    /// Caught by the Go e2e suite: "invalid type: null, expected internally tagged
    /// enum OcrStrategy".
    #[test]
    fn ocr_strategy_accepts_an_explicit_null() {
        let config: ExtractionConfig =
            serde_json::from_str(r#"{"ocr_strategy": null}"#).expect("null must deserialize");
        assert_eq!(config.ocr_strategy, OcrStrategy::Auto);
    }

    #[test]
    fn ocr_strategy_accepts_a_missing_key() {
        let config: ExtractionConfig = serde_json::from_str("{}").expect("missing key must deserialize");
        assert_eq!(config.ocr_strategy, OcrStrategy::Auto);
    }

    #[test]
    fn ocr_strategy_round_trips_its_payload_variant() {
        let json = r#"{"ocr_strategy": {"mode": "scanned_pages", "min_confidence": 0.7}}"#;
        let config: ExtractionConfig = serde_json::from_str(json).expect("payload variant must deserialize");
        assert_eq!(config.ocr_strategy, OcrStrategy::ScannedPages { min_confidence: 0.7 });
    }

    /// Regression test for task #709: without an internal fallback token, the
    /// `token.cancel()` calls at every `extraction_timeout_secs` call site
    /// (`core/extractor/{file,bytes,batch}.rs`, `engine/extract_impl.rs`) have nothing
    /// to signal on calls that do not explicitly supply a token. A Rust caller or the
    /// REST job store may provide one; other paths still require the internal fallback.
    #[test]
    fn ensure_cancel_token_installs_a_token_when_a_timeout_is_configured_and_none_was_supplied() {
        let mut config = ExtractionConfig {
            extraction_timeout_secs: Some(30),
            cancel_token: None,
            ..Default::default()
        };
        config.ensure_cancel_token();
        assert!(
            config.cancel_token.is_some(),
            "a timeout without a caller-supplied token must get an internal fallback"
        );
    }

    /// A caller-supplied token, whether retained by a Rust caller or the REST job
    /// cancellation path, must survive unchanged. `ensure_cancel_token` must never
    /// replace it with a different token or the retained handle would stop observing
    /// the token extractors poll.
    #[test]
    fn ensure_cancel_token_preserves_a_caller_supplied_token() {
        let supplied = crate::cancellation::CancellationToken::new();
        let mut config = ExtractionConfig {
            extraction_timeout_secs: Some(30),
            cancel_token: Some(supplied.clone()),
            ..Default::default()
        };
        config.ensure_cancel_token();

        supplied.cancel();
        assert!(
            config.cancel_token.expect("token must still be present").is_cancelled(),
            "ensure_cancel_token must keep the SAME token (a clone of the same Arc), not \
             install an unrelated one that never observes the caller's cancel() call"
        );
    }

    /// Without a configured timeout, no internal timeout path needs an automatically
    /// installed token. A caller may still supply its own explicit cancellation token.
    #[test]
    fn ensure_cancel_token_is_a_noop_without_a_configured_timeout() {
        let mut config = ExtractionConfig {
            extraction_timeout_secs: None,
            cancel_token: None,
            ..Default::default()
        };
        config.ensure_cancel_token();
        assert!(
            config.cancel_token.is_none(),
            "no timeout means no internal fallback token is needed"
        );
    }

    use super::*;
    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    use crate::core::config::{AccelerationConfig, ExecutionProviderType, LayoutDetectionConfig};
    use crate::core::config::{
        CaptioningConfig, LlmConfig, NerConfig, OcrConfig, PageClassificationConfig, RedactionConfig,
        SummarizationConfig, TranslationConfig,
    };

    fn wasm_managed_llm_json() -> serde_json::Value {
        serde_json::json!({
            "model": "test/model",
            "credential_provider": { "type": "vertex_adc" }
        })
    }

    fn wasm_ocr_and_embedding_cases(llm: &serde_json::Value) -> Vec<(&'static str, serde_json::Value)> {
        vec![
            (
                "ocr.vlm_config",
                serde_json::json!({ "ocr": { "vlm_config": llm.clone() } }),
            ),
            (
                "ocr.pipeline[].vlm_config",
                serde_json::json!({
                    "ocr": { "pipeline": { "stages": [{ "backend": "vlm", "vlm_config": llm.clone() }] } }
                }),
            ),
            (
                "chunking.embedding.model.llm",
                serde_json::json!({
                    "chunking": { "embedding": { "model": { "type": "llm", "llm": llm.clone() } } }
                }),
            ),
        ]
    }

    fn wasm_direct_llm_cases(llm: &serde_json::Value) -> Vec<(&'static str, serde_json::Value)> {
        vec![
            (
                "structured_extraction.llm",
                serde_json::json!({ "structured_extraction": { "schema": {}, "llm": llm.clone() } }),
            ),
            (
                "ner.llm",
                serde_json::json!({ "ner": { "backend": "llm", "llm": llm.clone() } }),
            ),
            (
                "summarization.llm",
                serde_json::json!({ "summarization": { "llm": llm.clone() } }),
            ),
            (
                "translation.llm",
                serde_json::json!({ "translation": { "target_lang": "de", "llm": llm.clone() } }),
            ),
            (
                "page_classification.llm",
                serde_json::json!({ "page_classification": { "labels": ["invoice"], "llm": llm.clone() } }),
            ),
            (
                "chunk_classification.llm",
                serde_json::json!({
                    "chunk_classification": {
                        "definitions": [{ "label": "invoice", "description": "An invoice" }],
                        "llm": llm.clone()
                    }
                }),
            ),
            (
                "captioning.llm",
                serde_json::json!({ "captioning": { "llm": llm.clone() } }),
            ),
        ]
    }

    #[test]
    fn should_reject_wasm_credential_provider_in_every_nested_llm_config() {
        let llm = wasm_managed_llm_json();
        let cases = wasm_ocr_and_embedding_cases(&llm)
            .into_iter()
            .chain(wasm_direct_llm_cases(&llm));

        for (path, json) in cases {
            let config: ExtractionConfig = serde_json::from_value(json).expect("nested config must deserialize");
            let error = config
                .validate_nested_llm_configs(LlmConfig::validate_for_wasm_target)
                .expect_err("managed credential provider must be rejected for wasm");
            assert!(error.to_string().contains(path), "missing path `{path}` in {error}");
        }
    }

    #[test]
    fn should_validate_nested_llm_configs_through_public_extraction_validation() {
        let config: ExtractionConfig = serde_json::from_value(serde_json::json!({
            "structured_extraction": {
                "schema": {},
                "llm": { "model": "test/model", "top_p": 2.0 }
            }
        }))
        .expect("nested config must deserialize");

        let error = config
            .validate()
            .expect_err("nested invalid LLM config must be rejected");
        assert!(
            error.to_string().contains("structured_extraction.llm"),
            "nested validation error must identify its config path: {error}"
        );
    }

    #[test]
    fn should_reject_invalid_ocr_element_confidence_through_public_validation() {
        for min_confidence in [-0.1, 1.1, f64::NAN, f64::INFINITY] {
            let config = ExtractionConfig {
                ocr: Some(crate::core::config::OcrConfig {
                    element_config: Some(crate::types::OcrElementConfig {
                        min_confidence,
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            };

            let error = config
                .validate()
                .expect_err("top-level validation must reject invalid OCR element confidence");
            assert!(
                error.to_string().contains("ocr.element_config.min_confidence"),
                "validation error must identify the invalid field: {error}"
            );
        }
    }

    #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
    #[test]
    fn should_reject_direct_invalid_nested_keyword_range() {
        let config = ExtractionConfig {
            keywords: Some(crate::keywords::KeywordConfig {
                ngram_range: crate::keywords::NgramRange { min: 0, max: 3 },
                ..Default::default()
            }),
            ..Default::default()
        };

        let error = config
            .validate()
            .expect_err("top-level validation must reject an invalid nested keyword range");
        assert_eq!(
            error.to_string(),
            "Validation error: ngram range minimum must be at least 1, got 0"
        );
    }

    #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
    #[test]
    fn should_reject_direct_invalid_nested_keyword_score() {
        for min_score in [-0.1, 1.1, f32::NAN] {
            let config = ExtractionConfig {
                keywords: Some(crate::keywords::KeywordConfig {
                    min_score,
                    ..Default::default()
                }),
                ..Default::default()
            };

            let error = config
                .validate()
                .expect_err("top-level validation must reject an invalid nested keyword score");
            assert!(
                error.to_string().contains("keywords.min_score"),
                "validation error must identify the invalid field: {error}"
            );
        }
    }

    #[test]
    fn should_reject_invalid_semantic_topic_threshold() {
        for topic_threshold in [-0.1, 1.1, f32::NAN] {
            let config = ExtractionConfig {
                chunking: Some(crate::core::config::ChunkingConfig {
                    topic_threshold: Some(topic_threshold),
                    ..Default::default()
                }),
                ..Default::default()
            };

            let error = config
                .validate()
                .expect_err("top-level validation must reject an invalid semantic threshold");
            assert!(
                error.to_string().contains("chunking.topic_threshold"),
                "validation error must identify the invalid field: {error}"
            );
        }
    }

    #[cfg(feature = "layout-types")]
    #[test]
    fn should_reject_invalid_layout_confidence_threshold() {
        for confidence_threshold in [-0.1, 1.1, f32::NAN] {
            let config = ExtractionConfig {
                layout: Some(crate::core::config::LayoutDetectionConfig {
                    confidence_threshold: Some(confidence_threshold),
                    ..Default::default()
                }),
                ..Default::default()
            };

            let error = config
                .validate()
                .expect_err("top-level validation must reject an invalid layout threshold");
            assert!(
                error.to_string().contains("layout.confidence_threshold"),
                "validation error must identify the invalid field: {error}"
            );
        }
    }

    #[test]
    fn should_reject_invalid_nested_redaction_pattern() {
        let config = ExtractionConfig {
            redaction: Some(RedactionConfig {
                custom_patterns: vec![crate::core::config::RedactionPattern::labeled("broken", "(")],
                ..Default::default()
            }),
            ..Default::default()
        };

        let error = config
            .validate()
            .expect_err("top-level validation must reject an invalid redaction pattern");
        assert!(
            error.to_string().contains("broken"),
            "validation error must identify the invalid custom pattern: {error}"
        );
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn should_reject_invalid_nested_pdf_options() {
        let config = ExtractionConfig {
            pdf_options: Some(crate::core::config::PdfConfig {
                top_margin_fraction: Some(f32::NAN),
                ..Default::default()
            }),
            ..Default::default()
        };

        let error = config
            .validate()
            .expect_err("top-level validation must reject invalid PDF options");
        assert!(
            error.to_string().contains("pdf_options.top_margin_fraction"),
            "validation error must identify the invalid field: {error}"
        );
    }

    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    #[test]
    fn resolved_layout_config_uses_global_acceleration_as_fallback() {
        let config = ExtractionConfig {
            layout: Some(Default::default()),
            acceleration: Some(AccelerationConfig {
                provider: ExecutionProviderType::Cpu,
                ..Default::default()
            }),
            ..Default::default()
        };

        let resolved = config.resolved_layout_config().expect("layout must be enabled");
        assert_eq!(
            config
                .resolved_layout_acceleration()
                .map(|acceleration| &acceleration.provider),
            Some(&ExecutionProviderType::Cpu)
        );
        assert_eq!(
            resolved
                .acceleration
                .as_ref()
                .map(|acceleration| &acceleration.provider),
            Some(&ExecutionProviderType::Cpu)
        );
    }

    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    #[test]
    fn resolved_layout_config_prefers_explicit_nested_auto_acceleration() {
        let config = ExtractionConfig {
            layout: Some(LayoutDetectionConfig {
                acceleration: Some(AccelerationConfig {
                    provider: ExecutionProviderType::Auto,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            acceleration: Some(AccelerationConfig {
                provider: ExecutionProviderType::Cpu,
                ..Default::default()
            }),
            ..Default::default()
        };

        let resolved = config.resolved_layout_config().expect("layout must be enabled");
        assert_eq!(
            config
                .resolved_layout_acceleration()
                .map(|acceleration| &acceleration.provider),
            Some(&ExecutionProviderType::Auto)
        );
        assert_eq!(
            resolved
                .acceleration
                .as_ref()
                .map(|acceleration| &acceleration.provider),
            Some(&ExecutionProviderType::Auto)
        );
    }

    #[test]
    fn test_effective_disable_ocr_from_top_level_flag() {
        let config = ExtractionConfig {
            disable_ocr: true,
            ..Default::default()
        };
        assert!(config.effective_disable_ocr());
    }

    #[test]
    fn test_effective_disable_ocr_from_ocr_enabled_false() {
        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                enabled: false,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(
            config.effective_disable_ocr(),
            "ocr.enabled = false should be treated as disable_ocr = true"
        );
    }

    #[test]
    fn test_effective_disable_ocr_default_is_false() {
        let config = ExtractionConfig::default();
        assert!(!config.effective_disable_ocr());
    }

    #[test]
    fn test_effective_disable_ocr_ocr_enabled_true_does_not_disable() {
        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                enabled: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(!config.effective_disable_ocr());
    }

    #[test]
    fn test_ocr_enabled_false_deserialized_from_json() {
        let json = r#"{"ocr": {"enabled": false}}"#;
        let config: ExtractionConfig = serde_json::from_str(json).unwrap();
        assert!(
            config.effective_disable_ocr(),
            "JSON ocr.enabled=false should disable OCR"
        );
    }

    #[test]
    fn test_ocr_enabled_defaults_to_true() {
        let json = r#"{"ocr": {"backend": "tesseract"}}"#;
        let config: ExtractionConfig = serde_json::from_str(json).unwrap();
        assert!(!config.effective_disable_ocr(), "OCR should be enabled by default");
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_use_layout_for_markdown_defaults_to_false() {
        let config = ExtractionConfig::default();
        assert!(!config.use_layout_for_markdown);
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_use_layout_for_markdown_can_be_set_true() {
        let config = ExtractionConfig {
            use_layout_for_markdown: true,
            ..Default::default()
        };
        assert!(config.use_layout_for_markdown);
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_use_layout_for_markdown_serde_round_trip() {
        let config = ExtractionConfig {
            use_layout_for_markdown: true,
            ..Default::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ExtractionConfig = serde_json::from_str(&json).unwrap();
        assert!(deserialized.use_layout_for_markdown);
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn test_use_layout_for_markdown_serde_default_false() {
        let json = r#"{}"#;
        let config: ExtractionConfig = serde_json::from_str(json).unwrap();
        assert!(!config.use_layout_for_markdown);
    }

    #[test]
    fn test_default_extraction_timeout_is_six_hundred_seconds() {
        let config = ExtractionConfig::default();
        assert_eq!(
            config.extraction_timeout_secs,
            Some(600),
            "default timeout must be Some(600) to bound unbounded extraction while leaving headroom for slow (VLM) paths"
        );
    }

    #[test]
    fn test_extraction_timeout_can_be_disabled_by_setting_none() {
        let config = ExtractionConfig {
            extraction_timeout_secs: None,
            ..Default::default()
        };
        assert_eq!(config.extraction_timeout_secs, None);
    }

    #[test]
    fn test_extraction_timeout_serde_round_trip() {
        let config = ExtractionConfig {
            extraction_timeout_secs: Some(120),
            ..Default::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ExtractionConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.extraction_timeout_secs, Some(120));
    }

    #[test]
    fn test_extraction_timeout_serde_absent_field_defaults_to_six_hundred() {
        let json = r#"{}"#;
        let config: ExtractionConfig = serde_json::from_str(json).unwrap();
        assert_eq!(
            config.extraction_timeout_secs,
            Some(600),
            "absent field must use default_extraction_timeout() -> Some(600)"
        );
    }

    /// Embedded-image OCR reads the image bytes, so a configuration that runs it
    /// has to report that it needs them. A container extractor asks this question
    /// to decide whether to read the image out of its archive; when the answer was
    /// no, the image still reached the OCR path, with an empty buffer (GH#1662).
    #[test]
    fn a_config_that_ocrs_embedded_images_needs_the_image_data() {
        let config = ExtractionConfig {
            ocr: Some(OcrConfig::default()),
            ..Default::default()
        };
        assert!(
            config.needs_image_data(),
            "OCR of embedded images consumes the bytes, so they must be read"
        );
    }

    /// Turning embedded-image OCR off leaves the answer where it was, so a
    /// text-only extraction still skips reading images out of the container.
    #[test]
    fn a_config_that_does_not_ocr_embedded_images_still_skips_the_data() {
        let config = ExtractionConfig {
            ocr: Some(OcrConfig::default()),
            images: Some(ImageExtractionConfig {
                extract_images: false,
                run_ocr_on_images: false,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(!config.needs_image_data());
    }

    #[test]
    fn test_needs_image_data_includes_qr_codes() {
        let config = ExtractionConfig {
            qr_codes: Some(true),
            ..Default::default()
        };
        assert!(config.needs_image_data());

        let config = ExtractionConfig {
            qr_codes: Some(false),
            ..Default::default()
        };
        assert!(!config.needs_image_data());
    }

    #[test]
    fn test_with_file_overrides_applies_enrichment_fields() {
        let llm = LlmConfig {
            model: "test/model".to_string(),
            ..Default::default()
        };
        let url = super::UrlExtractionConfig {
            max_total_urls: Some(7),
            ..Default::default()
        };

        let overrides = FileExtractionConfig {
            url: Some(url),
            ner: Some(NerConfig::default()),
            redaction: Some(RedactionConfig::default()),
            summarization: Some(SummarizationConfig {
                max_tokens: Some(32),
                ..Default::default()
            }),
            translation: Some(TranslationConfig {
                target_lang: "de".to_string(),
                source_lang: Some("en".to_string()),
                preserve_markup: true,
                llm: llm.clone(),
            }),
            page_classification: Some(PageClassificationConfig {
                prompt_template: None,
                labels: vec!["invoice".to_string()],
                multi_label: false,
                llm: llm.clone(),
            }),
            captioning: Some(CaptioningConfig {
                llm,
                prompt: Some("caption".to_string()),
                min_image_area: 42,
                alt_text: Default::default(),
            }),
            qr_codes: Some(true),
            ..Default::default()
        };

        let resolved = ExtractionConfig::default().with_file_overrides(&overrides);
        assert_eq!(resolved.url.max_total_urls, Some(7));
        assert!(resolved.ner.is_some());
        assert!(resolved.redaction.is_some());
        assert_eq!(resolved.summarization.as_ref().and_then(|c| c.max_tokens), Some(32));
        assert_eq!(
            resolved
                .translation
                .as_ref()
                .map(|c| (c.target_lang.as_str(), c.preserve_markup)),
            Some(("de", true))
        );
        assert_eq!(
            resolved.page_classification.as_ref().map(|c| c.labels.as_slice()),
            Some(&["invoice".to_string()][..])
        );
        assert_eq!(
            resolved
                .captioning
                .as_ref()
                .map(|c| (c.prompt.as_deref(), c.min_image_area)),
            Some((Some("caption"), 42))
        );
        assert_eq!(resolved.qr_codes, Some(true));
    }

    #[cfg(feature = "html")]
    #[test]
    fn test_with_file_overrides_applies_html_output() {
        let overrides = FileExtractionConfig {
            html_output: Some(crate::core::config::html_output::HtmlOutputConfig {
                css: Some(".kb-p { color: red; }".to_string()),
                embed_css: false,
                ..Default::default()
            }),
            ..Default::default()
        };

        let resolved = ExtractionConfig::default().with_file_overrides(&overrides);
        let html_output = resolved.html_output.expect("html output override should apply");
        assert_eq!(html_output.css.as_deref(), Some(".kb-p { color: red; }"));
        assert!(!html_output.embed_css);
    }

    #[test]
    fn validate_accepts_a_single_ascii_char_csv_delimiter() {
        let config = ExtractionConfig {
            csv: Some(crate::core::config::CsvConfig {
                delimiter: Some(";".to_string()),
                comment_prefixes: vec![],
            }),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_rejects_an_empty_csv_delimiter_with_a_helpful_message() {
        let config = ExtractionConfig {
            csv: Some(crate::core::config::CsvConfig {
                delimiter: Some(String::new()),
                comment_prefixes: vec![],
            }),
            ..Default::default()
        };
        let err = config.validate().expect_err("empty delimiter must be rejected");
        assert_eq!(
            err.to_string(),
            "Validation error: Invalid CSV delimiter ''. Must be exactly one ASCII character (e.g. ',', ';', '\\t', '|')."
        );
    }

    /// GH#1752. The load-bearing property of all three new settings: leaving one unset must
    /// reproduce the derived answer bit-for-bit, for every shape of `ocr` block. Written as a
    /// comparison against the pre-#1752 expression rather than against hard-coded booleans so
    /// it fails if the *derivation* drifts, not merely if a literal was mistyped.
    #[test]
    fn ocr_behaviour_settings_left_unset_reproduce_the_derived_answer() {
        for ocr in [None, Some(OcrConfig::default())] {
            for images in [
                None,
                Some(ImageExtractionConfig::default()),
                Some(ImageExtractionConfig {
                    run_ocr_on_images: false,
                    ..Default::default()
                }),
            ] {
                let config = ExtractionConfig {
                    ocr: ocr.clone(),
                    images: images.clone(),
                    ..Default::default()
                };

                assert_eq!(
                    config.ocr_near_empty_fallback, None,
                    "ocr_near_empty_fallback must default to None"
                );
                assert_eq!(
                    config.ocr_scanned_page_quality_gate, None,
                    "ocr_scanned_page_quality_gate must default to None"
                );
                assert_eq!(
                    config.ocr_embedded_images, None,
                    "ocr_embedded_images must default to None"
                );

                let pre_1752 =
                    config.ocr.is_some() && config.images.as_ref().map(|i| i.run_ocr_on_images).unwrap_or(true);
                assert_eq!(
                    config.runs_ocr_on_embedded_images(),
                    pre_1752,
                    "with ocr_embedded_images unset the answer must stay `ocr.is_some() && run_ocr_on_images`, \
                     for ocr={:?} images={:?}",
                    config.ocr.is_some(),
                    config.images.as_ref().map(|i| i.run_ocr_on_images),
                );
            }
        }
    }

    /// `Some(_)` decides the OCR half outright, in both directions, and neither direction can
    /// override `run_ocr_on_images: false` -- the two settings are orthogonal, not a fallback
    /// chain.
    #[test]
    fn ocr_embedded_images_overrides_the_presence_of_an_ocr_block_in_both_directions() {
        let on_without_block = ExtractionConfig {
            ocr: None,
            ocr_embedded_images: Some(true),
            ..Default::default()
        };
        assert!(
            on_without_block.runs_ocr_on_embedded_images(),
            "Some(true) must recognise picture text with no `ocr` block"
        );
        assert!(
            on_without_block.needs_image_data(),
            "and the container must therefore still read the bytes (GH#1662)"
        );

        let off_with_block = ExtractionConfig {
            ocr: Some(OcrConfig::default()),
            ocr_embedded_images: Some(false),
            ..Default::default()
        };
        assert!(
            !off_with_block.runs_ocr_on_embedded_images(),
            "Some(false) must suppress embedded-image OCR even with an `ocr` block"
        );
        assert!(
            !off_with_block.needs_image_data(),
            "and nothing else is asking for the bytes, so they must not be read"
        );

        let on_but_images_opted_out = ExtractionConfig {
            ocr: None,
            ocr_embedded_images: Some(true),
            images: Some(ImageExtractionConfig {
                run_ocr_on_images: false,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(
            !on_but_images_opted_out.runs_ocr_on_embedded_images(),
            "`images.run_ocr_on_images: false` still wins: the two settings are independent"
        );
    }

    /// `disable_ocr: true` is documented as skipping OCR "for all document types" and must
    /// win over `ocr_embedded_images: Some(true)`, the same way it already wins over a plain
    /// `ocr` block elsewhere (`needs_image_processing`'s `ocr_enabled`, `extractors/image.rs`,
    /// `engine/extract_impl.rs`).
    #[test]
    fn disable_ocr_suppresses_embedded_image_ocr_even_when_opted_in() {
        let config = ExtractionConfig {
            ocr: None,
            ocr_embedded_images: Some(true),
            disable_ocr: true,
            ..Default::default()
        };
        assert!(
            !config.runs_ocr_on_embedded_images(),
            "disable_ocr must suppress embedded-image OCR even when explicitly opted in"
        );
        assert!(
            !config.needs_image_data(),
            "and the container must not pay for reading bytes only embedded-image OCR wanted"
        );
    }

    /// `wants_own_bytes_in_result` is deliberately narrower and must not start echoing a
    /// standalone image's bytes back just because the new opt-in turned embedded-image OCR on.
    #[test]
    fn ocr_embedded_images_does_not_widen_wants_own_bytes_in_result() {
        let config = ExtractionConfig {
            ocr: None,
            ocr_embedded_images: Some(true),
            ..Default::default()
        };
        assert!(config.runs_ocr_on_embedded_images());
        assert!(
            !config.wants_own_bytes_in_result(),
            "the narrower predicate must keep its three conditions (extract_images, captioning, qr_codes)"
        );
    }

    /// `deny_unknown_fields` is on, so a config file written before these fields existed must
    /// still deserialize -- and land on the derived behaviour.
    #[test]
    fn a_config_without_the_new_ocr_fields_deserializes_to_the_derived_behaviour() {
        let config: ExtractionConfig =
            serde_json::from_str(r#"{"ocr":{"backend":"tesseract"}}"#).expect("older config must still parse");
        assert_eq!(config.ocr_near_empty_fallback, None);
        assert_eq!(config.ocr_scanned_page_quality_gate, None);
        assert_eq!(config.ocr_embedded_images, None);
        assert!(config.runs_ocr_on_embedded_images());
    }

    /// All three round-trip through the wire format, so a binding can actually set them.
    #[test]
    fn the_new_ocr_fields_round_trip_through_json() {
        let config = ExtractionConfig {
            ocr_near_empty_fallback: Some(true),
            ocr_scanned_page_quality_gate: Some(false),
            ocr_embedded_images: Some(true),
            ..Default::default()
        };
        let json = serde_json::to_string(&config).expect("serialize");
        let round_tripped: ExtractionConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(round_tripped.ocr_near_empty_fallback, Some(true));
        assert_eq!(round_tripped.ocr_scanned_page_quality_gate, Some(false));
        assert_eq!(round_tripped.ocr_embedded_images, Some(true));
    }

    #[test]
    fn validate_rejects_a_multi_byte_csv_delimiter_with_a_helpful_message() {
        let config = ExtractionConfig {
            csv: Some(crate::core::config::CsvConfig {
                delimiter: Some("::".to_string()),
                comment_prefixes: vec![],
            }),
            ..Default::default()
        };
        let err = config
            .validate()
            .expect_err("multi-character delimiter must be rejected");
        assert_eq!(
            err.to_string(),
            "Validation error: Invalid CSV delimiter '::'. Must be exactly one ASCII character (e.g. ',', ';', '\\t', '|')."
        );
    }
}
