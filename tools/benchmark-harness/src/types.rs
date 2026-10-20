//! Core types for benchmark results and metrics

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

/// Output format for document extraction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// Markdown output format with structure preservation
    #[default]
    Markdown,
    /// Plain text output format
    Plaintext,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Markdown => write!(f, "markdown"),
            OutputFormat::Plaintext => write!(f, "plaintext"),
        }
    }
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "markdown" | "md" => Ok(OutputFormat::Markdown),
            "plaintext" | "text" | "txt" => Ok(OutputFormat::Plaintext),
            _ => Err(format!(
                "unknown output format: {}. Valid: markdown, md, plaintext, text, txt",
                s
            )),
        }
    }
}

/// Default output format for backward compatibility with old results
fn default_output_format() -> OutputFormat {
    OutputFormat::Markdown
}

/// Per-stage cold-start timing breakdown parsed from an xberg CLI JSON envelope's
/// `stage_timings` field (see `crates/xberg-cli/src/output.rs::StageTimings`).
///
/// Field names and semantics mirror the CLI struct exactly; this is a plain-data mirror rather
/// than a shared type because the benchmark harness does not depend on the `xberg-cli` crate.
/// Only populated when the harness invokes the CLI with `XBERG_EMIT_STAGE_TIMING` set.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StageTimings {
    /// Time from CLI process start to the point extraction begins (arg parsing, logging setup,
    /// config load/merge), in milliseconds.
    pub process_init_ms: f64,
    /// Wall-clock time for the core library's extraction call to return, in milliseconds.
    pub first_parse_ms: f64,
    /// Coarse approximation of ONNX Runtime session-creation-plus-first-inference cost, present
    /// only when a layout/OCR configuration that uses ORT was active. See the CLI-side
    /// `StageTimings` doc comment for why this is not an independently measured sub-stage.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub ort_session_and_inference_ms: Option<f64>,
}

/// Xberg extraction pipeline variant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum XbergPipeline {
    /// Baseline: text extraction without layout or OCR
    Baseline,
    /// Layout: layout detection and structure preservation
    Layout,
    /// PaddleOCR: OCR with PaddleOCR backend
    #[serde(rename = "paddle-ocr")]
    PaddleOcr,
    /// Baseline structure pipeline with the PaddleOCR engine (force-OCR, no layout model)
    #[serde(rename = "baseline-paddle")]
    BaselinePaddle,
    /// Layout structure pipeline with the PaddleOCR engine (force-OCR + layout model)
    #[serde(rename = "layout-paddle")]
    LayoutPaddle,
    /// Sceptre OCR using ONNX Runtime.
    #[serde(rename = "sceptre-ort")]
    SceptreOrt,
    /// Sceptre OCR using ONNX Runtime with layout detection.
    #[serde(rename = "sceptre-ort-layout")]
    SceptreOrtLayout,
    /// Sceptre OCR using ONNX Runtime with automatic page orientation correction.
    #[serde(rename = "sceptre-ort-autorotate")]
    SceptreOrtAutoRotate,
    /// Sceptre OCR using tract. This is reserved for the bounded diagnostic matrix.
    #[serde(rename = "sceptre-tract")]
    SceptreTract,
    /// Candle TrOCR: OCR with candle-based TrOCR backend
    #[serde(rename = "candle-trocr")]
    CandleTrocr,
    /// Candle PaddleOCR-VL: OCR with candle-based PaddleOCR-VL backend (end-to-end markdown)
    #[serde(rename = "candle-paddleocr-vl")]
    CandlePaddleocrVl,
    /// Candle GLM-OCR: OCR with candle-based GLM-OCR vision-language backend
    #[serde(rename = "candle-glm-ocr")]
    CandleGlmOcr,
    /// Candle DeepSeek-OCR: OCR with candle-based DeepSeek-OCR vision-language backend
    #[serde(rename = "candle-deepseek-ocr")]
    CandleDeepseekOcr,
    /// Candle PaddleOCR-VL 1.5: OCR with candle-based PaddleOCR-VL 1.5 vision-language backend
    #[serde(rename = "candle-paddleocr-vl-15")]
    CandlePaddleocrVl15,
}

impl XbergPipeline {
    /// Get the string representation of the pipeline
    pub fn as_str(self) -> &'static str {
        match self {
            XbergPipeline::Baseline => "baseline",
            XbergPipeline::Layout => "layout",
            XbergPipeline::PaddleOcr => "paddle-ocr",
            XbergPipeline::BaselinePaddle => "baseline-paddle",
            XbergPipeline::LayoutPaddle => "layout-paddle",
            XbergPipeline::SceptreOrt => "sceptre-ort",
            XbergPipeline::SceptreOrtLayout => "sceptre-ort-layout",
            XbergPipeline::SceptreOrtAutoRotate => "sceptre-ort-autorotate",
            XbergPipeline::SceptreTract => "sceptre-tract",
            XbergPipeline::CandleTrocr => "candle-trocr",
            XbergPipeline::CandlePaddleocrVl => "candle-paddleocr-vl",
            XbergPipeline::CandleGlmOcr => "candle-glm-ocr",
            XbergPipeline::CandleDeepseekOcr => "candle-deepseek-ocr",
            XbergPipeline::CandlePaddleocrVl15 => "candle-paddleocr-vl-15",
        }
    }
}

impl std::fmt::Display for XbergPipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for XbergPipeline {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "baseline" => Ok(XbergPipeline::Baseline),
            "layout" => Ok(XbergPipeline::Layout),
            "paddle-ocr" | "paddle_ocr" | "paddleocr" => Ok(XbergPipeline::PaddleOcr),
            "baseline-paddle" | "baseline_paddle" => Ok(XbergPipeline::BaselinePaddle),
            "layout-paddle" | "layout_paddle" => Ok(XbergPipeline::LayoutPaddle),
            "sceptre" | "sceptre-ort" | "sceptre_ort" => Ok(XbergPipeline::SceptreOrt),
            "sceptre-ort-layout" | "sceptre_ort_layout" | "sceptre-layout" | "sceptre_layout" => {
                Ok(XbergPipeline::SceptreOrtLayout)
            }
            "sceptre-ort-autorotate" | "sceptre_ort_autorotate" | "sceptre-autorotate" | "sceptre_autorotate" => {
                Ok(XbergPipeline::SceptreOrtAutoRotate)
            }
            "sceptre-tract" | "sceptre_tract" => Ok(XbergPipeline::SceptreTract),
            "candle-trocr" | "candle_trocr" | "trocr" => Ok(XbergPipeline::CandleTrocr),
            "candle-paddleocr-vl" | "candle_paddleocr_vl" | "paddleocr-vl" => Ok(XbergPipeline::CandlePaddleocrVl),
            "candle-glm-ocr" | "candle_glm_ocr" | "glm-ocr" => Ok(XbergPipeline::CandleGlmOcr),
            "candle-deepseek-ocr" | "candle_deepseek_ocr" | "deepseek-ocr" => Ok(XbergPipeline::CandleDeepseekOcr),
            "candle-paddleocr-vl-15" | "candle_paddleocr_vl_15" | "paddleocr-vl-15" => {
                Ok(XbergPipeline::CandlePaddleocrVl15)
            }
            _ => Err(format!(
                "unknown Xberg pipeline: {}. Valid: baseline, layout, paddle-ocr, baseline-paddle, layout-paddle, sceptre-ort, sceptre-ort-layout, sceptre-ort-autorotate, sceptre-tract, candle-trocr, candle-paddleocr-vl, candle-glm-ocr, candle-deepseek-ocr, candle-paddleocr-vl-15",
                s
            )),
        }
    }
}

/// Xberg PDF extraction backend (`--pdf-backend`), a cohort dimension independent of
/// [`XbergPipeline`]. `Native` is the harness default and must keep producing the same
/// `--pdf-backend native` argument and framework name it always has, so existing committed
/// results stay comparable. `Pdfium` is opt-in only: it must be requested explicitly via the
/// `run` subcommand's `--pdf-backends` flag (only the `baseline` xberg pipeline honors it --
/// the pdfium engine has no OCR fallback path). It is only meaningful when the `xberg` binary
/// the harness shells out to (see `adapters::xberg::create_xberg_adapter`) was itself built
/// with the `pdf-pdfium` Cargo feature and has a `libpdfium` runtime library reachable via
/// `PDFIUM_DYNAMIC_LIB_PATH` -- neither of those is a harness-side concern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum XbergPdfBackend {
    /// xberg's own pure-Rust PDF engine. Harness default.
    #[default]
    Native,
    /// Google's PDFium engine (`pdf-pdfium` Cargo feature). Opt-in only.
    Pdfium,
}

impl XbergPdfBackend {
    /// Get the string representation of the backend, matching xberg's `--pdf-backend` values.
    pub fn as_str(self) -> &'static str {
        match self {
            XbergPdfBackend::Native => "native",
            XbergPdfBackend::Pdfium => "pdfium",
        }
    }
}

impl std::fmt::Display for XbergPdfBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for XbergPdfBackend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().replace('_', "-").as_str() {
            "native" => Ok(XbergPdfBackend::Native),
            "pdfium" => Ok(XbergPdfBackend::Pdfium),
            other => Err(format!("unknown Xberg PDF backend '{other}'. Valid: native, pdfium")),
        }
    }
}

/// OCR usage status for a benchmark extraction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OcrStatus {
    /// OCR was used for this extraction
    Used,
    /// OCR was not used for this extraction
    NotUsed,
    /// Unknown whether OCR was used
    #[default]
    Unknown,
}

/// Categorizes the source of a benchmark error.
///
/// This distinction is critical:
/// - **FrameworkError**: the framework itself reported an extraction error (returned `{"error": "..."}`)
/// - **HarnessError**: harness infrastructure problem (process crash, invalid JSON output, etc.)
/// - **ConfigSetupError**: environment/dependency misconfiguration (missing models, torch module not available, etc.)
/// - **Timeout**: extraction exceeded configured timeout
/// - **EmptyContent**: framework ran but produced no content
/// - **ZeroOverlap**: framework produced non-empty output that shares zero tokens with a
///   non-empty ground truth
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    /// The framework itself reported an extraction error (returned `{"error": "..."}`)
    /// This is NOT our fault - the framework couldn't handle this file.
    FrameworkError,
    /// A harness-level error: process crash, invalid JSON output, subprocess failure, etc.
    /// This IS our fault or an infrastructure issue.
    HarnessError,
    /// Configuration or setup error: missing dependencies, environment misconfiguration
    /// (e.g., torch.PP-OCRv6 not available, partition_X not available, missing tessdata, etc.)
    ConfigSetupError,
    /// Extraction timed out (exceeded the configured timeout duration).
    Timeout,
    /// Framework returned empty or missing content (ran but produced nothing).
    EmptyContent,
    /// Framework produced non-empty output that shares zero tokens with a non-empty ground
    /// truth (i.e. it ran to completion but the content is unusable garbage, not genuinely
    /// empty). Distinct from [`ErrorKind::EmptyContent`], which means the framework produced
    /// no content at all. Both are framework-fault failures and both are timing-eligible.
    ZeroOverlap,
    /// No error occurred
    #[default]
    None,
}

/// Complete benchmark result for a single file extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Framework that performed the extraction
    pub framework: String,

    /// Output format used for extraction (markdown or plaintext)
    #[serde(default = "default_output_format")]
    pub output_format: OutputFormat,

    /// Path to the test document
    pub file_path: PathBuf,

    /// File size in bytes
    pub file_size: u64,

    /// Whether extraction succeeded
    pub success: bool,

    /// Error message if extraction failed
    pub error_message: Option<String>,

    /// Categorizes the error source (framework vs harness)
    #[serde(default)]
    pub error_kind: ErrorKind,

    /// Total wall-clock duration (process spawn + extraction)
    /// For single iteration: the actual duration
    /// For multiple iterations: mean duration across all iterations
    pub duration: Duration,

    /// Pure extraction time (reported by subprocess via _extraction_time_ms)
    /// Only available for external frameworks with internal timing
    pub extraction_duration: Option<Duration>,

    /// Subprocess overhead outside framework-reported extraction work.
    ///
    /// For native batches this is process-wall time minus the reported batch
    /// total. It is never inferred from per-item extraction timings.
    pub subprocess_overhead: Option<Duration>,

    /// Performance metrics (averaged across iterations if multiple)
    pub metrics: PerformanceMetrics,

    /// Quality metrics (if ground truth available)
    pub quality: Option<QualityMetrics>,

    /// Individual iteration results (empty for single iteration)
    pub iterations: Vec<IterationResult>,

    /// Statistical analysis of durations across iterations
    /// Only present when multiple iterations were run
    pub statistics: Option<DurationStatistics>,

    /// Cold start duration: Time from framework not loaded to ready and warm state
    /// This is measured during the first warmup extraction and represents the
    /// initial framework load time (imports, initializations, etc.)
    pub cold_start_duration: Option<Duration>,

    /// File extension without dot (e.g., "pdf", "docx")
    /// Extracted from file_path for per-extension analysis
    pub file_extension: String,

    /// Framework capability metadata at time of extraction
    /// Contains OCR support, batch support, async support flags
    pub framework_capabilities: FrameworkCapabilities,

    /// PDF-specific metadata (only present for PDF files)
    /// Includes text layer detection results and OCR strategy
    pub pdf_metadata: Option<PdfMetadata>,

    /// OCR usage status for this extraction
    #[serde(default)]
    pub ocr_status: OcrStatus,

    /// Extracted text content (for quality assessment)
    /// Not serialized to output JSON to save space
    #[serde(skip)]
    pub extracted_text: Option<String>,

    /// System load captured at measurement time.
    ///
    /// Recorded so local timing comparisons can be qualified: throughput and
    /// cold-start numbers taken under heavy background load are not comparable
    /// to those taken on an idle machine. `None` for results that predate this
    /// field or were constructed outside a measurement path.
    #[serde(default)]
    pub system_load: Option<crate::system_load::SystemLoad>,
}

impl BenchmarkResult {
    /// Create a framework key combining framework name, output format, and execution mode
    /// Format: "{framework}:{output_format}:{execution_mode}"
    /// Example: "xberg-rust:markdown:batch"
    pub fn framework_key(&self, execution_mode: &str) -> String {
        format!("{}:{}:{}", self.framework, self.output_format, execution_mode)
    }

    /// Whether this row owns the aggregate process metrics for its native batch.
    ///
    /// Native batch adapters mark exactly one process-metric sample. Single-file
    /// results omit the marker and always own their metrics.
    pub(crate) fn is_performance_sample(&self) -> bool {
        self.framework_capabilities.batch_performance_sample.unwrap_or_else(|| {
            self.framework_capabilities.batch_capability.is_none() || self.metrics.throughput_bytes_per_sec > 0.0
        })
    }
}

/// Whether a result's timing/resource measurements are valid to pool into performance
/// percentiles, independent of whether it counts as a quality success.
///
/// A result is timing-eligible if the framework actually ran to completion and produced output
/// within the measured window. That holds for every `success == true` result, and also for a
/// result reclassified as [`ErrorKind::ZeroOverlap`]: the framework really did run and really did
/// produce output in that time, even though the output turned out to share no tokens with the
/// ground truth. Gating eligibility on `success` alone would delete exactly the fast-but-garbage
/// samples from a competitor's distribution, biasing its duration/throughput percentiles toward
/// looking slower than it actually is relative to a framework (like xberg) that rarely produces
/// zero-overlap output.
pub(crate) fn is_timing_eligible(result: &BenchmarkResult) -> bool {
    result.success || result.error_kind == ErrorKind::ZeroOverlap
}

/// Deduplicated performance samples for a group of results: one row per single-file success, or
/// one anchor row per native batch (see [`BenchmarkResult::is_performance_sample`]). Timing
/// eligibility (see [`is_timing_eligible`]) gates inclusion, not raw `success` — a zero-overlap
/// result still contributes its duration/throughput/memory/cpu_seconds measurements even though
/// it is excluded from quality percentiles elsewhere.
pub(crate) fn successful_performance_samples<'a>(
    results: impl IntoIterator<Item = &'a BenchmarkResult>,
) -> Vec<&'a BenchmarkResult> {
    let mut batch_samples = std::collections::HashSet::new();
    let mut samples = Vec::new();
    for result in results {
        if !is_timing_eligible(result) {
            continue;
        }
        if let Some(sample_id) = result.framework_capabilities.batch_sample_id.as_deref() {
            if batch_samples.insert(sample_id) {
                samples.push(result);
            }
        } else if result.is_performance_sample() {
            samples.push(result);
        }
    }
    samples
}

/// Expand row-level iteration payloads back into invocation-level measurement rows.
/// Quality and coverage remain row-scoped; callers use these clones only for performance
/// distributions so repeated measurements are not collapsed to their stored mean. ~keep
pub(crate) fn expand_performance_iterations(results: &[&BenchmarkResult]) -> Vec<BenchmarkResult> {
    let mut expanded = Vec::new();
    for result in results {
        if result.iterations.is_empty() {
            expanded.push((*result).clone());
            continue;
        }
        for iteration in &result.iterations {
            let mut sample = (*result).clone();
            sample.success = iteration.success.unwrap_or(sample.success);
            sample.error_kind = iteration.error_kind.unwrap_or(sample.error_kind);
            sample.duration = iteration.duration;
            sample.extraction_duration = iteration.extraction_duration;
            sample.subprocess_overhead = iteration.subprocess_overhead;
            sample.metrics = iteration.metrics.clone();
            sample.framework_capabilities.batch_sample_id = iteration.batch_sample_id.clone();
            sample.framework_capabilities.batch_performance_sample = iteration.batch_performance_sample;
            sample.iterations.clear();
            sample.statistics = None;
            expanded.push(sample);
        }
    }
    expanded
}

/// Performance metrics collected during extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// RSS captured immediately after the monitor attached to the target.
    #[serde(default)]
    pub baseline_memory_bytes: u64,

    /// Absolute peak RSS in bytes.
    pub peak_memory_bytes: u64,

    /// Peak RSS above the captured baseline.
    #[serde(default)]
    pub peak_memory_delta_bytes: u64,

    /// Average CPU usage percentage (0-100)
    pub avg_cpu_percent: f64,

    /// Total process-tree CPU time consumed, in core-seconds.
    ///
    /// Computed by trapezoidal integration of the un-normalized process-tree CPU percentage
    /// over the resource sampler's timeline (see
    /// `crate::monitoring::ResourceMonitor::calculate_stats`). Precision is bounded by the
    /// sampling interval (1-10ms, adaptive on file size): CPU bursts shorter than the gap
    /// between two samples are smoothed by the trapezoidal average rather than measured
    /// exactly. `#[serde(default)]` so historical capture files without this field deserialize
    /// as `0.0` rather than failing.
    #[serde(default)]
    pub cpu_seconds: f64,

    /// Throughput in bytes per second
    pub throughput_bytes_per_sec: f64,

    /// 50th percentile memory usage in bytes
    pub p50_memory_bytes: u64,

    /// 95th percentile memory usage in bytes
    pub p95_memory_bytes: u64,

    /// 99th percentile memory usage in bytes
    pub p99_memory_bytes: u64,
}

/// Quality metrics comparing extraction output to ground truth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    /// Text token F1 score (0.0-1.0)
    pub f1_score_text: f64,

    /// Numeric token F1 score (0.0-1.0)
    pub f1_score_numeric: f64,

    /// Layout/structure F1 score (0.0-1.0), optional for plaintext mode
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub f1_score_layout: Option<f64>,

    /// Overall text quality score (0.0-1.0)
    pub quality_score: f64,

    /// Tokens in ground truth but missing/under-represented in extraction (recall misses).
    /// Each entry is (token, deficit_count). Sorted by count descending.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_tokens: Vec<(String, usize)>,

    /// Tokens in extraction but not in ground truth or over-represented (precision misses).
    /// Each entry is (token, surplus_count). Sorted by count descending.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_tokens: Vec<(String, usize)>,

    /// Whether the extraction is considered correct (quality_score >= 0.95).
    #[serde(default)]
    pub correct: bool,

    /// Reading-order fidelity (0.0-1.0), via anchor-based Longest Increasing Subsequence.
    ///
    /// REPORT-ONLY: intentionally excluded from `quality_score`. `f1_score_text` is a
    /// bag-of-tokens metric and cannot detect reading-order failure by construction; this
    /// field exists to make that failure mode visible without re-baselining any published
    /// score. See `quality::reading_order_score` for the algorithm. `None` means either
    /// side was empty or too few unambiguous anchor tokens were found to report a number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reading_order_score: Option<f64>,
}

/// Framework capability metadata
///
/// Records the capabilities of the framework at the time of extraction,
/// enabling proper analysis and comparison of results based on framework features.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FrameworkCapabilities {
    /// Process lifecycle represented by the performance measurement. ~keep
    #[serde(default = "unknown_timing_regime")]
    pub timing_regime: TimingRegime,

    /// Process boundary covered by resource metrics. ~keep
    #[serde(default = "unknown_resource_measurement_scope")]
    pub resource_measurement_scope: ResourceMeasurementScope,

    /// Extensions this framework supports (e.g., ["pdf", "docx"])
    #[serde(default)]
    pub supported_extensions: Vec<String>,

    /// Whether framework supports OCR
    #[serde(default)]
    pub ocr_support: bool,

    /// Whether framework supports batch processing
    #[serde(default)]
    pub batch_support: bool,

    /// Verified batch entry point and timing semantics used for this result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_capability: Option<BatchCapability>,

    /// Whether this row owns its native batch's process metrics.
    ///
    /// `None` denotes a single-file measurement. Native batches set exactly
    /// one row to `Some(true)` and all sibling rows to `Some(false)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_performance_sample: Option<bool>,

    /// Opaque identity shared by every document row from one batch process.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_sample_id: Option<String>,

    /// Whether framework supports async extraction
    #[serde(default)]
    pub async_support: bool,

    /// Output formats this framework supports
    #[serde(default)]
    pub supported_output_formats: Vec<OutputFormat>,

    /// Framework version
    #[serde(default)]
    pub version: String,

    /// Disk installation size (if known)
    #[serde(default)]
    pub installation_size: Option<DiskSizeInfo>,
}

/// Process lifecycle represented by a benchmark result. ~keep
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimingRegime {
    /// A fresh process is launched for each measured invocation.
    ColdProcess,
    /// Warmup and measurement execute in the same retained process.
    WarmInProcess,
    /// Aggregate-only marker for a malformed bucket that pooled unlike lifecycles.
    Mixed,
    /// Lifecycle metadata was absent or could not be established.
    #[default]
    Unknown,
}

pub(crate) const fn unknown_timing_regime() -> TimingRegime {
    TimingRegime::Unknown
}

/// Process boundary covered by memory and CPU measurements. ~keep
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ResourceMeasurementScope {
    /// Metrics target the isolated framework process tree.
    IsolatedProcess,
    /// Extraction runs inside the long-lived harness; absolute RSS/CPU are unavailable.
    HarnessProcessLatencyOnly,
    /// Resource measurement boundary was absent or could not be established. ~keep
    #[default]
    Unknown,
}

pub(crate) const fn unknown_resource_measurement_scope() -> ResourceMeasurementScope {
    ResourceMeasurementScope::Unknown
}

/// Concrete framework API used for a batch benchmark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchEntryPoint {
    XbergCliExtractBatch,
    XbergRustEngineExtractBatch,
    DoclingJobkit,
    LiteparseBatchParse,
    MineruDoParse,
    MineruParser,
}

/// Scope represented by the measured batch makespan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchTimingScope {
    WarmSteadyState,
    ColdEndToEndSubprocess,
}

/// Verified batch capability advertised by an adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchCapability {
    pub entry_point: BatchEntryPoint,
    pub timing_scope: BatchTimingScope,
    pub per_item_timing: bool,
}

fn is_zero_u64(v: &u64) -> bool {
    *v == 0
}

/// Disk installation size information for a framework
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskSizeInfo {
    /// Total size in bytes (package + system deps)
    pub size_bytes: u64,

    /// Package-only size in bytes (before adding system deps)
    #[serde(default)]
    pub package_bytes: u64,

    /// System dependency size in bytes (libreoffice, tesseract, ffmpeg, etc.)
    #[serde(default)]
    pub system_deps_bytes: u64,

    /// ML model size in bytes (auto-downloaded on first use)
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub model_bytes: u64,

    /// Measurement method (e.g., "binary_size", "pip_package", "npm_package")
    pub method: String,

    /// Human-readable description
    pub description: String,

    /// Breakdown of system dependency sizes by package name
    /// Keys are package names (e.g., "poppler-utils"), values are installed sizes in bytes.
    /// Only populated when runtime measurement via dpkg-query succeeds.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub system_deps_detail: HashMap<String, u64>,
}

/// PDF-specific metadata
///
/// Contains PDF text layer detection results and OCR strategy used.
/// Only populated for PDF documents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfMetadata {
    /// Whether PDF has a quality text layer
    /// Detected via pdftotext/pdffonts/pypdf
    pub has_text_layer: bool,

    /// Detection method used ("pdftotext", "pdffonts", "pypdf", "fallback")
    pub detection_method: String,

    /// Number of pages in the PDF
    pub page_count: Option<u32>,

    /// Whether OCR was enabled for this extraction
    pub ocr_enabled: bool,

    /// Text extraction quality hint (0.0-1.0)
    /// 0.0 = scanned image, 1.0 = native text
    pub text_quality_score: Option<f64>,
}

/// Result from a single benchmark iteration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterationResult {
    /// Iteration number (1-indexed). ~keep
    pub iteration: usize,

    /// Whether this invocation succeeded; absent only in legacy artifacts. ~keep
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,

    /// Invocation-level failure class; absent only in legacy artifacts. ~keep
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_kind: Option<ErrorKind>,

    /// Total wall-clock duration for this iteration
    pub duration: Duration,

    /// Pure extraction time (if available from subprocess)
    pub extraction_duration: Option<Duration>,

    /// Process overhead for this invocation, when the adapter exposes it. ~keep
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subprocess_overhead: Option<Duration>,

    /// Performance metrics for this iteration
    pub metrics: PerformanceMetrics,

    /// Native-batch invocation identity retained before row-level aggregation. ~keep
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_sample_id: Option<String>,

    /// Whether this file row owns the invocation's process-level sample. ~keep
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_performance_sample: Option<bool>,
}

/// Statistical analysis of durations across multiple iterations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationStatistics {
    /// Mean duration
    pub mean: Duration,

    /// Median duration
    pub median: Duration,

    /// Standard deviation (in milliseconds as f64)
    pub std_dev_ms: f64,

    /// Minimum duration
    pub min: Duration,

    /// Maximum duration
    pub max: Duration,

    /// 95th percentile duration
    pub p95: Duration,

    /// 99th percentile duration
    pub p99: Duration,

    /// Number of iterations included in statistics
    pub sample_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_framework_capabilities_fail_closed() {
        let capabilities = FrameworkCapabilities::default();
        assert_eq!(capabilities.timing_regime, TimingRegime::Unknown);
        assert_eq!(
            capabilities.resource_measurement_scope,
            ResourceMeasurementScope::Unknown
        );
    }

    #[test]
    fn legacy_paddle_pipeline_parses_supported_spellings() {
        for value in ["paddle-ocr", "paddle_ocr", "paddleocr"] {
            assert_eq!(value.parse::<XbergPipeline>(), Ok(XbergPipeline::PaddleOcr));
        }
    }

    #[test]
    fn sceptre_pipeline_identities_are_explicit_and_alias_defaults_to_ort() {
        assert_eq!("sceptre".parse(), Ok(XbergPipeline::SceptreOrt));
        for pipeline in [
            XbergPipeline::SceptreOrt,
            XbergPipeline::SceptreOrtLayout,
            XbergPipeline::SceptreOrtAutoRotate,
            XbergPipeline::SceptreTract,
        ] {
            assert_eq!(pipeline.as_str().parse(), Ok(pipeline));
            assert!(pipeline.as_str().contains("ort") || pipeline.as_str().contains("tract"));
        }
    }

    #[test]
    fn pdf_backend_parses_case_and_separator_insensitively() {
        for value in ["native", "Native", "NATIVE"] {
            assert_eq!(value.parse::<XbergPdfBackend>(), Ok(XbergPdfBackend::Native));
        }
        for value in ["pdfium", "Pdfium", "PDFIUM"] {
            assert_eq!(value.parse::<XbergPdfBackend>(), Ok(XbergPdfBackend::Pdfium));
        }
    }

    #[test]
    fn pdf_backend_rejects_unknown_value_and_lists_valid_ones() {
        let error = "pdf_oxide".parse::<XbergPdfBackend>().unwrap_err();
        assert_eq!(error, "unknown Xberg PDF backend 'pdf-oxide'. Valid: native, pdfium");
    }

    #[test]
    fn stage_timings_round_trips_with_ort_field_present() {
        let timings = StageTimings {
            process_init_ms: 12.5,
            first_parse_ms: 1150.25,
            ort_session_and_inference_ms: Some(1150.25),
        };

        let json = serde_json::to_string(&timings).expect("serialize StageTimings");
        let parsed: StageTimings = serde_json::from_str(&json).expect("deserialize StageTimings");

        assert_eq!(parsed.process_init_ms, 12.5);
        assert_eq!(parsed.first_parse_ms, 1150.25);
        assert_eq!(parsed.ort_session_and_inference_ms, Some(1150.25));
    }

    #[test]
    fn stage_timings_omits_ort_field_when_absent() {
        let timings = StageTimings {
            process_init_ms: 8.0,
            first_parse_ms: 235.0,
            ort_session_and_inference_ms: None,
        };

        let json = serde_json::to_string(&timings).expect("serialize StageTimings");

        assert!(
            !json.contains("ort_session_and_inference_ms"),
            "expected ort_session_and_inference_ms to be skipped when None, got: {json}"
        );
    }

    #[test]
    fn stage_timings_parses_from_cli_json_shape() {
        // Mirrors the exact JSON shape emitted by `xberg-cli`'s `output::StageTimings` when
        // XBERG_EMIT_STAGE_TIMING is set and layout/OCR is active. ~keep
        let raw = r#"{
            "process_init_ms": 4.2,
            "first_parse_ms": 1171.0,
            "ort_session_and_inference_ms": 1171.0
        }"#;

        let parsed: StageTimings = serde_json::from_str(raw).expect("parse CLI stage_timings JSON");

        assert_eq!(parsed.process_init_ms, 4.2);
        assert_eq!(parsed.first_parse_ms, 1171.0);
        assert_eq!(parsed.ort_session_and_inference_ms, Some(1171.0));
    }

    fn minimal_result(success: bool, error_kind: ErrorKind, duration_ms: u64, throughput_bps: f64) -> BenchmarkResult {
        BenchmarkResult {
            framework: "fw".to_string(),
            output_format: OutputFormat::Markdown,
            file_path: PathBuf::from("test.pdf"),
            file_size: 1024,
            success,
            error_message: if success { None } else { Some("failed".to_string()) },
            error_kind,
            duration: Duration::from_millis(duration_ms),
            extraction_duration: None,
            subprocess_overhead: None,
            metrics: PerformanceMetrics {
                baseline_memory_bytes: 0,
                peak_memory_bytes: 1_000_000,
                peak_memory_delta_bytes: 1_000_000,
                avg_cpu_percent: 50.0,
                cpu_seconds: 1.0,
                throughput_bytes_per_sec: throughput_bps,
                p50_memory_bytes: 1_000_000,
                p95_memory_bytes: 1_000_000,
                p99_memory_bytes: 1_000_000,
            },
            quality: None,
            iterations: vec![],
            statistics: None,
            cold_start_duration: None,
            file_extension: "pdf".to_string(),
            framework_capabilities: FrameworkCapabilities::default(),
            pdf_metadata: None,
            ocr_status: OcrStatus::NotUsed,
            extracted_text: None,
            system_load: None,
        }
    }

    #[test]
    fn error_kind_zero_overlap_round_trips_through_json() {
        let json = serde_json::to_string(&ErrorKind::ZeroOverlap).expect("serialize ErrorKind::ZeroOverlap");
        assert_eq!(json, "\"zero_overlap\"");

        let parsed: ErrorKind = serde_json::from_str(&json).expect("deserialize ErrorKind::ZeroOverlap");
        assert_eq!(parsed, ErrorKind::ZeroOverlap);
    }

    /// The Defect A fix: a zero-overlap result is not a quality success, but it is still
    /// timing-eligible, so it must survive into the performance-sample set alongside genuine
    /// successes. A genuinely-empty result (`ErrorKind::EmptyContent`) is not timing-eligible and
    /// stays excluded, proving the two `success == false` cases are treated differently on
    /// purpose rather than both being swept up by a broad `!success` check.
    #[test]
    fn successful_performance_samples_includes_zero_overlap_but_not_empty_content() {
        let success = minimal_result(true, ErrorKind::None, 100, 1_000_000.0);
        let zero_overlap = minimal_result(false, ErrorKind::ZeroOverlap, 200, 2_000_000.0);
        let empty_content = minimal_result(false, ErrorKind::EmptyContent, 300, 3_000_000.0);

        let results = [success, zero_overlap, empty_content];
        let samples = successful_performance_samples(results.iter());

        assert_eq!(
            samples.len(),
            2,
            "expected the success and the zero-overlap result, got {samples:?}"
        );
        assert!(samples.iter().any(|r| r.error_kind == ErrorKind::None));
        assert!(samples.iter().any(|r| r.error_kind == ErrorKind::ZeroOverlap));
        assert!(!samples.iter().any(|r| r.error_kind == ErrorKind::EmptyContent));
    }

    #[test]
    fn is_timing_eligible_matches_success_or_zero_overlap() {
        assert!(is_timing_eligible(&minimal_result(true, ErrorKind::None, 100, 1.0)));
        assert!(is_timing_eligible(&minimal_result(
            false,
            ErrorKind::ZeroOverlap,
            100,
            1.0
        )));
        assert!(!is_timing_eligible(&minimal_result(
            false,
            ErrorKind::EmptyContent,
            100,
            1.0
        )));
        assert!(!is_timing_eligible(&minimal_result(
            false,
            ErrorKind::FrameworkError,
            100,
            1.0
        )));
        assert!(!is_timing_eligible(&minimal_result(
            false,
            ErrorKind::HarnessError,
            100,
            1.0
        )));
        assert!(!is_timing_eligible(&minimal_result(
            false,
            ErrorKind::Timeout,
            100,
            1.0
        )));
        assert!(!is_timing_eligible(&minimal_result(
            false,
            ErrorKind::ConfigSetupError,
            100,
            1.0
        )));
    }
}
