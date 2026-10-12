//! CLI extraction overrides via `#[derive(clap::Args)]`.
//!
//! Provides `ExtractionOverrides`, a flattened clap struct that captures all
//! optional CLI flags for extraction configuration. Call `validate()` then
//! `apply()` to layer these overrides onto an `ExtractionConfig`.
//!
//! The struct and its `validate`/`apply` entry points stay here; the field-by-field
//! validation and mutation logic lives in one submodule per CLI-flag domain (`ocr`,
//! `chunking`, `analysis`, `layout`, `pdf`, `html`, `output`, `general`, `llm`, `redaction`),
//! each adding its own `impl ExtractionOverrides` block for the methods it owns.
//!
//! `ExtractionOverrides` itself stays one flat struct rather than a composition of
//! per-domain sub-structs behind `#[command(flatten)]`. Splitting it would not change
//! any CLI flag name or `--help` text (clap flattens nested `#[command(flatten)]` structs
//! into the same flag namespace either way), but roughly 95 unit tests below construct
//! this struct with flat field-name struct-literal syntax (`ExtractionOverrides { ocr:
//! Some(true), .. }`); decomposing the struct would require rewriting every one of them
//! to route through the right sub-struct, a large, purely mechanical, and error-prone
//! diff for a lint-only benefit that is not worth the risk. ~keep

#![allow(deprecated)]

use anyhow::Result;
use xberg::ExtractionConfig;

#[cfg(feature = "analysis")]
pub use self::analysis::ReductionLevelArg;
pub use self::general::AccelerationArg;
pub use self::output::JupyterCellRenderingArg;
use crate::ContentOutputFormatArg;

mod analysis;
mod chunking;
mod general;
mod html;
mod layout;
mod llm;
mod ocr;
mod output;
mod pdf;
#[cfg(feature = "redaction")]
mod redaction;
#[cfg(test)]
mod tests;

use llm::{apply_llm_api_key, resolve_llm_api_key};

/// Optional CLI flags that override fields in `ExtractionConfig`.
///
/// Every field is `Option<T>` (or `Vec<T>` for repeatable flags) so that
/// only explicitly-provided flags take effect. Flatten this struct into any
/// clap command with `#[command(flatten)]`.
#[derive(Debug, Default, clap::Args)]
pub struct ExtractionOverrides {
    /// Enable or disable OCR. When true, configures an OCR backend
    /// (default: tesseract). When false, hard-disables OCR and removes its configuration.
    #[cfg(feature = "ocr-surface")]
    #[arg(long)]
    pub ocr: Option<bool>,

    /// OCR backend to use when --ocr is enabled (tesseract, paddle-ocr, sceptre, vlm, or candle-*).
    #[cfg(feature = "ocr-surface")]
    #[arg(long)]
    pub ocr_backend: Option<String>,

    /// OCR language code. Tesseract uses ISO 639-3 (eng, fra, deu).
    /// PaddleOCR uses short codes (en, ch, french, korean).
    #[cfg(feature = "ocr-surface")]
    #[arg(long)]
    pub ocr_language: Option<String>,

    /// Force OCR even if text extraction succeeds.
    #[cfg(feature = "ocr-surface")]
    #[arg(long)]
    pub force_ocr: Option<bool>,

    /// OCR pages that look like scans, keeping native text elsewhere.
    ///
    /// Detects pages that are full-page images, including scans whose hidden
    /// text layer would otherwise pass the default quality check.
    #[cfg(feature = "ocr-surface")]
    #[arg(long)]
    pub ocr_scanned_pages: bool,

    /// Minimum scan confidence (0.0-1.0) for --ocr-scanned-pages. Default: 0.7.
    ///
    /// A threshold of 0.50 or lower also OCRs born-digital slides that use a
    /// full-bleed background image.
    #[cfg(feature = "ocr-surface")]
    #[arg(long, requires = "ocr_scanned_pages")]
    pub scanned_min_confidence: Option<f64>,

    /// Disable OCR entirely (even for images)
    #[cfg(feature = "ocr-surface")]
    #[arg(long)]
    pub disable_ocr: Option<bool>,

    /// Disable extraction result caching.
    #[arg(long)]
    pub no_cache: Option<bool>,

    /// Enable automatic image rotation before OCR based on detected orientation.
    #[cfg(feature = "ocr-surface")]
    #[arg(long)]
    pub ocr_auto_rotate: Option<bool>,

    /// Bypass the on-disk OCR result cache for this run: neither read nor write it.
    ///
    /// Unlike `--no-cache` (which controls the whole-extraction-result cache), this
    /// only affects the Tesseract OCR cache keyed by image + engine config
    /// (`TesseractConfig::use_cache`, `crates/xberg/src/ocr/processor/execution.rs`).
    /// Use it to force a fresh OCR pass while iterating on OCR settings, without
    /// clearing the cache directory by hand.
    ///
    /// Only takes effect when `ocr.tesseract_config` is already set (e.g. by a loaded
    /// config file). When it is still unset, this flag currently has no effect and logs
    /// a warning instead of running: materialising `tesseract_config` here — even just to
    /// carry `use_cache: false` — would flip it from `None` to `Some(..)`, and several
    /// call sites in `crates/xberg/src/extractors/image.rs`
    /// (`apply_default_tesseract_psm`, `is_implicit_horizontal_tesseract`,
    /// `should_retry_sparse_image_ocr`) treat `tesseract_config.is_none()` as "no explicit
    /// Tesseract config yet" and use it to decide whether to install their own PSM/element
    /// defaults (e.g. `WHOLE_IMAGE_TESSERACT_PSM = 11` for whole-page image OCR, vs.
    /// `TesseractConfig::default()`'s `psm = 3`). Materialising a default-valued
    /// `TesseractConfig` here would silently disarm all of that and change what Tesseract
    /// actually recognises, which this flag's contract forbids (issue #693).
    #[cfg(feature = "ocr-surface")]
    #[arg(long)]
    pub ocr_no_cache: Option<bool>,

    /// JSON object of per-backend OCR options (e.g. `{"layout_mode":"whole_page"}`).
    #[cfg(feature = "ocr-surface")]
    #[arg(long, value_name = "JSON")]
    pub ocr_backend_options: Option<String>,

    /// VLM model for OCR (implies --ocr-backend vlm). Uses liter-llm routing format
    /// (e.g., "openai/gpt-4o", "anthropic/claude-sonnet-4-20250514").
    #[cfg(feature = "ocr-surface")]
    #[arg(long)]
    pub vlm_model: Option<String>,

    /// VLM API key for OCR
    #[cfg(feature = "ocr-surface")]
    #[arg(long)]
    pub vlm_api_key: Option<String>,

    /// Default LLM API key shared across every LLM-backed feature
    /// (VLM OCR, structured extraction, translation, classification, captioning,
    /// summarisation, NER). Lower precedence than `--vlm-api-key` and any
    /// `api_key` set in the loaded config file, higher precedence than the
    /// `XBERG_LLM_API_KEY` environment variable.
    #[arg(long, value_name = "KEY")]
    pub api_key: Option<String>,

    /// Custom VLM OCR prompt template (Jinja2)
    #[cfg(feature = "ocr-surface")]
    #[arg(long)]
    pub vlm_prompt: Option<String>,

    /// Enable or disable text chunking.
    #[cfg(any(feature = "core-cli", feature = "analysis"))]
    #[arg(long)]
    pub chunk: Option<bool>,

    /// Maximum chunk size in characters.
    #[cfg(any(feature = "core-cli", feature = "analysis"))]
    #[arg(long)]
    pub chunk_size: Option<usize>,

    /// Overlap between consecutive chunks in characters.
    #[cfg(any(feature = "core-cli", feature = "analysis"))]
    #[arg(long)]
    pub chunk_overlap: Option<usize>,

    /// Tokenizer model for token-based chunk sizing (e.g. "Xenova/gpt-4o").
    /// Implicitly enables chunking. Requires the chunking-tokenizers feature.
    #[cfg(any(feature = "core-cli", feature = "analysis"))]
    #[arg(long)]
    pub chunking_tokenizer: Option<String>,

    /// Content rendering format (plain, markdown, djot, html).
    /// Controls the format of extracted content.
    #[arg(long, value_enum)]
    pub content_format: Option<ContentOutputFormatArg>,

    /// Content rendering format (DEPRECATED: use --content-format instead).
    #[arg(long, value_enum, hide = true)]
    pub output_format: Option<ContentOutputFormatArg>,

    /// Include hierarchical document structure in results.
    #[arg(long)]
    pub include_structure: Option<bool>,

    /// Fail on unparseable YAML/JSON/TOML/XML and on table-cell or XML-entity limit hits instead of
    /// falling back to plain text with a warning.
    #[arg(long)]
    pub strict_parse: Option<bool>,

    /// For Jupyter notebooks: render code cells as source, outputs, or both (default: both).
    /// Cells are never executed — outputs come only from those saved in the notebook.
    #[arg(long, value_enum)]
    pub jupyter_cell_rendering: Option<JupyterCellRenderingArg>,

    /// Enable quality post-processing.
    #[cfg(feature = "analysis")]
    #[arg(long)]
    pub quality: Option<bool>,

    /// Enable language detection on extracted text.
    #[cfg(feature = "analysis")]
    #[arg(long)]
    pub detect_language: Option<bool>,

    /// Enable layout detection with default model settings (RT-DETR v2).
    /// Use `--layout` to enable or `--layout false` to explicitly disable.
    #[cfg(feature = "layout-detection")]
    #[arg(long, default_missing_value = "true", num_args = 0..=1)]
    pub layout: Option<bool>,

    /// Layout detection confidence threshold (0.0 - 1.0).
    #[cfg(feature = "layout-detection")]
    #[arg(long)]
    pub layout_confidence: Option<f32>,

    /// Which pages the layout model runs on: always (default, every page) or
    /// auto (pre-screen each page and skip the model where it cannot help).
    #[cfg(feature = "layout-detection")]
    #[arg(
        long,
        help = "Layout page selection: always (default, every page) or auto (pre-screen pages)"
    )]
    pub layout_strategy: Option<String>,

    /// Table structure model: tatr (default), slanet_wired, slanet_wireless, slanet_plus, slanet_auto, disabled.
    #[cfg(feature = "layout-detection")]
    #[arg(
        long,
        help = "Table structure model: tatr (default), slanet_wired, slanet_wireless, slanet_plus, slanet_auto, disabled"
    )]
    pub layout_table_model: Option<String>,

    /// Formula recognition model for layout-detected formula regions: latex_ocr.
    #[cfg(feature = "formula-recognition")]
    #[arg(
        long,
        help = "Formula recognition model for layout-detected formula regions: latex_ocr"
    )]
    pub layout_formula_model: Option<String>,

    /// Feed layout detection regions into the non-OCR markdown pipeline to improve
    /// heading/table/list/figure structure. Requires `--layout` to be enabled.
    /// Default: false.
    #[cfg(feature = "layout-detection")]
    #[arg(long)]
    pub use_layout_for_markdown: bool,

    /// ONNX Runtime execution provider for model inference.
    #[arg(long, value_enum)]
    pub acceleration: Option<AccelerationArg>,

    /// Maximum number of concurrent extractions in batch mode.
    #[arg(long, help = "Limit parallel extractions in batch mode")]
    pub max_concurrent: Option<usize>,

    /// Cap all internal thread pools (Rayon, ONNX intra-op, batch semaphore).
    #[arg(long, help = "Limit total threads for constrained environments")]
    pub max_threads: Option<usize>,

    /// Set concurrent Tesseract recognition sessions directly. The value is applied as given and is not capped by the
    /// thread budget. The first extraction in a process fixes it for that process.
    #[arg(
        long,
        help = "Set concurrent OCR sessions directly, instead of following the thread budget"
    )]
    pub max_concurrent_ocr: Option<usize>,

    /// Extract pages as a separate array in results.
    #[arg(long)]
    pub extract_pages: Option<bool>,

    /// Insert page marker comments into the main content string.
    #[arg(long)]
    pub page_markers: Option<bool>,

    /// Enable image extraction from documents.
    #[arg(long)]
    pub extract_images: Option<bool>,

    /// Target DPI for image normalisation (e.g. 150, 300, 600).
    #[arg(long)]
    pub target_dpi: Option<i32>,

    /// Password(s) for encrypted PDFs. Can be specified multiple times.
    #[cfg(feature = "pdf-surface")]
    #[arg(long)]
    pub pdf_password: Vec<String>,

    /// Extract images embedded in PDF pages.
    #[cfg(feature = "pdf-surface")]
    #[arg(long)]
    pub pdf_extract_images: Option<bool>,

    /// Extract tables from PDF (native engine grid + heuristic text-layer fallback).
    /// Default: true.
    #[cfg(feature = "pdf-surface")]
    #[arg(long)]
    pub pdf_extract_tables: Option<bool>,

    /// OCR extracted inline images and inject results into the document.
    #[cfg(all(feature = "pdf-surface", feature = "ocr-surface"))]
    #[arg(long)]
    pub pdf_ocr_inline_images: Option<bool>,

    /// Extract PDF metadata (title, author, etc.).
    #[cfg(feature = "pdf-surface")]
    #[arg(long)]
    pub pdf_extract_metadata: Option<bool>,

    /// PDF extraction backend to use: "native" (default) or "pdfium".
    ///
    /// "pdfium" requires the CLI to be built with the `pdf-pdfium-surface`
    /// feature, and requires the pdfium shared library to be loadable at run
    /// time (system library search path, or `PDFIUM_DYNAMIC_LIB_PATH`).
    /// Selecting it on a build without that feature is rejected with an error
    /// rather than silently falling back to native.
    ///
    /// The pdfium engine is deliberately narrower than native: page count,
    /// per-page plain text, and Info-dictionary metadata only -- no tables,
    /// layout detection, annotations, form fields, embedded files, or OCR
    /// fallback. It is not a drop-in replacement for the native backend.
    #[cfg(feature = "pdf-surface")]
    #[arg(long, value_name = "BACKEND")]
    pub pdf_backend: Option<String>,

    /// Token reduction level (off, light, moderate, aggressive, maximum).
    #[cfg(feature = "analysis")]
    #[arg(long, value_enum)]
    pub token_reduction: Option<ReductionLevelArg>,

    /// Windows codepage fallback for MSG files without codepage metadata.
    /// Common values: 1250 (Central European), 1251 (Cyrillic), 1252 (Western).
    #[arg(long)]
    pub msg_codepage: Option<u32>,

    /// Cache namespace for tenant isolation.
    #[arg(long)]
    pub cache_namespace: Option<String>,

    /// Per-request cache TTL in seconds (0 = skip cache).
    #[arg(long)]
    pub cache_ttl_secs: Option<u64>,

    /// Built-in colour theme for styled HTML output (default, github, dark, light, unstyled).
    /// Implies --content-format html and enables the styled HTML renderer.
    #[cfg(feature = "html")]
    #[arg(long, value_name = "THEME")]
    pub html_theme: Option<String>,

    /// Inline CSS string appended after the theme stylesheet in styled HTML output.
    #[cfg(feature = "html")]
    #[arg(long, value_name = "CSS")]
    pub html_css: Option<String>,

    /// Path to a CSS file loaded once and appended after the theme stylesheet in styled HTML output.
    #[cfg(feature = "html")]
    #[arg(long, value_name = "PATH")]
    pub html_css_file: Option<std::path::PathBuf>,

    /// CSS class prefix used on every emitted class name (default: "kb-").
    #[cfg(feature = "html")]
    #[arg(long, value_name = "PREFIX")]
    pub html_class_prefix: Option<String>,

    /// Suppress the embedded `<style>` block in styled HTML output.
    #[cfg(feature = "html")]
    #[arg(long)]
    pub html_no_embed_css: bool,

    /// CSV/TSV field delimiter (single ASCII character, e.g. ";", "|", "\t").
    /// When unset, the delimiter is auto-detected from the file.
    #[arg(long, value_name = "CHAR")]
    pub csv_delimiter: Option<String>,

    /// Line prefix marking a CSV/TSV comment line to skip entirely (e.g. "#").
    /// Can be specified multiple times. Default: no comment filtering.
    #[arg(long, value_name = "PREFIX")]
    pub csv_comment_prefix: Vec<String>,

    /// Redact the findings an external content-inspection engine (Presidio, AWS
    /// Comprehend) reported over this document's text. Takes a JSON array or JSON
    /// Lines file, or `-` to read them from stdin. Enables redaction if the config
    /// does not already.
    #[cfg(feature = "redaction")]
    #[arg(long, value_name = "PATH|-")]
    pub redaction_findings: Option<std::path::PathBuf>,

    /// Findings read from stdin by `read_redaction_findings_stdin`.
    #[cfg(feature = "redaction")]
    #[arg(skip)]
    pub(crate) redaction_findings_stdin: Option<Vec<xberg::core::config::redaction::ExternalRedactionFinding>>,
}

impl ExtractionOverrides {
    /// Validate flag combinations before applying.
    ///
    /// Call this before `apply()` to surface user-friendly errors for
    /// invalid or contradictory options. Each domain's checks live in its own
    /// `validate_*` method (see the submodules declared above); this just calls
    /// them in the same order the checks used to run inline.
    pub fn validate(&self) -> Result<()> {
        #[cfg(any(feature = "core-cli", feature = "analysis"))]
        self.validate_chunking()?;
        self.validate_target_dpi()?;
        #[cfg(feature = "layout-detection")]
        self.validate_layout()?;
        #[cfg(feature = "ocr-surface")]
        self.validate_ocr()?;
        #[cfg(all(
            any(feature = "core-cli", feature = "analysis"),
            not(feature = "chunking-tokenizers")
        ))]
        self.validate_chunking_tokenizer_feature()?;
        self.validate_concurrency()?;
        #[cfg(feature = "pdf-surface")]
        self.validate_pdf_backend()?;
        self.validate_csv()?;
        Ok(())
    }

    /// Apply these overrides onto an existing `ExtractionConfig`.
    ///
    /// Only fields that were explicitly provided on the command line take
    /// effect; everything else is left untouched.
    pub fn apply(self, config: &mut ExtractionConfig) {
        let resolved_api_key = resolve_llm_api_key(self.api_key.as_deref());
        #[cfg(feature = "ocr-surface")]
        self.apply_ocr(config);
        #[cfg(feature = "ocr-surface")]
        self.apply_vlm_ocr(config);
        #[cfg(any(feature = "core-cli", feature = "analysis"))]
        self.apply_chunking(config);
        #[cfg(feature = "analysis")]
        self.apply_quality_and_detection(config);
        self.apply_output_format(config);
        self.apply_include_structure(config);
        self.apply_strict_parse(config);
        self.apply_jupyter_cell_rendering(config);
        self.apply_layout(config);
        self.apply_acceleration(config);
        self.apply_concurrency(config);
        self.apply_pages(config);
        self.apply_images(config);
        #[cfg(feature = "pdf-surface")]
        self.apply_pdf(config);
        #[cfg(feature = "analysis")]
        self.apply_token_reduction(config);
        self.apply_email(config);
        self.apply_cache(config);
        self.apply_html_styled(config);
        self.apply_csv(config);
        #[cfg(feature = "redaction")]
        self.apply_redaction(config);
        if let Some(key) = resolved_api_key {
            apply_llm_api_key(config, &key);
        }
        // Last: every prior `apply_*` that can touch `config.layout` (`apply_layout`) or
        // `config.output_format` (`apply_output_format`, `apply_html_styled`) has already run,
        // so this observes the fully resolved combination rather than an intermediate one.
        #[cfg(feature = "layout-detection")]
        self.warn_layout_wastes_plain_output(config);
    }
}
