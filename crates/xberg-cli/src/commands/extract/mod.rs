//! Extract command - Extract text and data from documents
//!
//! This module provides the extract and batch extract commands for processing single
//! or multiple documents with customizable extraction configurations.
//!
//! The implementation is split by responsibility across submodules:
//! - `timing` - optional per-stage cold-start timing for `--format json`
//! - `images` - writing extracted images to disk
//! - `manifest` - batch input manifest parsing and per-file config resolution
//! - `runtime` - tokio runtime sizing and construction for extraction work
//! - `batch` - the batch extraction command and its result aggregation

use anyhow::{Context, Result};
use base64::Engine as _;
use std::io::{IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, ExtractionErrorItem, ExtractionResult, OutputFormat};

use crate::{
    WireFormat,
    output::{ExtractEnvelope, write_text_envelope},
};

mod batch;
mod images;
mod manifest;
mod runtime;
mod timing;

pub use batch::batch_command;
pub use manifest::{BatchInputFormat, load_batch_input_manifest};
// `RUNTIME_WORKER_STACK_SIZE_BYTES`'s only external reader is `server.rs`, which is
// `#[cfg(any(feature = "api", feature = "mcp"))]`; under a feature set with neither, this
// re-export is legitimately unused even though the underlying const in `runtime.rs` is not
// (it is also read locally there). Keeping the re-export preserves the stable
// `commands::extract::RUNTIME_WORKER_STACK_SIZE_BYTES` path for builds that do enable api/mcp. ~keep
#[allow(unused_imports)]
pub(crate) use runtime::RUNTIME_WORKER_STACK_SIZE_BYTES;
// `STAGE_TIMING_ENV_VAR` is part of this module's public path for external documentation/tooling
// (see `crate::output`'s doc comment) but has no in-crate reader outside `timing.rs` itself;
// `stage_timing_requested` (re-exported alongside it) is always read from within this module, so
// only the constant needs the allow. ~keep
#[allow(unused_imports)]
pub use timing::STAGE_TIMING_ENV_VAR;
pub use timing::stage_timing_requested;

use images::write_extracted_images;
use runtime::block_on_extract;
use timing::build_stage_timings;

/// The library's DOCX renderer name, and the `metadata.output_format` of a result whose
/// `content` is a base64-encoded `.docx` package.
pub(crate) const DOCX_CONTENT_FORMAT: &str = "docx";

/// The library's PDF renderer name, and the `metadata.output_format` of a result whose
/// `content` is a base64-encoded PDF file.
pub(crate) const PDF_CONTENT_FORMAT: &str = "pdf";

/// The binary document format `config` asks for, if any. Each produces one binary
/// document rather than text.
pub(crate) fn requested_binary_format(config: &ExtractionConfig) -> Option<&str> {
    match &config.output_format {
        OutputFormat::Custom(name) if name == DOCX_CONTENT_FORMAT || name == PDF_CONTENT_FORMAT => Some(name),
        _ => None,
    }
}

/// Input source for single-document extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractInputSource {
    /// Local path or URI string.
    Uri(String),
    /// Bytes read from stdin.
    Stdin,
}

/// Where a single-document extraction writes its extracted images and its output document.
#[derive(Debug, Clone, Default)]
pub struct ExtractDestinations {
    /// Directory the extracted images are written to; the current directory when `None`.
    pub output_dir: Option<PathBuf>,
    /// File the extracted document is written to instead of stdout.
    pub output: Option<PathBuf>,
}

/// Execute single document extraction command.
///
/// `process_start` is the [`Instant`] captured as early as feasible in `main()`. It is used only
/// to compute `process_init_ms` for the optional stage-timing breakdown (see
/// [`stage_timing_requested`]); pass `None` to skip that measurement entirely (e.g. from tests
/// that construct this call directly).
#[expect(
    clippy::print_stdout,
    reason = "extracted content and JSON/TOON envelope are the command's stdout result output"
)]
pub fn extract_command(
    input: ExtractInputSource,
    config: ExtractionConfig,
    mime_type: Option<String>,
    format: WireFormat,
    destinations: ExtractDestinations,
    process_start: Option<Instant>,
) -> Result<()> {
    let ExtractDestinations { output_dir, output } = destinations;
    let emit_stage_timing = stage_timing_requested();

    refuse_binary_output_to_terminal(&config, &format, output.is_some())?;

    let t0 = Instant::now();
    let result = extract_input_sync(input, mime_type.as_deref(), &config)?;
    let elapsed = t0.elapsed();
    let extraction_time_ms = elapsed.as_secs_f64() * 1000.0;

    let stage_timings = emit_stage_timing.then(|| build_stage_timings(process_start, t0, extraction_time_ms, &config));

    if let Some(path) = &output {
        write_document_file(&result, requested_binary_format(&config), path)?;
    }

    match format {
        WireFormat::Text => {
            write_result_images(&result, output_dir.as_deref())?;
            let written = if output.is_some() {
                Ok(())
            } else if let Some(binary_format) = requested_binary_format(&config) {
                write_binary_document(&result, binary_format)
            } else {
                print!("{}", result.content);
                Ok(())
            };
            // `stdout` stays exactly the extracted content so it remains pipeable; everything
            // else the extraction produced — warnings included — goes to `stderr`.
            let mut diagnostics = std::io::stderr().lock();
            write_text_envelope(&result, extraction_time_ms, &mut diagnostics)
                .context("Failed to write the extraction envelope summary")?;
            written?;
        }
        WireFormat::Json => {
            // `getrusage` reports the peak for the process's whole lifetime, so the exact sample
            // point doesn't matter as long as it's after the work being measured.
            let peak_memory_bytes = crate::peak_memory::peak_memory_bytes().unwrap_or(0);
            let envelope = ExtractEnvelope {
                result,
                extraction_time_ms,
                peak_memory_bytes,
                stage_timings,
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&envelope).context("Failed to serialize extraction result to JSON")?
            );
        }
        WireFormat::Toon => {
            write_result_images(&result, output_dir.as_deref())?;
            // Serialize the same envelope the JSON path emits. Previously this serialized the
            // bare `ExtractedDocument`, so TOON consumers lost the timing/peak-memory fields
            // that JSON consumers get.
            let peak_memory_bytes = crate::peak_memory::peak_memory_bytes().unwrap_or(0);
            let envelope = ExtractEnvelope {
                result,
                extraction_time_ms,
                peak_memory_bytes,
                stage_timings,
            };
            println!(
                "{}",
                serde_toon::to_string(&envelope).context("Failed to serialize extraction result to TOON")?
            );
        }
    }

    Ok(())
}

fn write_result_images(result: &ExtractedDocument, output_dir: Option<&Path>) -> Result<()> {
    if let Some(images) = &result.images {
        write_extracted_images(images, output_dir.unwrap_or(Path::new(".")))?;
    }
    Ok(())
}

/// Refuse binary output to a terminal before extracting anything.
fn refuse_binary_output_to_terminal(config: &ExtractionConfig, format: &WireFormat, writes_file: bool) -> Result<()> {
    if let Some(binary_format) = requested_binary_format(config)
        && matches!(format, WireFormat::Text)
        && !writes_file
        && std::io::stdout().is_terminal()
    {
        anyhow::bail!(
            "--content-format {binary_format} writes a binary document to stdout; redirect it to a file (for \
             example `> output.{binary_format}`), or use --format json to receive it base64-encoded in `content`"
        );
    }
    Ok(())
}

/// Write the document a DOCX or PDF extraction carries base64-encoded in `content`.
///
/// Fails rather than writing text when the library fell back to plain text (for example
/// when it was built without the feature that renders `binary_format`), since the caller
/// is redirecting stdout into a binary file.
fn write_binary_document(result: &ExtractedDocument, binary_format: &str) -> Result<()> {
    let document = document_bytes(result, Some(binary_format))?;
    let label = binary_format.to_uppercase();
    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(&document)
        .with_context(|| format!("Failed to write the {label} document to stdout"))?;
    stdout
        .flush()
        .with_context(|| format!("Failed to write the {label} document to stdout"))
}

pub(super) fn write_document_file(result: &ExtractedDocument, binary_format: Option<&str>, path: &Path) -> Result<()> {
    let bytes = document_bytes(result, binary_format)?;
    if path.exists() {
        anyhow::bail!("Output file already exists: '{}'", path.display());
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("Failed to create output file '{}'", path.display()))?;
    file.write_all(&bytes)
        .with_context(|| format!("Failed to write output file '{}'", path.display()))
}

fn document_bytes(result: &ExtractedDocument, binary_format: Option<&str>) -> Result<Vec<u8>> {
    let Some(binary_format) = binary_format else {
        return Ok(result.content.as_bytes().to_vec());
    };
    let label = binary_format.to_uppercase();
    if result.metadata.output_format.as_deref() != Some(binary_format) {
        anyhow::bail!(
            "{label} output was requested but the extraction produced {} text instead; see the warnings above",
            result.metadata.output_format.as_deref().unwrap_or("plain")
        );
    }
    base64::engine::general_purpose::STANDARD
        .decode(&result.content)
        .with_context(|| format!("{label} output was not valid base64"))
}

fn extract_input_sync(
    input: ExtractInputSource,
    mime_type: Option<&str>,
    config: &ExtractionConfig,
) -> Result<ExtractedDocument> {
    let output = match input {
        ExtractInputSource::Uri(uri) => {
            let mut input = ExtractInput::from_uri(uri.clone());
            input.mime_type = mime_type.map(str::to_string);
            // Describe what was attempted, not why it failed: `.context()` preserves the
            // underlying error as this error's source (anyhow's `{:?}` rendering walks it via
            // "Caused by:"), so asserting a specific diagnosis here ("ensure the resource is
            // readable and the format is supported") would misreport failures that have nothing
            // to do with readability or format support — an OCR backend crash, for example.
            block_on_extract(input, config).with_context(|| format!("Failed to extract input '{uri}'"))?
        }
        ExtractInputSource::Stdin => {
            let mime_type = mime_type.unwrap_or("text/plain");
            let mut data = Vec::new();
            std::io::stdin()
                .read_to_end(&mut data)
                .context("Failed to read extraction input from stdin")?;
            if data.is_empty() {
                anyhow::bail!("No input received from stdin.");
            }
            // See the URI branch above: describe the attempted operation only. The real cause
            // (e.g. a genuinely wrong --mime-type) still surfaces via the preserved error chain.
            block_on_extract(ExtractInput::from_bytes(data, mime_type, None), config)
                .with_context(|| format!("Failed to extract stdin input as MIME type '{mime_type}'"))?
        }
    };
    single_result_from_output(output)
}

pub fn uri_to_local_path(uri: &str) -> Result<PathBuf> {
    if uri.starts_with("http://") || uri.starts_with("https://") {
        anyhow::bail!("Cannot convert HTTP(S) URL '{uri}' to a local filesystem path.");
    }

    Ok(PathBuf::from(uri.strip_prefix("file://").unwrap_or(uri)))
}

fn single_result_from_output(mut output: ExtractionResult) -> Result<ExtractedDocument> {
    fail_if_errors(&output.errors)?;
    if output.results.len() != 1 {
        anyhow::bail!("Expected one extraction result, got {}.", output.results.len());
    }
    Ok(output.results.remove(0))
}

fn fail_if_errors(errors: &[ExtractionErrorItem]) -> Result<()> {
    if let Some(error) = errors.first() {
        anyhow::bail!(
            "Extraction failed for input {} ({}): {}",
            error.index,
            error.source,
            error.message
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uri_to_local_path_strips_file_scheme() {
        assert_eq!(
            uri_to_local_path("file:///tmp/doc.txt").unwrap(),
            PathBuf::from("/tmp/doc.txt")
        );
    }

    /// Regression test for a real reported bug: extraction failures were wrapped in a context
    /// message ("Ensure the resource is readable and the format is supported.") that confidently
    /// diagnoses the wrong cause whenever the actual failure has nothing to do with readability
    /// or format support (e.g. an OCR backend crash on a file three other backends had just
    /// extracted successfully in the same session). anyhow's `.context()`/`.with_context()`
    /// preserve the underlying error as this error's `source()`, so the real cause was always
    /// present in the chain -- but the outermost line a user reads asserted a specific, false
    /// diagnosis instead of describing what was attempted. This exercises the real (unmocked)
    /// `extract_input_sync` failure path with a nonexistent file, which `xberg::extract` reports
    /// as `XbergError::Io` or `XbergError::Validation` (see
    /// `crates/xberg/tests/error_handling.rs::test_nonexistent_file`), to confirm the CLI's own
    /// wrapping context no longer asserts that false diagnosis.
    #[test]
    fn uri_extraction_failure_does_not_assert_a_false_readability_diagnosis() {
        let config = ExtractionConfig::default();
        let missing_uri = "test_documents_definitely_missing/does-not-exist-9f3c2a11.txt";

        let err = extract_input_sync(ExtractInputSource::Uri(missing_uri.to_string()), None, &config)
            .expect_err("extracting a nonexistent file must fail");
        let rendered = format!("{err:?}");

        assert!(
            !rendered.contains("Ensure the resource is readable and the format is supported"),
            "context must describe the attempted operation, not assert a (possibly false) \
             diagnosis; got: {rendered}"
        );
        assert!(
            rendered.contains(missing_uri),
            "context must name the input that failed to extract so the user knows what was \
             attempted; got: {rendered}"
        );
    }
}
