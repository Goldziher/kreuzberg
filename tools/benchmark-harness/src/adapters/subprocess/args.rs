//! Command-line argument construction for single-file and native-batch subprocess
//! invocations: xberg thread/worker budgets, liteparse batch args, and the
//! per-invocation batch sample id.

use crate::types::{BatchEntryPoint, OcrStatus, OutputFormat};
use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::Duration;

use super::SubprocessAdapter;

impl SubprocessAdapter {
    pub(super) fn is_xberg(&self) -> bool {
        self.name.starts_with("xberg-")
    }
    pub(super) fn append_explicit_xberg_thread_budget(&self, args: &mut Vec<String>) {
        if self.is_xberg()
            && let Some(max_threads) = self.xberg_max_threads
        {
            args.extend(["--max-threads".to_string(), max_threads.to_string()]);
        }
    }
    pub(super) fn single_file_request_args(&self, force_ocr: bool) -> Vec<String> {
        let mut args = self.request_args_from(self.single_file_args.as_deref().unwrap_or(&self.args), force_ocr);
        self.append_explicit_xberg_thread_budget(&mut args);
        args
    }
    pub(super) fn provenance_args_for_mode(&self, mode: crate::config::BenchmarkMode) -> Vec<String> {
        let mut args = match mode {
            crate::config::BenchmarkMode::SingleFile => self.single_file_args.as_deref().unwrap_or(&self.args).to_vec(),
            crate::config::BenchmarkMode::Batch => self.args.clone(),
        };
        match mode {
            crate::config::BenchmarkMode::SingleFile => self.append_explicit_xberg_thread_budget(&mut args),
            crate::config::BenchmarkMode::Batch
                if self.is_xberg()
                    && self
                        .batch_capability
                        .is_some_and(|capability| capability.entry_point == BatchEntryPoint::XbergCliExtractBatch) =>
            {
                args.extend([
                    "--max-concurrent".to_string(),
                    self.batch_workers.to_string(),
                    "--max-threads".to_string(),
                    self.effective_xberg_max_threads().to_string(),
                ]);
            }
            crate::config::BenchmarkMode::Batch => {}
        }
        args
    }
    pub(super) fn effective_xberg_max_threads(&self) -> usize {
        self.xberg_max_threads.unwrap_or(self.batch_workers)
    }
    pub(super) fn liteparse_batch_command(&self) -> &Path {
        self.native_batch_command.as_deref().unwrap_or_else(|| Path::new("lit"))
    }
    pub(super) fn liteparse_batch_args(
        &self,
        input_dir: impl Into<String>,
        output_dir: impl Into<String>,
        output_format: impl Into<String>,
        disable_ocr: bool,
    ) -> Vec<String> {
        let mut args = vec![
            "batch-parse".to_string(),
            input_dir.into(),
            output_dir.into(),
            "--format".to_string(),
            output_format.into(),
            "--num-workers".to_string(),
            self.batch_workers.to_string(),
            "--quiet".to_string(),
        ];
        if disable_ocr {
            args.push("--no-ocr".to_string());
        }
        args
    }
    pub(super) fn batch_sample_id(&self, file_paths: &[&Path], force_ocr: bool, output_format: OutputFormat) -> String {
        let mut hasher = blake3::Hasher::new();
        let sequence = self.batch_sequence.fetch_add(1, Ordering::Relaxed);
        let invocation_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        hasher.update(&std::process::id().to_le_bytes());
        hasher.update(&(std::ptr::from_ref(self).addr() as u64).to_le_bytes());
        hasher.update(&sequence.to_le_bytes());
        hasher.update(&invocation_time.to_le_bytes());
        hasher.update(&(self.batch_workers as u64).to_le_bytes());
        hasher.update(&(self.effective_xberg_max_threads() as u64).to_le_bytes());
        let output_format = output_format.to_string();
        let ocr_mode: &[u8] = match (force_ocr, self.configured_ocr_status) {
            (true, _) => b"force-ocr",
            (false, Some(OcrStatus::Used)) => b"configured-ocr-enabled",
            (false, Some(OcrStatus::NotUsed)) => b"configured-ocr-disabled",
            (false, _) => b"framework-reported-ocr",
        };
        let entry_point: &[u8] = match self.batch_capability.map(|capability| capability.entry_point) {
            Some(BatchEntryPoint::XbergCliExtractBatch) => b"xberg-cli-extract-batch",
            Some(BatchEntryPoint::XbergRustEngineExtractBatch) => b"xberg-rust-engine-extract-batch",
            Some(BatchEntryPoint::DoclingJobkit) => b"docling-jobkit",
            Some(BatchEntryPoint::LiteparseBatchParse) => b"liteparse-batch-parse",
            Some(BatchEntryPoint::MineruDoParse) => b"mineru-do-parse",
            Some(BatchEntryPoint::MineruParser) => b"mineru-parser",
            None => b"unverified",
        };
        for value in [self.name.as_bytes(), entry_point, output_format.as_bytes(), ocr_mode] {
            hasher.update(&(value.len() as u64).to_le_bytes());
            hasher.update(value);
        }
        for path in file_paths {
            let value = path.as_os_str().as_encoded_bytes();
            hasher.update(&(value.len() as u64).to_le_bytes());
            hasher.update(value);
        }
        hasher.finalize().to_hex().to_string()
    }
    /// Get the effective timeout, clamped by the adapter's max_timeout if set.
    pub(super) fn effective_timeout(&self, timeout: Duration) -> Duration {
        match self.max_timeout {
            Some(max) => timeout.min(max),
            None => timeout,
        }
    }
}
