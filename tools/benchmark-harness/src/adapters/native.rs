//! Native Xberg Rust adapter
//!
//! This adapter measures same-process Xberg extraction latency after a configured warmup.
//! Its rows are reported separately from cold-process competitive rankings.

use crate::adapter::FrameworkAdapter;
use crate::adapters::subprocess::SubprocessAdapter;
use crate::extract_xberg_file;
use crate::monitoring::{ResourceMonitor, ResourceStats};
use crate::types::{
    BatchCapability, BatchEntryPoint, BatchTimingScope, BenchmarkResult, ErrorKind, FrameworkCapabilities, OcrStatus,
    PerformanceMetrics, ResourceMeasurementScope, TimingRegime,
};
use crate::{Error, Result};
use async_trait::async_trait;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use xberg::{ExtractedDocument, ExtractionConfig, FormatMetadata};

/// Assemble the metrics block every `BenchmarkResult` in this adapter carries.
///
/// Harness-process RSS and CPU cannot be attributed to one extraction, so these metrics are
/// explicitly unavailable while latency and throughput remain reportable. ~keep
fn metrics_from(resource_stats: &ResourceStats, throughput_bytes_per_sec: f64) -> PerformanceMetrics {
    let _ = resource_stats;
    PerformanceMetrics {
        baseline_memory_bytes: 0,
        peak_memory_bytes: 0,
        peak_memory_delta_bytes: 0,
        avg_cpu_percent: 0.0,
        cpu_seconds: 0.0,
        throughput_bytes_per_sec,
        p50_memory_bytes: 0,
        p95_memory_bytes: 0,
        p99_memory_bytes: 0,
    }
}

fn native_capabilities() -> FrameworkCapabilities {
    FrameworkCapabilities {
        timing_regime: TimingRegime::WarmInProcess,
        resource_measurement_scope: ResourceMeasurementScope::HarnessProcessLatencyOnly,
        supported_extensions: native_supported_extensions(),
        ocr_support: true,
        batch_support: true,
        batch_capability: Some(native_batch_capability()),
        supported_output_formats: vec![
            crate::types::OutputFormat::Markdown,
            crate::types::OutputFormat::Plaintext,
        ],
        version: env!("CARGO_PKG_VERSION").to_string(),
        ..Default::default()
    }
}

fn native_batch_capability() -> BatchCapability {
    BatchCapability {
        entry_point: BatchEntryPoint::XbergRustEngineExtractBatch,
        timing_scope: BatchTimingScope::WarmSteadyState,
        per_item_timing: true,
    }
}

static NATIVE_BATCH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn native_batch_sample_id(adapter: &NativeAdapter) -> String {
    let mut hasher = blake3::Hasher::new();
    let sequence = NATIVE_BATCH_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let invocation_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    hasher.update(&std::process::id().to_le_bytes());
    hasher.update(&(std::ptr::from_ref(adapter).addr() as u64).to_le_bytes());
    hasher.update(&sequence.to_le_bytes());
    hasher.update(&invocation_time.to_le_bytes());
    hasher.finalize().to_hex().to_string()
}

/// The per-file facts both single-file rows share, independent of extraction success.
struct SingleResultContext<'a> {
    framework: &'a str,
    output_format: crate::types::OutputFormat,
    file_path: &'a Path,
    file_size: u64,
    duration: Duration,
    extraction_duration: Duration,
    resource_stats: &'a ResourceStats,
}

fn single_failure_result(context: SingleResultContext<'_>, error: &Error, timed_out: bool) -> BenchmarkResult {
    let error_kind = if timed_out {
        ErrorKind::Timeout
    } else {
        ErrorKind::HarnessError
    };
    BenchmarkResult {
        framework: context.framework.to_string(),
        output_format: context.output_format,
        file_path: context.file_path.to_path_buf(),
        file_size: context.file_size,
        success: false,
        error_message: Some(error.to_string()),
        error_kind,
        duration: context.duration,
        extraction_duration: Some(context.extraction_duration),
        subprocess_overhead: Some(Duration::ZERO),
        metrics: metrics_from(context.resource_stats, 0.0),
        quality: None,
        iterations: vec![],
        statistics: None,
        cold_start_duration: None,
        file_extension: context
            .file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown")
            .to_lowercase(),
        framework_capabilities: native_capabilities(),
        pdf_metadata: None,
        ocr_status: OcrStatus::Unknown,
        extracted_text: None,
        system_load: None,
    }
}

fn single_success_result(
    context: SingleResultContext<'_>,
    extraction_result: ExtractedDocument,
    ocr_status: OcrStatus,
    throughput: f64,
) -> BenchmarkResult {
    let (success, error_message, error_kind) = if extraction_result.content.trim().is_empty() {
        (
            false,
            Some("Framework returned empty content".to_string()),
            ErrorKind::EmptyContent,
        )
    } else {
        (true, None, ErrorKind::None)
    };

    BenchmarkResult {
        framework: context.framework.to_string(),
        output_format: context.output_format,
        file_path: context.file_path.to_path_buf(),
        file_size: context.file_size,
        success,
        error_message,
        error_kind,
        duration: context.duration,
        extraction_duration: Some(context.extraction_duration),
        subprocess_overhead: Some(Duration::ZERO),
        metrics: metrics_from(context.resource_stats, throughput),
        quality: None,
        iterations: vec![],
        statistics: None,
        cold_start_duration: None,
        file_extension: context
            .file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown")
            .to_lowercase(),
        framework_capabilities: native_capabilities(),
        pdf_metadata: None,
        ocr_status,
        extracted_text: Some(extraction_result.content),
        system_load: None,
    }
}

fn reject_zero_timeout(timeout: Duration, cancel_token: &xberg::cancellation::CancellationToken) -> Result<()> {
    if timeout.is_zero() {
        cancel_token.cancel();
        return Err(Error::Timeout(
            "Batch extraction exceeded 0ns; retained Engine is no longer reusable".to_string(),
        ));
    }
    Ok(())
}

fn sum_file_sizes(file_paths: &[&Path]) -> u64 {
    file_paths
        .iter()
        .filter_map(|path| std::fs::metadata(path).ok())
        .map(|m| m.len())
        .sum()
}

/// Validate the per-file OCR arrays against `file_paths` and build one `ExtractInput` per file.
///
/// # Errors
///
/// Returns [`Error::Benchmark`] if the OCR arrays do not have one entry per file — a mismatch
/// would silently misalign the overrides.
fn build_batch_inputs(
    file_paths: &[&Path],
    force_ocr: &[bool],
    ocr_languages: &[Option<String>],
    config: &ExtractionConfig,
) -> Result<Vec<xberg::ExtractInput>> {
    if force_ocr.len() != file_paths.len() || ocr_languages.len() != file_paths.len() {
        return Err(Error::Benchmark(format!(
            "native batch config cardinality mismatch for {} files",
            file_paths.len()
        )));
    }

    Ok(file_paths
        .iter()
        .zip(force_ocr)
        .zip(ocr_languages)
        .map(|((path, force_ocr), ocr_language)| build_batch_input(path, *force_ocr, ocr_language.as_deref(), config))
        .collect())
}

/// Turn one successful batch envelope into one row per input file.
///
/// Xberg returns successful documents in discovery order, each carrying `source_index`, and
/// reports failed inputs separately by request index. Reject ambiguous or missing outcomes, then
/// emit exactly one request-ordered row. Multi-document inputs fail closed because their child
/// timing and metadata cannot be truthfully collapsed into one performance row. ~keep
fn assemble_batch_rows(
    context: &BatchRowContext<'_>,
    file_paths: &[&Path],
    output: &xberg::ExtractionResult,
    config: &ExtractionConfig,
) -> Result<Vec<BenchmarkResult>> {
    let mut results_by_input: Vec<Vec<&ExtractedDocument>> = vec![Vec::new(); file_paths.len()];
    for (position, result) in output.results.iter().enumerate() {
        let source_index = result
            .metadata
            .additional
            .get("source_index")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| {
                Error::Benchmark(format!(
                    "native batch result at position {position} is missing a valid metadata.additional.source_index"
                ))
            })?;
        let Some(slot) = results_by_input.get_mut(source_index) else {
            return Err(Error::Benchmark(format!(
                "native batch result at position {position} has out-of-range source_index {source_index} for {} inputs",
                file_paths.len()
            )));
        };
        slot.push(result);
    }

    let mut errors_by_input = vec![None; file_paths.len()];
    for error in &output.errors {
        let Some(slot) = errors_by_input.get_mut(error.index) else {
            return Err(Error::Benchmark(format!(
                "native batch error has out-of-range input index {} for {} inputs",
                error.index,
                file_paths.len()
            )));
        };
        if slot.replace(error.message.as_str()).is_some() {
            return Err(Error::Benchmark(format!(
                "native batch returned multiple errors for input index {}",
                error.index
            )));
        }
    }

    file_paths
        .iter()
        .enumerate()
        .map(|(input_index, file_path)| {
            let documents = &results_by_input[input_index];
            match (documents.is_empty(), errors_by_input[input_index]) {
                (false, Some(_)) => Err(Error::Benchmark(format!(
                    "native batch returned both results and an error for input index {input_index}"
                ))),
                (true, Some(message)) => Ok(batch_failure_result(
                    context,
                    file_path,
                    message.to_string(),
                    ErrorKind::FrameworkError,
                )),
                (false, None) if documents.len() == 1 => {
                    Ok(batch_success_result(context, file_path, documents[0], config))
                }
                (false, None) => Err(Error::Benchmark(format!(
                    "native batch returned multiple results for input index {input_index}; cannot aggregate per-child timing and metadata truthfully"
                ))),
                (true, None) => Err(Error::Benchmark(format!(
                    "native batch returned no result or error for input index {input_index}"
                ))),
            }
        })
        .collect()
}

fn finalize_batch_rows(
    adapter: &NativeAdapter,
    mut results: Vec<BenchmarkResult>,
    total_duration: Duration,
    resource_stats: &ResourceStats,
) -> Vec<BenchmarkResult> {
    let successful_bytes: u64 = results
        .iter()
        .filter(|result| result.success)
        .map(|result| result.file_size)
        .sum();
    let batch_throughput = if total_duration.is_zero() {
        0.0
    } else {
        successful_bytes as f64 / total_duration.as_secs_f64()
    };
    let throughput_anchor = results.iter().position(|result| result.success);
    let batch_sample_id = native_batch_sample_id(adapter);
    for (index, result) in results.iter_mut().enumerate() {
        result.duration = total_duration;
        result.subprocess_overhead = Some(Duration::ZERO);
        result.metrics = metrics_from(resource_stats, batch_throughput);
        result.framework_capabilities.batch_performance_sample = Some(throughput_anchor == Some(index));
        result.framework_capabilities.batch_sample_id = Some(batch_sample_id.clone());
    }
    results
}

/// The facts every row of one batch shares.
struct BatchRowContext<'a> {
    framework: &'a str,
    output_format: crate::types::OutputFormat,
    avg_duration_per_file: Duration,
    resource_stats: &'a ResourceStats,
}

/// Build a failed per-file row. Used for both a whole-batch failure and a per-input
/// `output.errors` entry, so both paths carry identical metrics. ~keep
fn batch_failure_result(
    context: &BatchRowContext<'_>,
    file_path: &Path,
    error_message: String,
    error_kind: ErrorKind,
) -> BenchmarkResult {
    let file_size = std::fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);
    let file_extension = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_string();
    BenchmarkResult {
        framework: context.framework.to_string(),
        output_format: context.output_format,
        file_path: file_path.to_path_buf(),
        file_size,
        success: false,
        error_message: Some(error_message),
        error_kind,
        duration: context.avg_duration_per_file,
        extraction_duration: Some(context.avg_duration_per_file),
        subprocess_overhead: Some(Duration::ZERO),
        metrics: metrics_from(context.resource_stats, 0.0),
        quality: None,
        iterations: vec![],
        statistics: None,
        cold_start_duration: None,
        file_extension,
        framework_capabilities: native_capabilities(),
        pdf_metadata: None,
        ocr_status: OcrStatus::Unknown,
        extracted_text: None,
        system_load: None,
    }
}

fn batch_success_result(
    context: &BatchRowContext<'_>,
    file_path: &Path,
    extraction_result: &ExtractedDocument,
    config: &ExtractionConfig,
) -> BenchmarkResult {
    let file_size = std::fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);

    let extraction_duration = extraction_result
        .metadata
        .extraction_duration_ms
        .filter(|&ms| ms > 0)
        .map(Duration::from_millis)
        .unwrap_or(context.avg_duration_per_file);

    let file_throughput = if extraction_duration > Duration::from_secs(0) {
        file_size as f64 / extraction_duration.as_secs_f64()
    } else {
        0.0
    };

    let file_extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();

    let (success, error_message, error_kind) = if extraction_result.metadata.error.is_some() {
        (
            false,
            extraction_result.metadata.error.as_ref().map(|e| e.message.clone()),
            ErrorKind::FrameworkError,
        )
    } else if extraction_result.content.trim().is_empty() {
        (
            false,
            Some("Framework returned empty content".to_string()),
            ErrorKind::EmptyContent,
        )
    } else {
        (true, None, ErrorKind::None)
    };

    BenchmarkResult {
        framework: context.framework.to_string(),
        output_format: context.output_format,
        file_path: file_path.to_path_buf(),
        file_size,
        success,
        error_message,
        error_kind,
        duration: extraction_duration,
        extraction_duration: Some(extraction_duration),
        subprocess_overhead: Some(Duration::ZERO),
        metrics: metrics_from(context.resource_stats, file_throughput),
        quality: None,
        iterations: vec![],
        statistics: None,
        cold_start_duration: None,
        file_extension,
        framework_capabilities: native_capabilities(),
        pdf_metadata: None,
        ocr_status: determine_ocr_status(extraction_result, config),
        extracted_text: Some(extraction_result.content.clone()),
        system_load: None,
    }
}

fn native_supported_extensions() -> Vec<String> {
    xberg::list_supported_formats()
        .into_iter()
        .map(|format| format.extension)
        .collect()
}

/// Determine OCR status by inspecting the actual extraction result metadata.
///
/// The xberg crate sets `FormatMetadata::Ocr` for raw tesseract results, but the
/// image extractor overwrites format to `FormatMetadata::Image` even when OCR was used.
/// So we also check: if the format is `Image` and OCR was enabled in config, OCR was used.
///
/// Returns:
/// - `OcrStatus::Used` if OCR metadata is present, or if this is an image with OCR enabled
/// - `OcrStatus::NotUsed` if format metadata is present and OCR was not involved
/// - `OcrStatus::Unknown` if no format metadata is available
fn determine_ocr_status(result: &ExtractedDocument, config: &ExtractionConfig) -> OcrStatus {
    match &result.metadata.format {
        Some(FormatMetadata::Ocr(_)) => OcrStatus::Used,
        Some(FormatMetadata::Image(_)) => {
            if config.ocr.is_some() || config.force_ocr {
                OcrStatus::Used
            } else {
                OcrStatus::NotUsed
            }
        }
        Some(_) => OcrStatus::NotUsed,
        None => OcrStatus::Unknown,
    }
}

/// Native Rust adapter using xberg crate directly
pub struct NativeAdapter {
    name: String,
    config: ExtractionConfig,
    cold_adapter: Option<SubprocessAdapter>,
    engine: xberg::engine::Engine,
}

fn apply_ocr_language(ocr: &mut xberg::OcrConfig, language: &str) {
    ocr.language = crate::adapter::canonicalize_ocr_languages(language);
    if ocr.backend == "tesseract" {
        let tesseract = ocr.tesseract_config.get_or_insert_with(Default::default);
        tesseract.language.clone_from(&ocr.language);
        tesseract.use_cache = false;
    }
}

impl NativeAdapter {
    /// Create a new native adapter with default configuration
    ///
    /// NOTE: Cache is explicitly disabled for accurate benchmarking
    pub fn new() -> Self {
        let config = ExtractionConfig {
            use_cache: false,
            ..Default::default()
        };
        Self {
            name: "xberg-rust-steady-state".to_string(),
            config,
            cold_adapter: None,
            engine: xberg::engine::Engine::new_default(),
        }
    }

    /// Clone the adapter's base config and apply the per-fixture OCR overrides.
    fn extraction_config_for(&self, force_ocr: bool, ocr_language: Option<&str>) -> ExtractionConfig {
        let mut config = self.config.clone();
        config.force_ocr |= force_ocr;
        if let Some(language) = ocr_language {
            apply_ocr_language(config.ocr.get_or_insert_with(Default::default), language);
        }
        config
    }

    /// Calculate adaptive sampling interval based on estimated task duration from file size
    ///
    /// Uses file size as a proxy for task duration to optimize sampling frequency:
    /// - Small files (<100KB, ~50-100ms tasks): 1ms sampling for high resolution
    /// - Medium files (100KB-1MB, ~100-1000ms tasks): 5ms sampling for balance
    /// - Large files (>1MB, >1000ms tasks): 10ms sampling to reduce overhead
    ///
    /// This adaptive approach ensures:
    /// - Quick tasks: 50-100 samples (sufficient for variance calculation)
    /// - Long tasks: 100-1000+ samples (excellent statistical significance)
    /// - Minimal monitoring overhead for all workloads
    ///
    /// # Arguments
    /// * `file_size` - File size in bytes
    ///
    /// # Returns
    /// Sampling interval in milliseconds (1, 5, or 10)
    fn calculate_adaptive_sampling_interval(file_size: u64) -> u64 {
        crate::monitoring::adaptive_sampling_interval_ms(file_size)
    }

    /// Create a new native adapter with custom configuration
    pub fn with_config(config: ExtractionConfig) -> Self {
        Self {
            name: "xberg-rust-steady-state".to_string(),
            config,
            cold_adapter: None,
            engine: xberg::engine::Engine::new_default(),
        }
    }

    pub(crate) fn with_identity_and_cold_adapter(
        name: String,
        config: ExtractionConfig,
        cold_adapter: SubprocessAdapter,
    ) -> Self {
        Self {
            name,
            config,
            cold_adapter: Some(cold_adapter),
            engine: xberg::engine::Engine::new_default(),
        }
    }

    pub(crate) fn with_identity(name: String, config: ExtractionConfig) -> Self {
        Self {
            name,
            config,
            cold_adapter: None,
            engine: xberg::engine::Engine::new_default(),
        }
    }

    #[cfg(test)]
    pub(crate) fn has_separate_cold_probe(&self) -> bool {
        self.cold_adapter.is_some()
    }
}

impl Default for NativeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FrameworkAdapter for NativeAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn timing_regime(&self) -> TimingRegime {
        TimingRegime::WarmInProcess
    }

    fn resource_measurement_scope(&self) -> ResourceMeasurementScope {
        ResourceMeasurementScope::HarnessProcessLatencyOnly
    }

    fn batch_capability(&self) -> Option<BatchCapability> {
        Some(native_batch_capability())
    }

    fn supports_format(&self, file_type: &str) -> bool {
        let file_type = file_type.to_ascii_lowercase();
        xberg::list_supported_formats()
            .iter()
            .any(|format| format.extension == file_type)
    }

    fn supported_output_formats(&self) -> Vec<crate::types::OutputFormat> {
        vec![
            crate::types::OutputFormat::Markdown,
            crate::types::OutputFormat::Plaintext,
        ]
    }

    fn ocr_language_policy(&self) -> crate::adapter::OcrLanguagePolicy {
        crate::adapter::OcrLanguagePolicy::AnyPerDocument
    }

    async fn extract(
        &self,
        file_path: &Path,
        timeout: Duration,
        force_ocr: bool,
        ocr_language: Option<&str>,
        output_format: crate::types::OutputFormat,
    ) -> Result<BenchmarkResult> {
        let file_size = std::fs::metadata(file_path).map_err(Error::Io)?.len();

        let mut config = self.extraction_config_for(force_ocr, ocr_language);
        let cancel_token = xberg::cancellation::CancellationToken::new();
        config.cancel_token = Some(cancel_token.clone());

        let start = Instant::now();

        let extraction_start = Instant::now();

        let timed_result = tokio::time::timeout(timeout, extract_xberg_file(file_path, &config)).await;
        let timed_out = timed_result.is_err();
        let extraction_result = match timed_result {
            Ok(inner) => inner.map_err(|e| Error::Benchmark(format!("Extraction failed: {}", e))),
            Err(_) => {
                cancel_token.cancel();
                Err(Error::Timeout(format!("Extraction exceeded {:?}", timeout)))
            }
        };

        let extraction_duration = extraction_start.elapsed();
        let duration = start.elapsed();

        let resource_stats = ResourceStats::default();

        let throughput = if duration.as_secs_f64() > 0.0 {
            file_size as f64 / duration.as_secs_f64()
        } else {
            0.0
        };

        if let Err(e) = extraction_result {
            return Ok(single_failure_result(
                SingleResultContext {
                    framework: self.name(),
                    output_format,
                    file_path,
                    file_size,
                    duration,
                    extraction_duration,
                    resource_stats: &resource_stats,
                },
                &e,
                timed_out,
            ));
        }

        let extraction_result = extraction_result.unwrap();
        let ocr_status = determine_ocr_status(&extraction_result, &config);

        Ok(single_success_result(
            SingleResultContext {
                framework: self.name(),
                output_format,
                file_path,
                file_size,
                duration,
                extraction_duration,
                resource_stats: &resource_stats,
            },
            extraction_result,
            ocr_status,
            throughput,
        ))
    }

    async fn extract_batch(
        &self,
        file_paths: &[&Path],
        timeout: Duration,
        force_ocr: &[bool],
        ocr_languages: &[Option<String>],
        output_format: crate::types::OutputFormat,
    ) -> Result<Vec<BenchmarkResult>> {
        if file_paths.is_empty() {
            return Ok(Vec::new());
        }
        let mut config = self.config.clone();
        let cancel_token = xberg::cancellation::CancellationToken::new();
        config.cancel_token = Some(cancel_token.clone());
        let inputs = build_batch_inputs(file_paths, force_ocr, ocr_languages, &config)?;

        reject_zero_timeout(timeout, &cancel_token)?;

        let total_file_size = sum_file_sizes(file_paths);

        let monitor = ResourceMonitor::new();
        let sampling_interval_ms = Self::calculate_adaptive_sampling_interval(total_file_size);
        monitor.start(Duration::from_millis(sampling_interval_ms)).await;

        let start = Instant::now();

        let timed_result = tokio::time::timeout(timeout, self.engine.extract_batch(inputs, &config)).await;
        let timed_out = timed_result.is_err();
        // Keep the whole envelope: output.errors carries per-input failures (with
        // their original index) that must not be dropped, or successful results
        // would be misattributed to the wrong files when zipped positionally. ~keep
        let batch_result = match timed_result {
            Ok(inner) => inner.map_err(|e| Error::Benchmark(format!("Batch extraction failed: {}", e))),
            Err(_) => {
                cancel_token.cancel();
                Err(Error::Timeout(format!(
                    "Batch extraction exceeded {timeout:?}; retained Engine is no longer reusable"
                )))
            }
        };

        let total_duration = start.elapsed();

        let samples = monitor.stop().await;
        let snapshots = monitor.get_snapshots().await;
        let baseline = monitor.baseline_memory().await;
        let resource_stats = ResourceMonitor::calculate_stats(&samples, &snapshots, baseline);

        if timed_out {
            return Err(batch_result.expect_err("timed-out extraction must carry a timeout error"));
        }

        let num_files = file_paths.len() as f64;
        let avg_duration_per_file = Duration::from_secs_f64(total_duration.as_secs_f64() / num_files.max(1.0));

        let row_context = BatchRowContext {
            framework: self.name(),
            output_format,
            avg_duration_per_file,
            resource_stats: &resource_stats,
        };

        if let Err(e) = batch_result {
            let message = e.to_string();
            let failure_results: Vec<BenchmarkResult> = file_paths
                .iter()
                .map(|file_path| {
                    batch_failure_result(&row_context, file_path, message.clone(), ErrorKind::HarnessError)
                })
                .collect();
            return Ok(failure_results);
        }

        let output = batch_result.unwrap();
        let results = assemble_batch_rows(&row_context, file_paths, &output, &config)?;
        Ok(finalize_batch_rows(self, results, total_duration, &resource_stats))
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn executable_provenance(&self) -> Option<crate::provenance::ExecutableProvenance> {
        std::env::current_exe()
            .ok()
            .map(|path| crate::provenance::ExecutableProvenance::from_command(&path))
    }

    fn executable_build_identity(&self) -> Option<crate::adapter::ExecutableBuildIdentity> {
        std::env::current_exe()
            .ok()
            .map(|path| crate::adapter::ExecutableBuildIdentity {
                build_id: xberg::embedded_build_id().to_string(),
                path,
            })
    }

    fn configured_thread_budget(&self) -> Option<usize> {
        self.config.concurrency.as_ref().and_then(|value| value.max_threads)
    }

    async fn setup(&self) -> Result<()> {
        Ok(())
    }

    async fn teardown(&self) -> Result<()> {
        Ok(())
    }

    async fn warmup(
        &self,
        warmup_file: &Path,
        timeout: Duration,
        output_format: crate::types::OutputFormat,
    ) -> Result<Duration> {
        if let Some(cold_adapter) = &self.cold_adapter {
            return cold_adapter.warmup(warmup_file, timeout, output_format).await;
        }
        let start = Instant::now();
        let result = self.extract(warmup_file, timeout, false, None, output_format).await?;
        if !result.success {
            return Err(Error::Benchmark(
                result
                    .error_message
                    .unwrap_or_else(|| "native warmup failed".to_string()),
            ));
        }
        Ok(start.elapsed())
    }
}

/// Build one batch `ExtractInput`, applying per-file OCR overrides on top of the
/// batch `base` config.
///
/// Semantics that keep the batch honest:
/// - `force_ocr` is folded as `base.force_ocr || per_file` so a per-file `false`
///   never disables OCR the base forced on (mirrors the single-file `|=`).
/// - a per-file language clones the base `OcrConfig` and overrides only the
///   (canonicalized) `language`, so backend/pipeline/cache survive — a Paddle/VLM
///   backend is never silently turned into Tesseract.
/// - a file with neither a force flag nor a language gets no per-file config and
///   inherits the base unchanged (`FileExtractionConfig` fields default to "inherit").
fn build_batch_input(
    path: &Path,
    force_ocr: bool,
    ocr_language: Option<&str>,
    base: &ExtractionConfig,
) -> xberg::ExtractInput {
    let mut input = xberg::ExtractInput::from_uri(path.to_string_lossy());
    if force_ocr || ocr_language.is_some() {
        let mut file_config = xberg::FileExtractionConfig {
            force_ocr: Some(base.force_ocr || force_ocr),
            ..Default::default()
        };
        if let Some(language) = ocr_language {
            let mut ocr = base.ocr.clone().unwrap_or_default();
            apply_ocr_language(&mut ocr, language);
            file_config.ocr = Some(ocr);
        }
        input.config = Some(file_config);
    }
    input
}

#[cfg(test)]
#[path = "native/batch_tests.rs"]
mod batch_tests;

#[cfg(test)]
#[path = "native/input_tests.rs"]
mod input_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_native_adapter_creation() {
        let adapter = NativeAdapter::new();
        assert_eq!(adapter.name(), "xberg-rust-steady-state");
        assert_eq!(adapter.timing_regime(), TimingRegime::WarmInProcess);
        assert_eq!(adapter.batch_capability(), Some(native_batch_capability()));
        assert_eq!(
            adapter.supported_output_formats(),
            vec![
                crate::types::OutputFormat::Markdown,
                crate::types::OutputFormat::Plaintext,
            ]
        );
        assert_eq!(
            adapter.ocr_language_policy(),
            crate::adapter::OcrLanguagePolicy::AnyPerDocument
        );
        assert_eq!(
            adapter.executable_build_identity().unwrap().build_id,
            xberg::embedded_build_id()
        );
        assert_eq!(adapter.configured_thread_budget(), None);
    }

    #[test]
    fn configured_thread_budget_reports_embedded_config() {
        let mut config = ExtractionConfig::default();
        config.concurrency.get_or_insert_with(Default::default).max_threads = Some(3);
        let adapter = NativeAdapter::with_config(config);

        assert_eq!(adapter.configured_thread_budget(), Some(3));
    }

    #[test]
    fn steady_adapter_retains_a_separate_subprocess_cold_probe() {
        let cold = SubprocessAdapter::new(
            "xberg-cold",
            "/nonexistent/xberg",
            Vec::new(),
            Vec::new(),
            vec!["pdf".to_string()],
        );
        let adapter = NativeAdapter::with_identity_and_cold_adapter(
            "xberg-steady".to_string(),
            ExtractionConfig::default(),
            cold,
        );

        assert!(adapter.has_separate_cold_probe());
    }

    #[test]
    fn supported_formats_match_xberg_registry() {
        let adapter = NativeAdapter::new();
        let expected: std::collections::BTreeSet<String> = xberg::list_supported_formats()
            .into_iter()
            .map(|format| format.extension)
            .collect();
        let actual: std::collections::BTreeSet<String> =
            native_capabilities().supported_extensions.into_iter().collect();

        assert_eq!(actual, expected);
        assert!(expected.iter().all(|extension| adapter.supports_format(extension)));
        assert!(!adapter.supports_format("definitely-not-a-format"));
    }

    #[tokio::test]
    async fn test_supports_format() {
        let adapter = NativeAdapter::new();
        assert!(adapter.supports_format("pdf"));
        assert!(adapter.supports_format("docx"));
        assert!(adapter.supports_format("txt"));
        assert!(!adapter.supports_format("unknown"));
    }

    #[tokio::test]
    async fn test_extract_text_file() {
        let adapter = NativeAdapter::new();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "Hello, world!").unwrap();

        let result = adapter
            .extract(
                &file_path,
                Duration::from_secs(10),
                false,
                None,
                crate::types::OutputFormat::Plaintext,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.framework, "xberg-rust-steady-state");
        assert_eq!(result.framework_capabilities.timing_regime, TimingRegime::WarmInProcess);
        assert_eq!(
            result.framework_capabilities.resource_measurement_scope,
            ResourceMeasurementScope::HarnessProcessLatencyOnly
        );
        assert_eq!(result.metrics.peak_memory_bytes, 0);
        assert_eq!(result.metrics.cpu_seconds, 0.0);
        assert!(result.duration.as_millis() < 1000);
    }
}
