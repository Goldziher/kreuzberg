//! Single-file execution: building the subprocess command, mapping its output into a
//! `BenchmarkResult`, and the implementation `FrameworkAdapter::extract` delegates to.

use crate::types::{
    BenchmarkResult, ErrorKind, FrameworkCapabilities, OcrStatus, OutputFormat, PerformanceMetrics,
    ResourceMeasurementScope, TimingRegime,
};
use crate::{Error, Result};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use super::SubprocessAdapter;
use super::ocr_args::{apply_tesseract_ocr_override_to_args, xberg_ocr_language_args};
use super::support::{
    SubprocessExecution, bytes_per_second, detect_pdf_page_count, error_to_error_kind, is_debug_enabled,
};

impl SubprocessAdapter {
    /// Execute the extraction subprocess
    pub(super) async fn execute_subprocess(
        &self,
        file_path: &Path,
        timeout: Duration,
        force_ocr: bool,
        ocr_language: Option<&str>,
        output_format: OutputFormat,
    ) -> Result<SubprocessExecution> {
        let absolute_path = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            std::env::current_dir().map_err(Error::Io)?.join(file_path)
        };

        let mut cmd = Self::measured_command(&self.command);
        if let Some(dir) = &self.working_dir {
            cmd.current_dir(dir);
        }
        let mut request_args = self.single_file_request_args(force_ocr);
        let mut tesseract_ocr_rewritten = false;
        if self.name.starts_with("xberg-")
            && let Some(rewritten) = apply_tesseract_ocr_override_to_args(&request_args, ocr_language)
        {
            request_args = rewritten;
            tesseract_ocr_rewritten = true;
        }
        cmd.args(&request_args);
        if !tesseract_ocr_rewritten
            && self.name.starts_with("xberg-")
            && let Some(language_args) = xberg_ocr_language_args(&request_args, ocr_language)
        {
            cmd.args(language_args);
        }
        if let Some(forward) = self.ocr_language_forward_arg(ocr_language) {
            cmd.arg(forward);
        }

        if self.format_aware {
            cmd.arg(format!("--format={}", output_format));
        }

        cmd.arg(&*absolute_path.to_string_lossy());

        for (key, value) in &self.env {
            cmd.env(key, value);
        }

        Self::configure_measured_stdin(&mut cmd);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        Self::configure_child_process(&mut cmd);

        let sampling_ms =
            crate::monitoring::adaptive_sampling_interval_ms(std::fs::metadata(file_path).map_err(Error::Io)?.len());
        let measured =
            Self::execute_measured_command(&mut cmd, timeout, "subprocess", Duration::from_millis(sampling_ms)).await?;
        Ok(Self::finish_measured_command(measured, "Subprocess"))
    }

    /// Build a failure `BenchmarkResult` for error paths in `extract()`. ~keep
    ///
    /// Centralises the repeated pattern of constructing an error result with
    /// resource statistics, throughput, and framework capabilities.
    pub(super) fn build_failure_result(
        &self,
        file_path: &Path,
        file_size: u64,
        duration: Duration,
        resource_stats: &crate::monitoring::ResourceStats,
        error: &Error,
        output_format: OutputFormat,
    ) -> BenchmarkResult {
        let framework_capabilities = FrameworkCapabilities {
            timing_regime: TimingRegime::ColdProcess,
            resource_measurement_scope: ResourceMeasurementScope::IsolatedProcess,
            supported_extensions: self.supported_formats.clone(),
            ocr_support: Self::framework_supports_ocr(&self.name),
            batch_support: self.batch_capability.is_some(),
            batch_capability: self.batch_capability,
            batch_performance_sample: Some(true),
            ..Default::default()
        };

        let error_kind = error_to_error_kind(error);

        BenchmarkResult {
            framework: self.name.clone(),
            output_format,
            file_path: file_path.to_path_buf(),
            file_size,
            success: false,
            error_message: Some(error.to_string()),
            error_kind,
            duration,
            extraction_duration: None,
            subprocess_overhead: None,
            metrics: PerformanceMetrics {
                baseline_memory_bytes: resource_stats.baseline_memory_bytes,
                peak_memory_bytes: resource_stats.peak_memory_bytes,
                peak_memory_delta_bytes: resource_stats.peak_memory_delta_bytes,
                avg_cpu_percent: resource_stats.avg_cpu_percent,
                cpu_seconds: resource_stats.cpu_seconds,
                throughput_bytes_per_sec: 0.0,
                p50_memory_bytes: resource_stats.p50_memory_bytes,
                p95_memory_bytes: resource_stats.p95_memory_bytes,
                p99_memory_bytes: resource_stats.p99_memory_bytes,
            },
            quality: None,
            iterations: vec![],
            statistics: None,
            cold_start_duration: None,
            file_extension: file_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown")
                .to_lowercase(),
            framework_capabilities,
            pdf_metadata: None,
            ocr_status: OcrStatus::Unknown,
            extracted_text: None,
            system_load: None,
        }
    }

    /// Parse extraction result from subprocess output
    ///
    /// Expected subprocess output format:
    /// ```json
    /// {
    ///   "content": "extracted text...",          // REQUIRED
    ///   "_ocr_used": true|false,                 // optional
    ///   "_extraction_time_ms": 123.45            // optional
    /// }
    /// ```
    pub(super) fn parse_output(&self, stdout: &str) -> Result<serde_json::Value> {
        if is_debug_enabled() {
            let preview = if stdout.len() > 300 {
                let end = (0..=300).rev().find(|&i| stdout.is_char_boundary(i)).unwrap_or(0);
                format!("{}...[{} bytes total]", &stdout[..end], stdout.len())
            } else {
                stdout.to_string()
            };
            tracing::debug!(
                framework = %self.name,
                raw_len = stdout.len(),
                preview = %preview.trim(),
                "parsed subprocess output preview"
            );
        }

        let raw: serde_json::Value = serde_json::from_str(stdout)
            .map_err(|e| Error::Benchmark(format!("Failed to parse subprocess output as JSON: {}", e)))?;

        if !raw.is_object() {
            return Err(Error::Benchmark(
                "Subprocess output must be a JSON object with 'content' field".to_string(),
            ));
        }

        let parsed = if let Some(inner) = raw.get("result").filter(|v| v.is_object()) {
            let mut flat = inner.clone();
            if let (Some(obj), Some(t)) = (flat.as_object_mut(), raw.get("extraction_time_ms")) {
                obj.insert("_extraction_time_ms".to_string(), t.clone());
            }
            // Xberg self-reports peak RSS the same way every competitor wrapper does (see
            // `crates/xberg-cli/src/peak_memory.rs`). Surface it under the same
            // `_peak_memory_bytes` key the Python wrappers use so the memory comparison downstream
            // treats xberg identically to them instead of only ever trusting the sysinfo sampler
            // for xberg. ~keep
            if let (Some(obj), Some(mem)) = (flat.as_object_mut(), raw.get("peak_memory_bytes")) {
                obj.insert("_peak_memory_bytes".to_string(), mem.clone());
            }
            if let (Some(obj), Some(meta)) = (flat.as_object_mut(), inner.get("metadata"))
                && let Some(ocr) = meta.get("ocr_used")
            {
                obj.insert("_ocr_used".to_string(), ocr.clone());
            }
            flat
        } else {
            raw
        };

        if let Some(error_val) = parsed.get("error") {
            let error_msg = error_val.as_str().unwrap_or("unknown error");
            if !error_msg.is_empty() {
                if error_msg.contains("timed out") {
                    return Err(Error::Timeout(error_msg.to_string()));
                }
                return Err(Error::FrameworkError(error_msg.to_string()));
            }
        }

        if !parsed.get("content").is_some_and(|v| v.is_string()) {
            let extraction_time = parsed
                .get("_extraction_time_ms")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            if extraction_time == 0.0 {
                return Err(Error::EmptyContent(
                    "No content extracted (unsupported format or empty result)".to_string(),
                ));
            }
            return Err(Error::Benchmark(
                "Subprocess output missing required 'content' field (must be a string)".to_string(),
            ));
        }

        let content_str = parsed["content"].as_str().unwrap();
        if content_str.trim().is_empty() {
            return Err(Error::EmptyContent("Framework returned empty content".to_string()));
        }

        Ok(parsed)
    }

    /// Run the subprocess, classify any harness/framework failure into a failure
    /// `BenchmarkResult`, and parse the successful JSON payload. Isolates the three
    /// early-return failure paths from `extract`'s happy-path result assembly. ~keep
    async fn execute_and_parse(
        &self,
        file_path: &Path,
        timeout: Duration,
        force_ocr: bool,
        ocr_language: Option<&str>,
        output_format: OutputFormat,
        file_size: u64,
    ) -> std::result::Result<(serde_json::Value, Duration, crate::monitoring::ResourceStats), Box<BenchmarkResult>>
    {
        let start_time = std::time::Instant::now();

        let execution = match self
            .execute_subprocess(file_path, timeout, force_ocr, ocr_language, output_format)
            .await
        {
            Ok(result) => result,
            Err(e) => {
                let actual_duration = start_time.elapsed();
                return Err(Box::new(self.build_failure_result(
                    file_path,
                    file_size,
                    actual_duration,
                    &crate::monitoring::ResourceStats::default(),
                    &e,
                    output_format,
                )));
            }
        };
        let SubprocessExecution {
            stdout,
            duration,
            resource_stats,
            error,
            ..
        } = execution;
        if let Some(error) = error {
            return Err(Box::new(self.build_failure_result(
                file_path,
                file_size,
                duration,
                &resource_stats,
                &error,
                output_format,
            )));
        }

        let parsed = match self.parse_output(&stdout) {
            Ok(value) => value,
            Err(e) => {
                return Err(Box::new(self.build_failure_result(
                    file_path,
                    file_size,
                    duration,
                    &resource_stats,
                    &e,
                    output_format,
                )));
            }
        };

        Ok((parsed, duration, resource_stats))
    }

    /// Merge the framework's self-reported peak RSS (when present and not lower than what the
    /// harness's own sampler observed) into the measured metrics. Xberg self-reports peak RSS
    /// precisely; a self-report lower than the sampler's observation is treated as unreliable
    /// and the sampled metrics are kept instead. ~keep
    fn merge_reported_metrics(
        resource_stats: &crate::monitoring::ResourceStats,
        self_reported_memory: Option<u64>,
        throughput: f64,
    ) -> PerformanceMetrics {
        match self_reported_memory {
            Some(reported_mem) if reported_mem >= resource_stats.peak_memory_bytes => PerformanceMetrics {
                baseline_memory_bytes: resource_stats.baseline_memory_bytes,
                peak_memory_bytes: reported_mem,
                peak_memory_delta_bytes: reported_mem.saturating_sub(resource_stats.baseline_memory_bytes),
                avg_cpu_percent: resource_stats.avg_cpu_percent,
                cpu_seconds: resource_stats.cpu_seconds,
                throughput_bytes_per_sec: throughput,
                p50_memory_bytes: reported_mem,
                p95_memory_bytes: reported_mem,
                p99_memory_bytes: reported_mem,
            },
            _ => PerformanceMetrics {
                baseline_memory_bytes: resource_stats.baseline_memory_bytes,
                peak_memory_bytes: resource_stats.peak_memory_bytes,
                peak_memory_delta_bytes: resource_stats.peak_memory_delta_bytes,
                avg_cpu_percent: resource_stats.avg_cpu_percent,
                cpu_seconds: resource_stats.cpu_seconds,
                throughput_bytes_per_sec: throughput,
                p50_memory_bytes: resource_stats.p50_memory_bytes,
                p95_memory_bytes: resource_stats.p95_memory_bytes,
                p99_memory_bytes: resource_stats.p99_memory_bytes,
            },
        }
    }

    /// Build the framework-capabilities and (PDF-only) page-count metadata attached to a
    /// successful `extract()` result. ~keep
    fn build_capabilities_and_pdf_metadata(
        &self,
        file_path: &Path,
        ocr_status: OcrStatus,
    ) -> (FrameworkCapabilities, Option<crate::types::PdfMetadata>) {
        let framework_capabilities = FrameworkCapabilities {
            timing_regime: TimingRegime::ColdProcess,
            resource_measurement_scope: ResourceMeasurementScope::IsolatedProcess,
            supported_extensions: self.supported_formats.clone(),
            ocr_support: Self::framework_supports_ocr(&self.name),
            batch_support: self.batch_capability.is_some(),
            batch_capability: self.batch_capability,
            batch_performance_sample: Some(true),
            ..Default::default()
        };

        let pdf_metadata = if file_path.extension().and_then(|e| e.to_str()) == Some("pdf") {
            Some(crate::types::PdfMetadata {
                has_text_layer: false,
                detection_method: "unknown".to_string(),
                page_count: detect_pdf_page_count(file_path),
                ocr_enabled: ocr_status == OcrStatus::Used,
                text_quality_score: None,
            })
        } else {
            None
        };

        (framework_capabilities, pdf_metadata)
    }

    /// Implements `FrameworkAdapter::extract`; the trait method itself lives in
    /// `adapter_impl.rs` (a type may have only one trait impl block per crate) and
    /// delegates here so the single-file execution logic stays with its helpers. ~keep
    pub(super) async fn extract_impl(
        &self,
        file_path: &Path,
        timeout: Duration,
        force_ocr: bool,
        ocr_language: Option<&str>,
        output_format: OutputFormat,
    ) -> Result<BenchmarkResult> {
        let timeout = self.effective_timeout(timeout);
        let file_size = std::fs::metadata(file_path).map_err(Error::Io)?.len();

        let (parsed, duration, resource_stats) = match self
            .execute_and_parse(file_path, timeout, force_ocr, ocr_language, output_format, file_size)
            .await
        {
            Ok(value) => value,
            Err(failure) => return Ok(*failure),
        };

        let extraction_time_raw = parsed.get("_extraction_time_ms");
        if is_debug_enabled() {
            tracing::debug!(
                framework = %self.name,
                extraction_time_ms = ?extraction_time_raw,
                keys = ?parsed.as_object().map(|object| object.keys().collect::<Vec<_>>()),
                "parsed subprocess extraction metadata"
            );
        }

        let extraction_duration = extraction_time_raw
            .and_then(|v| v.as_f64())
            .map(|ms| Duration::from_secs_f64(ms / 1000.0));

        let extracted_text = parsed.get("content").and_then(|v| v.as_str()).map(|s| s.to_string());

        let subprocess_overhead = extraction_duration.map(|ext| duration.saturating_sub(ext));

        let throughput = bytes_per_second(file_size, duration);

        let self_reported_memory = parsed.get("_peak_memory_bytes").and_then(|v| v.as_u64());

        let metrics = Self::merge_reported_metrics(&resource_stats, self_reported_memory, throughput);

        let ocr_status = self.resolve_ocr_status(parsed.get("_ocr_used"), force_ocr);

        let (framework_capabilities, pdf_metadata) = self.build_capabilities_and_pdf_metadata(file_path, ocr_status);

        Ok(BenchmarkResult {
            framework: self.name.clone(),
            output_format,
            file_path: file_path.to_path_buf(),
            file_size,
            success: true,
            error_message: None,
            error_kind: ErrorKind::None,
            duration,
            extraction_duration,
            subprocess_overhead,
            metrics,
            quality: None,
            iterations: vec![],
            statistics: None,
            cold_start_duration: None,
            file_extension: file_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown")
                .to_lowercase(),
            framework_capabilities,
            pdf_metadata,
            ocr_status,
            extracted_text,
            system_load: None,
        })
    }
}
