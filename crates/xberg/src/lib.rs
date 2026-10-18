//! Xberg - High-Performance Document Intelligence Library
//!
//! Xberg is a Rust-first document extraction library with language-agnostic plugin support.
//! It provides fast, accurate extraction from PDFs, images, Office documents, emails, and more.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use xberg::{extract, ExtractInput, ExtractionConfig};
//!
//! # async fn run() -> xberg::Result<()> {
//! let config = ExtractionConfig::default();
//! let output = extract(ExtractInput::from_uri("document.pdf"), &config).await?;
//! println!("Extracted: {}", output.results[0].content);
//! # Ok(())
//! # }
//! ```
//!
//! # Architecture
//!
//! - **Core Module** (`core`): Main extraction orchestration, MIME detection, config loading
//! - **Plugin System**: Language-agnostic plugin architecture
//! - **Extractors**: Format-specific extraction (PDF, images, Office docs, email, etc.)
//! - **OCR**: Multiple OCR backend support (Tesseract, PaddleOCR, VLM)
//!
//! # Features
//!
//! - Fast parallel processing with async/await
//! - Priority-based extractor selection
//! - Comprehensive MIME type detection (141 file extensions)
//! - Configurable caching and quality processing
//! - Cross-language plugin support (Python, Node.js planned)

#![deny(clippy::print_stdout, clippy::print_stderr)]
#![cfg_attr(test, allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro))]
// Deprecated public compatibility fields remain wired internally until their planned 2.0 removal. ~keep
#![allow(deprecated)]
#![deny(unsafe_code)]

/// Return the source build identifier embedded in this Xberg library.
///
/// This is intended for executable provenance checks, not as a stable content
/// or cache identifier.
#[doc(hidden)]
#[cfg_attr(alef, alef(skip))]
#[must_use]
pub const fn embedded_build_id() -> &'static str {
    env!("XBERG_BUILD_ID")
}

#[cfg(all(
    feature = "sceptre-ocr-ort",
    any(target_arch = "wasm32", target_os = "android", target_os = "ios")
))]
compile_error!("`sceptre-ocr-ort` supports desktop and server targets only; use `sceptre-ocr-tract` on mobile");

#[cfg(all(
    feature = "paddle-ocr-ort",
    any(target_arch = "wasm32", target_os = "android", target_os = "ios")
))]
compile_error!("`paddle-ocr-ort` supports desktop and server targets only; use `paddle-ocr-tract` on mobile");

pub mod cache;
pub(crate) mod cache_dir;
pub mod cancellation;
pub mod core;
pub mod engine;
pub mod error;
/// Format-specific document extraction implementations and office metadata types.
pub mod extraction;
pub mod extractors;
#[cfg(all(
    feature = "layout-detection",
    any(feature = "pdf", feature = "ocr", feature = "ocr-wasm")
))]
pub mod model_cache;
pub mod plugins;
pub mod rendering;
pub mod telemetry;
/// Text post-processing: NER, summarisation, redaction, token reduction, and translation.
pub mod text;
pub mod types;
pub mod utils;

/// Pure-arithmetic sub/superscript ("script run") decision rule, shared by PDF prose assembly
/// and native table cells.
///
/// Gated on `pdf` alone, NOT on `table_core`'s wider `any(ocr, pdf, paddle_ocr)`: both consumers
/// (`pdf::structure::pipeline`, `pdf::native::table`) are `pdf`-only, so the wider gate left every
/// item here dead on an `ocr`-without-`pdf` leg -- which CI builds with `-D warnings`. ~keep
#[cfg(feature = "pdf")]
pub(crate) mod script_run;

#[cfg(any(feature = "ocr", feature = "pdf", paddle_ocr))]
pub mod table_core;

#[cfg(feature = "tower-service")]
pub mod service;

#[cfg(feature = "api")]
pub mod api;

#[cfg(feature = "mcp")]
pub mod mcp;

#[cfg(feature = "chunking")]
pub mod chunking;

#[cfg(feature = "diff")]
pub mod diff;

#[cfg(all(feature = "liter-llm", not(target_arch = "wasm32")))]
pub mod llm;

#[cfg(feature = "embedding-presets")]
pub mod embeddings;

#[cfg(any(feature = "reranker-presets", feature = "reranker"))]
pub mod reranking;

/// Shared ONNX Runtime model-loading helpers (download, tokenizer, session).
#[cfg(feature = "onnx-runtime")]
pub(crate) mod onnx;

#[cfg(feature = "onnx-runtime")]
#[cfg_attr(alef, alef(skip))]
pub use onnx::{OrtSessionOptions, ort_session_options, set_ort_session_options};

/// Sparse (SPLADE) learned embeddings for hybrid dense+sparse retrieval.
#[cfg(any(feature = "sparse-embedding-presets", feature = "sparse-embeddings"))]
pub mod sparse_embeddings;

/// ColBERT late-interaction (multi-vector) embeddings for MaxSim retrieval.
#[cfg(any(feature = "late-interaction-presets", feature = "late-interaction"))]
pub mod late_interaction;

/// Process-wide caches of loaded model engines.
#[cfg(any(
    feature = "embeddings",
    feature = "static-embeddings",
    feature = "reranker",
    feature = "sparse-embeddings",
    feature = "late-interaction",
    feature = "candle-trocr",
    feature = "candle-paddleocr-vl",
    feature = "candle-glm-ocr",
    feature = "candle-deepseek-ocr"
))]
mod engine_cache;

// `layout-detection` renders PDF pages itself and needs `image::dpi` to honour a configured
// render DPI (#1577); it does not imply `ocr-pipeline`, so the module gate has to cover both or
// `pdf + layout-detection` fails to compile. The submodules that genuinely need the OCR
// dependency set stay gated inside `image/mod.rs`. ~keep
#[cfg(any(feature = "ocr-pipeline", feature = "layout-detection"))]
/// Image preprocessing and DPI utilities for OCR and layout pipelines.
pub mod image;

#[cfg(feature = "language-detection")]
pub mod language_detection;

#[cfg(feature = "stopwords")]
pub mod stopwords;

#[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
pub mod keywords;

#[cfg(feature = "enrichment")]
pub mod enrichment;

#[cfg(feature = "heuristics")]
pub mod heuristics;

#[cfg(feature = "heuristics")]
pub use heuristics::{
    BoundaryReason, ChunkInfo, ChunkPlan, ChunkingDecision, ChunkingReason, ConfidenceSignals, ConfidenceWeights,
    DocumentBoundary, DocumentMetadata, ExtractionConfidence, HeuristicsConfig, HeuristicsError, MultidocInput,
    MultidocThresholds, NoChunkingReason, PageRange, PageSignals, SchemaCompliance, StructuredCallMode,
    StructuredInput, StructuredThresholds, UserChunkConfig, analyze_document, analyze_with_user_chunks,
    boundaries_from_extraction_result, calculate_chunk_plan, calculate_plan_from_overrides, check_format_limits,
    choose_call_mode, detect_boundaries, score_confidence,
};

#[cfg(feature = "presets")]
pub mod presets;

#[cfg(any(feature = "ocr", feature = "ocr-wasm"))]
pub mod ocr;

/// Canonical OCR metadata key names, shared across every OCR-producing and
/// OCR-consuming feature domain. Deliberately ungated — see the module docs.
pub(crate) mod ocr_metadata_keys;

pub mod doctor;

#[cfg(any(
    paddle_ocr,
    feature = "embeddings",
    feature = "reranker",
    feature = "onnx-runtime",
    feature = "layout-detection",
    feature = "auto-rotate",
    feature = "transcription",
    feature = "ner-onnx"
))]
pub mod ort_discovery;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod model_download;

/// Engine-neutral inference seam (issue #1275): backend/session traits over ONNX
/// Runtime on native builds and the pure-Rust `tract` engine on no-ORT targets
/// (Android x86_64; WASM once embedded-weight loading lands). `auto_rotate` covers
/// both the ORT `auto-rotate` and the tract `auto-rotate-tract` variants; `layout_detection`
/// covers both the ORT `layout-detection` and the tract `layout-tract` variants.
#[cfg(any(layout_detection, auto_rotate))]
pub(crate) mod inference;

pub mod paddle_ocr;

#[cfg(all(paddle_ocr, sceptre_ocr, not(target_arch = "wasm32")))]
#[cfg_attr(alef, alef(skip))]
pub mod classic_ocr_cache;

#[cfg(all(sceptre_ocr, not(target_arch = "wasm32")))]
pub mod sceptre_ocr;

#[cfg(any(sceptre_ocr, feature = "sceptre-wasm"))]
mod sceptre_languages;

#[cfg(feature = "sceptre-wasm")]
pub mod sceptre_wasm;

#[cfg(feature = "candle-ocr")]
pub mod candle_ocr;

#[cfg(feature = "auto-rotate-types")]
pub mod doc_orientation;

#[cfg(feature = "layout-types")]
pub mod layout;

/// LaTeX recognition for rasterized formula regions (RapidLaTeXOCR ONNX).
#[cfg(feature = "formula-recognition")]
pub mod formula_recognition;

#[cfg(feature = "pdf")]
pub mod pdf;

#[cfg(feature = "transcription")]
pub mod transcription;

#[cfg(feature = "captioning")]
pub mod captioning;

// NOTE: `CancellationToken` is intentionally NOT re-exported here.
pub use error::{Result, XbergError};
pub use types::*;

// root (`#[frb(mirror(CoreProperties))]` → `xberg::CoreProperties`).
#[cfg(feature = "office")]
pub use extraction::office_metadata::{CoreProperties, DocxAppProperties};

#[cfg(feature = "redaction")]
pub use core::extract::extract_with_external_redaction;
#[cfg(feature = "url-ingestion")]
pub use core::extract::map_url;
pub use core::extract::{extract, extract_batch};
#[cfg(feature = "pdf")]
pub use core::split::{SplitConfig, SplitSegment, SplitStrategy, split_and_extract};
#[cfg(feature = "redaction")]
pub use text::redaction::redact_external;

pub use core::config::{
    AccelerationConfig, BedrockConfig, CallMode, CaptionAltTextMode, CaptioningConfig, ChunkClassificationConfig,
    ChunkClassificationDefinition, ChunkSizing, ChunkerType, ChunkingConfig, ConcurrencyConfig, ContentFilterConfig,
    CredentialProviderConfig, CsvConfig, EmailConfig, EmbeddingConfig, EmbeddingModelType, ExecutionProviderType,
    ExternalRedactionFinding, ExtractInput, ExtractInputKind, ExtractionConfig, ExtractionErrorItem, ExtractionResult,
    ExtractionSummary, FileExtractionConfig, GeoJsonExtractionConfig, ImageExtractionConfig, JupyterCellRendering,
    LanguageDetectionConfig, LlmBudgetConfig, LlmCacheConfig, LlmConfig, LlmProviderConfig, LlmRateLimitConfig,
    MergeMode, MimeDetectionPolicy, NerBackendKind, NerConfig, OcrConfig, OutputFormat, PageClassificationConfig,
    PageConfig, PostProcessorConfig, RedactionConfig, RedactionOffsetEncoding, RedactionPattern, RedactionTerm,
    RerankerConfig, RerankerHead, RerankerModelType, StructuredExtractionConfig, SummarizationConfig,
    TableChunkingMode, TokenReductionOptions, TranslationConfig, UrlExtractionConfig, UrlExtractionMode,
};
pub use core::config::{
    LateInteractionConfig, LateInteractionModelType, SparseEmbeddingConfig, SparseEmbeddingModelType,
};
#[cfg(feature = "transcription-types")]
pub use core::config::{TranscriptionConfig, WhisperModel};
#[cfg(any(feature = "url-ingestion", feature = "url-config-types"))]
pub use crawlberg::{
    AssetCategory, AuthConfig, BrowserBackend, BrowserConfig, BrowserMode, BrowserWait, ContentConfig, CrawlConfig,
    ProxyConfig, SsrfPolicy,
};
#[cfg(feature = "url-ingestion")]
pub use crawlberg::{MapResult, SitemapUrl};
pub use extractors::security::SecurityLimits;

#[cfg(feature = "presets")]
pub use presets::{
    LoadError, MetaSchema, Preset, PresetCategory, PresetSample, PresetSummary, Registry, ResolveError, ResolvedPreset,
    resolve,
};

#[cfg(feature = "quality")]
pub use text::{ReductionLevel, TokenReductionConfig};

#[cfg(all(
    feature = "ner-llm",
    not(target_arch = "wasm32"),
    not(all(target_os = "android", target_arch = "x86_64"))
))]
#[cfg_attr(alef, alef(skip))]
pub use text::ner::llm::LlmBackend;

#[cfg(feature = "ner")]
pub use text::ner::NerBackend;

#[cfg(feature = "ner-onnx")]
pub use text::ner::gline::GlineBackend;

#[cfg(feature = "ner")]
#[cfg_attr(alef, alef(skip))]
pub use text::ner::detect_entities;

#[cfg(feature = "classification")]
#[cfg_attr(alef, alef(skip))]
pub use text::classification::classify_document;

#[cfg(feature = "redaction")]
pub use text::redaction::strategy::TokenCounter;

#[cfg(feature = "api-types")]
pub use core::server_config::ServerConfig;

#[cfg(feature = "pdf")]
pub use core::config::{HierarchyConfig, PdfBackend, PdfConfig};

#[cfg(feature = "html")]
pub use core::config::{HtmlOutputConfig, HtmlTheme};
// `ExtractionConfig::html_options` and `FileConfig::html_options` are public fields of this
// external type, so callers already have to name it; without this re-export they must take a
// direct `html-to-markdown-rs` dependency and keep its version in lockstep with ours. It is
// also what the generated bindings resolve against — the wasm serde mirror emits
// `xberg::ConversionOptions`, which is the same path every sibling config type uses. ~keep
#[cfg(feature = "html")]
pub use html_to_markdown_rs::ConversionOptions;
#[cfg(feature = "html")]
pub use rendering::StyledHtmlRenderer;

pub use paddle_ocr::{ModelPaths, PaddleInferenceBackend, PaddleLanguage, PaddleOcrConfig};

#[cfg(paddle_ocr)]
pub use paddle_ocr::{ModelCacheStats, ModelManager, ModelManifestEntry, PaddleOcrBackend};

/// The layout model manager's local manifest-entry mirror, promoted to the
/// same public name the paddle build re-exports, so `manifest()` surfaces
/// stay public in every feature combination.
#[cfg(all(feature = "layout-detection", not(paddle_ocr)))]
pub use layout::model_manager::ModelManifestEntry;

pub use cache::CacheStats;

#[cfg(feature = "layout-types")]
pub use core::config::{LayoutDetectionConfig, LayoutStrategy, TableModel};

#[cfg(feature = "layout-types")]
pub use layout::types::{BBox, DetectionResult, LayoutClass, LayoutDetection};

#[cfg(feature = "layout-types")]
pub use layout::types::RecognizedTable;
#[cfg(any(feature = "ocr", feature = "ocr-wasm"))]
pub use ocr::types::PSMMode;

pub use core::config::{OcrPipelineConfig, OcrPipelineStage, OcrQualityThresholds, OcrStrategy, VlmFallbackPolicy};

#[cfg(feature = "auto-rotate-types")]
pub use doc_orientation::OrientationResult;

#[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
pub use keywords::{Keyword, KeywordAlgorithm, KeywordConfig, NgramRange};

#[cfg(feature = "keywords-rake")]
pub use keywords::RakeParams;

#[cfg(feature = "keywords-yake")]
pub use keywords::YakeParams;

#[cfg(feature = "markdown-footnotes")]
pub use text::markdown_footnotes::{
    Citation, FootnoteAnchor, FootnoteConfig, FootnoteDefinition, find_footnote_anchors, find_inference_markers,
    find_unmarked_claims, parse_citations, parse_footnote_definitions, verify_excerpt,
};

#[cfg(feature = "diff")]
pub use diff::{DiffHunk, DiffOptions, EmbeddedChanges, EmbeddedDiff, ExtractionDiff, TableDiff, compare};

#[cfg(feature = "tree-sitter")]
pub use core::config::{CodeContentMode, TreeSitterConfig, TreeSitterProcessConfig};
#[cfg(feature = "tree-sitter")]
pub use tree_sitter_language_pack::{
    CommentKind, DiagnosticSeverity, ExportKind, FileMetrics, ProcessConfig, StructureKind,
};

pub use core::mime::{
    SUPPORTED_EXTENSION_COUNT, SUPPORTED_FORMAT_COUNT, SupportedFormat, detect_mime_type_from_bytes,
    get_extensions_for_mime, list_supported_formats,
};

/// Detect the MIME type of a file at the given path.
///
/// Detection uses the path's extension and does not read the file. Set `check_exists`
/// to verify that the path exists first. To inspect document content directly, use
/// [`detect_mime_type_from_bytes`].
///
/// # Errors
///
/// Returns an I/O error when `check_exists` is `true` and the path does not exist,
/// or an unsupported-format or validation error when the extension cannot be resolved.
pub fn detect_mime_type(path: String, check_exists: bool) -> crate::Result<String> {
    core::mime::detect_mime_type(path, check_exists)
}

#[cfg(feature = "pdf")]
pub use pdf::render::{pdf_page_count, render_pdf_page_to_png};

pub use doctor::{DoctorCheck, DoctorReport, ProbeStatus, doctor};

#[cfg_attr(alef, alef(skip))]
pub use plugins::{
    clear_document_extractors, clear_embedding_backends, clear_ocr_backends, clear_post_processors, clear_renderers,
    clear_reranker_backends, clear_tokenizer_backends, clear_validators, list_document_extractors,
    list_embedding_backends, list_ocr_backend_capabilities, list_ocr_backend_capabilities_for, list_ocr_backends,
    list_post_processors, list_renderers, list_reranker_backends, list_tokenizer_backends, list_validators,
    ocr_backend_supports_language, ocr_backend_supports_language_for, register_document_extractor,
    register_embedding_backend, register_ocr_backend, register_post_processor, register_renderer,
    register_reranker_backend, register_tokenizer_backend, register_validator, unregister_document_extractor,
    unregister_embedding_backend, unregister_ocr_backend, unregister_post_processor, unregister_renderer,
    unregister_reranker_backend, unregister_tokenizer_backend, unregister_validator,
};

#[cfg_attr(alef, alef(skip))]
pub use plugins::{
    ConfidenceSemantics, DocumentExtractor, EmbeddingBackend, InternalDocumentExtractor, OcrBackend,
    OcrBackendCapabilities, OcrBackendType, PageOrientationHandling, Plugin, PostProcessor, ProcessingStage, Renderer,
    RerankerBackend, TokenizerBackend, Validator,
};

#[cfg(feature = "embedding-presets")]
pub use embeddings::EmbeddingPreset;

/// Embed a list of texts using the configured embedding model.
///
/// Returns a 2D vector where each inner vector is the embedding for the corresponding text.
#[cfg(any(feature = "embeddings", feature = "static-embeddings"))]
#[cfg_attr(alef, alef(skip))]
pub fn embed_texts(texts: Vec<String>, config: &core::config::EmbeddingConfig) -> crate::Result<Vec<Vec<f32>>> {
    embeddings::embed_texts(&texts, config)
}

#[cfg(all(
    feature = "tokio-runtime",
    any(feature = "embeddings", feature = "static-embeddings")
))]
#[cfg_attr(alef, alef(skip))]
pub use embeddings::embed_texts_async;

/// Drop every resident model engine: dense, static, sparse and late-interaction
/// embeddings and rerankers. Returns the number of engines removed.
///
/// A caller that still holds an engine keeps it alive until it drops its
/// handle. The next call that needs a model loads it again. A build without
/// any engine feature has nothing to drop and returns 0.
#[cfg_attr(alef, alef(skip))]
pub fn clear_engine_caches() -> usize {
    let removed = 0;
    #[cfg(any(feature = "embeddings", feature = "static-embeddings"))]
    let removed = removed + embeddings::clear_engine_cache();
    #[cfg(feature = "reranker")]
    let removed = removed + reranking::clear_engine_cache();
    #[cfg(feature = "sparse-embeddings")]
    let removed = removed + sparse_embeddings::clear_engine_cache();
    #[cfg(feature = "late-interaction")]
    let removed = removed + late_interaction::clear_engine_cache();
    removed
}

/// Drop only engines whose sessions use the shared ONNX Runtime session options. ~keep
#[cfg(feature = "onnx-runtime")]
pub(crate) fn clear_onnx_engine_caches() -> usize {
    let removed = 0;
    #[cfg(feature = "embeddings")]
    let removed = removed + embeddings::clear_onnx_engine_cache();
    #[cfg(feature = "reranker")]
    let removed = removed + reranking::clear_engine_cache();
    #[cfg(feature = "sparse-embeddings")]
    let removed = removed + sparse_embeddings::clear_engine_cache();
    #[cfg(feature = "late-interaction")]
    let removed = removed + late_interaction::clear_engine_cache();
    removed
}

/// Get an embedding preset by name.
///
/// Returns `None` if no preset with the given name exists. Returns an owned
/// clone so the value is safe to pass across FFI boundaries.
#[cfg(feature = "embedding-presets")]
#[cfg_attr(alef, alef(skip))]
pub fn get_embedding_preset(name: &str) -> Option<embeddings::EmbeddingPreset> {
    embeddings::get_preset(name)
}

/// List the names of all available embedding presets.
///
/// Returns owned `String`s so the values are safe to pass across FFI boundaries.
#[cfg(feature = "embedding-presets")]
#[cfg_attr(alef, alef(skip))]
pub fn list_embedding_presets() -> Vec<String> {
    embeddings::list_presets()
}

/// Query-side instruction prefix for an embedding config, if its preset defines
/// one (asymmetric retrieval models such as Arctic-Embed). The RAG query path
/// prepends this to query text; document text is embedded verbatim. Returns
/// `None` for symmetric presets, custom models, and non-preset backends.
#[cfg(feature = "embedding-presets")]
#[cfg_attr(alef, alef(skip))]
pub fn embedding_query_prefix(config: &EmbeddingConfig) -> Option<String> {
    embeddings::embedding_query_prefix(config)
}

/// Re-export `RerankerPreset` when the `reranker-presets` feature is active.
///
#[cfg(feature = "reranker-presets")]
pub use reranking::RerankerPreset;

/// Re-export `RerankedDocument` — needed for stub signatures and result types.
///
#[cfg(any(feature = "reranker-presets", feature = "reranker"))]
pub use reranking::RerankedDocument;

/// Rerank a list of documents by relevance to a query.
///
/// Returns documents sorted descending by score. Applies `top_k` truncation if
/// configured.
///
/// # Errors
///
/// - [`XbergError::Validation`] if `query` is empty or blank.
/// - [`XbergError::MissingDependency`] if ONNX Runtime is not installed (ONNX path).
/// - [`XbergError::Reranking`] if the preset is unknown or model download fails.
///
#[cfg(feature = "reranker")]
#[cfg_attr(alef, alef(skip))]
pub fn rerank(
    query: String,
    documents: Vec<String>,
    config: &core::config::RerankerConfig,
) -> crate::Result<Vec<reranking::RerankedDocument>> {
    reranking::rerank(query, documents, config)
}

#[cfg(all(feature = "reranker", feature = "tokio-runtime"))]
#[cfg_attr(alef, alef(skip))]
pub use reranking::rerank_async;

/// Get a reranker preset by name.
///
/// Returns `None` if no preset with the given name exists. Returns an owned
/// clone so the value is safe to pass across FFI boundaries.
///
#[cfg(feature = "reranker-presets")]
#[cfg_attr(alef, alef(skip))]
pub fn get_reranker_preset(name: &str) -> Option<reranking::RerankerPreset> {
    reranking::get_preset(name)
}

/// List the names of all available reranker presets.
///
/// Returns owned `String`s so the values are safe to pass across FFI boundaries.
///
#[cfg(feature = "reranker-presets")]
#[cfg_attr(alef, alef(skip))]
pub fn list_reranker_presets() -> Vec<String> {
    reranking::list_presets()
}

/// Stub result document type for builds without `reranker-presets`.
///
#[cfg(not(feature = "reranker-presets"))]
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct RerankedDocument {
    /// Position of this document in the original input slice.
    pub index: usize,
    /// Relevance score in `[0, 1]`.
    pub score: f32,
    /// The document text.
    pub document: String,
}

/// Re-export the sparse-embedding result and preset types when the presets
/// feature is active.
///
#[cfg(feature = "sparse-embedding-presets")]
pub use sparse_embeddings::{SparseEmbedding, SparseEmbeddingPreset};

/// Generate sparse (SPLADE) embeddings for a list of texts.
///
/// Returns one [`SparseEmbedding`] per input text, in order.
///
#[cfg(feature = "sparse-embeddings")]
#[cfg_attr(alef, alef(skip))]
pub fn embed_sparse(
    texts: Vec<String>,
    config: &core::config::SparseEmbeddingConfig,
) -> crate::Result<Vec<SparseEmbedding>> {
    sparse_embeddings::embed_sparse(&texts, config)
}

#[cfg(all(feature = "sparse-embeddings", feature = "tokio-runtime"))]
#[cfg_attr(alef, alef(skip))]
pub use sparse_embeddings::embed_sparse_async;

/// Get a sparse-embedding preset by name.
///
#[cfg(feature = "sparse-embedding-presets")]
#[cfg_attr(alef, alef(skip))]
pub fn get_sparse_embedding_preset(name: &str) -> Option<sparse_embeddings::SparseEmbeddingPreset> {
    sparse_embeddings::get_preset(name)
}

/// List the names of all available sparse-embedding presets.
///
#[cfg(feature = "sparse-embedding-presets")]
#[cfg_attr(alef, alef(skip))]
pub fn list_sparse_embedding_presets() -> Vec<String> {
    sparse_embeddings::list_presets()
}

/// Stub result type for builds without the `sparse-embedding-presets` feature.
///
/// Field names match the real type so JSON round-trips remain schema-compatible.
///
#[cfg(not(feature = "sparse-embedding-presets"))]
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct SparseEmbedding {
    /// Vocabulary token ids with non-zero weight, ascending.
    pub indices: Vec<u32>,
    /// Weights parallel to `indices`.
    pub values: Vec<f32>,
}

/// Stub preset type for builds without the `sparse-embedding-presets` feature.
///
#[cfg(not(feature = "sparse-embedding-presets"))]
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SparseEmbeddingPreset {
    /// Unique preset identifier (e.g. "splade").
    pub name: String,
    /// HuggingFace repository ID for the model.
    pub model_repo: String,
    /// ONNX model file name within the repository.
    pub model_file: String,
    /// Sibling files that must be downloaded alongside `model_file`.
    pub additional_files: Vec<String>,
    /// Maximum token sequence length the model supports.
    pub max_length: usize,
    /// Human-readable description of the preset's intended use case.
    pub description: String,
}

/// Re-export the multi-vector result/preset types and the pure-CPU MaxSim
/// primitives when the presets feature is active.
///
#[cfg(feature = "late-interaction-presets")]
pub use late_interaction::{
    LateInteractionMatch, LateInteractionPreset, MultiVectorEmbedding, max_sim_rank, max_sim_score,
};

/// Generate ColBERT multi-vector embeddings for a list of texts.
///
/// `is_query` selects `[Q]`/`[D]` marker insertion and, when `true`, query
/// augmentation padding.
///
#[cfg(feature = "late-interaction")]
#[cfg_attr(alef, alef(skip))]
pub fn embed_multi_vector(
    texts: Vec<String>,
    config: &core::config::LateInteractionConfig,
    is_query: bool,
) -> crate::Result<Vec<MultiVectorEmbedding>> {
    late_interaction::embed_multi_vector(&texts, config, is_query)
}

#[cfg(all(feature = "late-interaction", feature = "tokio-runtime"))]
#[cfg_attr(alef, alef(skip))]
pub use late_interaction::embed_multi_vector_async;

/// Get a late-interaction preset by name.
///
#[cfg(feature = "late-interaction-presets")]
#[cfg_attr(alef, alef(skip))]
pub fn get_late_interaction_preset(name: &str) -> Option<late_interaction::LateInteractionPreset> {
    late_interaction::get_preset(name)
}

/// List the names of all available late-interaction presets.
///
#[cfg(feature = "late-interaction-presets")]
#[cfg_attr(alef, alef(skip))]
pub fn list_late_interaction_presets() -> Vec<String> {
    late_interaction::list_presets()
}

/// Stub multi-vector result type for builds without the `late-interaction-presets` feature.
///
#[cfg(not(feature = "late-interaction-presets"))]
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct MultiVectorEmbedding {
    /// Number of attention-live token rows.
    pub num_tokens: u32,
    /// Dimensionality of each per-token vector.
    pub dim: u32,
    /// Flat row-major buffer, length `num_tokens * dim`.
    pub data: Vec<f32>,
}

/// Stub match type for builds without the `late-interaction-presets` feature.
///
#[cfg(not(feature = "late-interaction-presets"))]
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct LateInteractionMatch {
    /// Position of this document in the original input slice.
    pub index: usize,
    /// MaxSim relevance score.
    pub score: f32,
}

/// Stub preset type for builds without the `late-interaction-presets` feature.
///
#[cfg(not(feature = "late-interaction-presets"))]
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LateInteractionPreset {
    /// Unique preset identifier (e.g. "colbert").
    pub name: String,
    /// HuggingFace repository ID for the model.
    pub model_repo: String,
    /// ONNX model file name within the repository.
    pub model_file: String,
    /// Sibling files that must be downloaded alongside `model_file`.
    pub additional_files: Vec<String>,
    /// Maximum document token sequence length.
    pub max_length: usize,
    /// Fixed padded query length (ColBERT query augmentation).
    pub query_max_length: usize,
    /// Per-token embedding dimensionality.
    pub dim: usize,
    /// Human-readable description of the preset's intended use case.
    pub description: String,
}

/// Caption a single image from bytes using a configured LLM.
///
/// # Arguments
///
/// * `image_bytes` - The image data.
/// * `llm_config` - LLM configuration for the VLM call.
/// * `custom_prompt` - Optional custom caption prompt. Uses the default
///   `RegionKind::Caption` prompt when `None`.
///
/// # Returns
///
/// The generated caption text.
///
/// # Errors
///
/// Returns an error if the VLM call fails or if image format detection fails.
///
/// # Example
///
/// ```ignore
/// use xberg::captioning::caption_image;
/// use xberg::LlmConfig;
///
/// # async fn example() -> xberg::Result<()> {
/// let image_bytes = std::fs::read("photo.jpg")?;
/// let config = LlmConfig {
///     model: "openai/gpt-4o-mini".to_string(),
///     ..Default::default()
/// };
/// let caption = caption_image(&image_bytes, &config, None).await?;
/// println!("Caption: {}", caption);
/// # Ok(())
/// # }
/// ```
#[cfg(all(feature = "captioning", feature = "tokio-runtime"))]
#[cfg_attr(alef, alef(skip))]
pub use captioning::caption_image;

/// Caption a single image from a file path using a configured LLM.
///
/// # Arguments
///
/// * `path` - Path to the image file.
/// * `llm_config` - LLM configuration for the VLM call.
/// * `custom_prompt` - Optional custom caption prompt. Uses the default
///   `RegionKind::Caption` prompt when `None`.
///
/// # Returns
///
/// The generated caption text.
///
/// # Errors
///
/// Returns an error if the file cannot be read, if image format detection fails,
/// or if the VLM call fails.
///
/// # Example
///
/// ```ignore
/// use xberg::captioning::caption_image_file;
/// use xberg::LlmConfig;
///
/// # async fn example() -> xberg::Result<()> {
/// let config = LlmConfig {
///     model: "openai/gpt-4o-mini".to_string(),
///     ..Default::default()
/// };
/// let caption = caption_image_file("document_page_001.png", &config, None).await?;
/// # Ok(())
/// # }
/// ```
#[cfg(all(feature = "captioning", feature = "tokio-runtime"))]
#[cfg_attr(alef, alef(skip))]
pub use captioning::caption_image_file;

/// Caption multiple images in a single batch.
///
/// Processes images sequentially (not in parallel). Returns one caption per input image
/// in the same order. If a caption fails, the error is returned immediately without
/// processing remaining images.
///
/// # Arguments
///
/// * `images` - Slice of image byte references to caption.
/// * `llm_config` - LLM configuration for the VLM calls.
/// * `custom_prompt` - Optional custom caption prompt. Uses the default
///   `RegionKind::Caption` prompt when `None`.
///
/// # Returns
///
/// A vector of captions, one per input image, in the same order.
///
/// # Errors
///
/// Returns an error if any VLM call fails.
///
/// # Example
///
/// ```ignore
/// use xberg::captioning::caption_images;
/// use xberg::LlmConfig;
///
/// # async fn example() -> xberg::Result<()> {
/// let image1 = std::fs::read("photo1.jpg")?;
/// let image2 = std::fs::read("photo2.jpg")?;
/// let images = vec![image1.as_ref(), image2.as_ref()];
/// let config = LlmConfig {
///     model: "openai/gpt-4o-mini".to_string(),
///     ..Default::default()
/// };
/// let captions = caption_images(&images, &config, None).await?;
/// assert_eq!(captions.len(), 2);
/// # Ok(())
/// # }
/// ```
#[cfg(all(feature = "captioning", feature = "tokio-runtime"))]
#[cfg_attr(alef, alef(skip))]
pub use captioning::caption_images;

/// Unified post-extraction enrichment: classification, chunk classification, NER
/// and captioning in a single composable call. Transcription is not an enrichment
/// stage — it runs at extraction time via [`core::config::ExtractionConfig`].
pub mod enrich;
#[cfg_attr(alef, alef(skip))]
pub use enrich::enrich;
pub use enrich::{EnrichedResult, EnrichmentConfig};

#[cfg(feature = "ner")]
pub use enrich::NerEnrichmentConfig;

#[cfg(feature = "classification")]
pub use enrich::{ChunkClassificationEnrichmentConfig, ClassificationEnrichmentConfig};

#[cfg(feature = "captioning")]
pub use enrich::CaptioningEnrichmentConfig;

#[cfg(test)]
mod public_api_compile_tests {
    #[test]
    fn llm_and_concurrency_configs_are_available_at_crate_root() {
        fn accept_root_types(
            _concurrency: Option<crate::ConcurrencyConfig>,
            _provider: Option<crate::LlmProviderConfig>,
            _cache: Option<crate::LlmCacheConfig>,
            _budget: Option<crate::LlmBudgetConfig>,
            _rate_limit: Option<crate::LlmRateLimitConfig>,
        ) {
        }

        accept_root_types(None, None, None, None, None);
    }
}
