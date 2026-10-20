//! The remaining small `FrameworkAdapter` trait method implementations: identity/format
//! queries, provenance reporting, worker-budget resolution, version probing, and the
//! no-op setup/teardown hooks.

use crate::adapter::FrameworkAdapter;
use crate::types::{
    BatchCapability, BatchEntryPoint, BenchmarkResult, OutputFormat, ResourceMeasurementScope, TimingRegime,
};
use crate::{Error, Result};
use async_trait::async_trait;
use std::path::Path;
use std::time::Duration;

use super::SubprocessAdapter;

const DOCLING_VERSION_PROBE: &str = r#"
import importlib.metadata as metadata
import sys

version = None
for distribution in ("docling", "docling-slim"):
    try:
        candidate = metadata.version(distribution).strip()
    except metadata.PackageNotFoundError:
        continue
    if candidate:
        version = candidate
        break

if version is None:
    try:
        import docling
    except ImportError:
        candidate = ""
    else:
        module_version = getattr(docling, "__version__", None)
        candidate = str(module_version).strip() if module_version is not None else ""
    if candidate:
        version = candidate

if version is None:
    sys.exit(1)

print(version)
"#;

const MINERU_VERSION_PROBE: &str = r#"
from importlib.metadata import PackageNotFoundError, version

try:
    print(version("mineru"))
except PackageNotFoundError:
    raise SystemExit(1)
"#;

fn first_output_line(output: std::process::Output) -> Option<String> {
    output
        .status
        .success()
        .then_some(output.stdout)
        .and_then(|stdout| String::from_utf8(stdout).ok())
        .and_then(|value| value.lines().next().map(str::trim).map(str::to_string))
        .filter(|value| !value.is_empty())
}

#[async_trait]
impl FrameworkAdapter for SubprocessAdapter {
    fn name(&self) -> &str {
        &self.name
    }
    fn timing_regime(&self) -> TimingRegime {
        TimingRegime::ColdProcess
    }
    fn resource_measurement_scope(&self) -> ResourceMeasurementScope {
        ResourceMeasurementScope::IsolatedProcess
    }
    fn supports_format(&self, file_type: &str) -> bool {
        let file_type_lower = file_type.to_lowercase();
        self.supported_formats
            .iter()
            .any(|fmt| fmt.to_lowercase() == file_type_lower)
    }
    fn should_skip_file(&self, file_name: &str) -> bool {
        self.skip_files.iter().any(|f| f == file_name)
    }
    fn supported_output_formats(&self) -> Vec<OutputFormat> {
        self.supported_output_formats.clone()
    }
    fn ocr_language_policy(&self) -> crate::adapter::OcrLanguagePolicy {
        self.ocr_language_policy
    }
    fn executable_provenance(&self) -> Option<crate::provenance::ExecutableProvenance> {
        self.executable_provenance_for_mode(crate::config::BenchmarkMode::Batch)
    }
    fn executable_provenance_for_mode(
        &self,
        mode: crate::config::BenchmarkMode,
    ) -> Option<crate::provenance::ExecutableProvenance> {
        if self.batch_capability.is_some_and(|capability| {
            mode == crate::config::BenchmarkMode::Batch
                && capability.entry_point == crate::types::BatchEntryPoint::LiteparseBatchParse
        }) {
            let args = self.liteparse_batch_args(
                "<input-dir>",
                "<output-dir>",
                "<output-format>",
                self.args.iter().any(|arg| arg == "--no-ocr"),
            );
            return Some(crate::provenance::ExecutableProvenance::from_invocation(
                self.liteparse_batch_command(),
                &args,
            ));
        }
        let args = self.provenance_args_for_mode(mode);
        Some(crate::provenance::ExecutableProvenance::from_invocation(
            &self.command,
            &args,
        ))
    }
    fn executable_build_identity(&self) -> Option<crate::adapter::ExecutableBuildIdentity> {
        self.executable_build_identity.clone()
    }
    fn worker_provenance(&self, requested: usize) -> (Option<usize>, Option<usize>) {
        match self.batch_capability.map(|capability| capability.entry_point) {
            Some(crate::types::BatchEntryPoint::DoclingJobkit) => (None, None),
            Some(crate::types::BatchEntryPoint::MineruDoParse) => (None, None),
            Some(crate::types::BatchEntryPoint::MineruParser) => (None, Some(1)),
            Some(crate::types::BatchEntryPoint::XbergCliExtractBatch) => (Some(requested), None),
            Some(crate::types::BatchEntryPoint::XbergRustEngineExtractBatch) => (Some(requested), None),
            Some(crate::types::BatchEntryPoint::LiteparseBatchParse) => (Some(requested), Some(self.batch_workers)),
            None => (Some(requested), Some(requested)),
        }
    }
    fn configured_thread_budget(&self) -> Option<usize> {
        if !self.is_xberg() {
            return None;
        }
        self.xberg_max_threads.or_else(|| {
            self.batch_capability
                .is_some_and(|capability| capability.entry_point == BatchEntryPoint::XbergCliExtractBatch)
                .then(|| self.effective_xberg_max_threads())
        })
    }

    fn version(&self) -> String {
        let output = match self.batch_capability.map(|capability| capability.entry_point) {
            Some(crate::types::BatchEntryPoint::DoclingJobkit) => std::process::Command::new(&self.command)
                .args(["-c", DOCLING_VERSION_PROBE])
                .envs(self.env.iter().map(|(key, value)| (key, value)))
                .output(),
            Some(crate::types::BatchEntryPoint::LiteparseBatchParse) => {
                std::process::Command::new(self.liteparse_batch_command())
                    .arg("--version")
                    .output()
            }
            Some(crate::types::BatchEntryPoint::MineruDoParse | crate::types::BatchEntryPoint::MineruParser) => {
                std::process::Command::new(&self.command)
                    .args(["-c", MINERU_VERSION_PROBE])
                    .envs(self.env.iter().map(|(key, value)| (key, value)))
                    .output()
            }
            _ => std::process::Command::new(&self.command).arg("--version").output(),
        };
        output
            .ok()
            .and_then(first_output_line)
            .unwrap_or_else(|| "unknown".to_string())
    }

    fn batch_capability(&self) -> Option<BatchCapability> {
        self.batch_capability
    }

    async fn extract(
        &self,
        file_path: &Path,
        timeout: Duration,
        force_ocr: bool,
        ocr_language: Option<&str>,
        output_format: OutputFormat,
    ) -> Result<BenchmarkResult> {
        self.extract_impl(file_path, timeout, force_ocr, ocr_language, output_format)
            .await
    }

    async fn extract_batch(
        &self,
        file_paths: &[&Path],
        timeout: Duration,
        force_ocr: &[bool],
        ocr_languages: &[Option<String>],
        output_format: OutputFormat,
    ) -> Result<Vec<BenchmarkResult>> {
        self.extract_batch_impl(file_paths, timeout, force_ocr, ocr_languages, output_format)
            .await
    }

    async fn setup(&self) -> Result<()> {
        which::which(&self.command)
            .map_err(|e| Error::Benchmark(format!("Command '{}' not found: {}", self.command.display(), e)))?;
        Ok(())
    }
    async fn teardown(&self) -> Result<()> {
        Ok(())
    }
}
