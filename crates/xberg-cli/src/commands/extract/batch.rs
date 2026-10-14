//! The batch extraction command and its result/timing aggregation.

use anyhow::{Context, Result};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, ExtractionErrorItem, ExtractionResult};

use crate::{
    WireFormat,
    output::{BatchEnvelope, write_processing_warnings},
    style,
};

use super::images::write_extracted_images;
use super::manifest::build_batch_inputs;
use super::runtime::block_on_extract_batch;

/// Execute batch extraction command with optional per-file configuration overrides
pub fn batch_command(
    uris: Vec<String>,
    file_configs_map: Option<std::collections::HashMap<String, serde_json::Value>>,
    config: ExtractionConfig,
    format: WireFormat,
    output_dir: Option<PathBuf>,
    content_output_dir: Option<PathBuf>,
) -> Result<()> {
    match format {
        WireFormat::Json => {
            let total_t0 = Instant::now();

            let inputs = build_batch_inputs(&uris, file_configs_map.as_ref())?;
            let (output, per_file_ms) = run_json_batch_sync(inputs, &config)?;
            let total_ms = total_t0.elapsed().as_secs_f64() * 1000.0;
            if let Some(dir) = &content_output_dir {
                write_batch_documents(&uris, &output.results, &config, dir)?;
            }
            let envelope = BatchEnvelope {
                results: output.results,
                total_ms,
                per_file_ms,
                errors: output.errors,
            };
            let stdout = std::io::stdout();
            let mut stdout = stdout.lock();
            serde_json::to_writer_pretty(&mut stdout, &envelope)
                .context("Failed to serialize batch extraction results to JSON")?;
            writeln!(stdout).context("Failed to write batch extraction results to stdout")?;
            let mut diagnostics = std::io::stderr().lock();
            write_batch_errors(&envelope.errors, &mut diagnostics).context("Failed to write batch errors")?;
            fail_batch_errors(&envelope.errors)?;
        }
        WireFormat::Text => batch_text_output(
            &uris,
            file_configs_map.as_ref(),
            &config,
            output_dir.as_deref(),
            content_output_dir.as_deref(),
        )?,
        WireFormat::Toon => batch_toon_output(
            &uris,
            file_configs_map.as_ref(),
            &config,
            output_dir.as_deref(),
            content_output_dir.as_deref(),
        )?,
    }

    Ok(())
}

#[expect(
    clippy::print_stdout,
    reason = "batch extraction results are the command's stdout result output"
)]
fn batch_text_output(
    uris: &[String],
    file_configs_map: Option<&std::collections::HashMap<String, serde_json::Value>>,
    config: &ExtractionConfig,
    output_dir: Option<&Path>,
    content_output_dir: Option<&Path>,
) -> Result<()> {
    let inputs = build_batch_inputs(uris, file_configs_map)?;
    refuse_binary_text_output(config, &inputs, content_output_dir.is_some())?;
    let batch_result = run_batch_sync(inputs, config)?;
    if let Some(dir) = content_output_dir {
        write_batch_documents(uris, &batch_result.results, config, dir)?;
    }
    let dir = output_dir.unwrap_or(Path::new("."));
    let mut diagnostics = std::io::stderr().lock();
    for (i, result) in batch_result.results.iter().enumerate() {
        if let Some(images) = &result.images {
            write_extracted_images(images, dir)?;
        }
        if content_output_dir.is_none() {
            println!("{}", style::header(&format!("=== Document {} ===", i + 1)));
            println!("{} {}", style::label("MIME Type:"), style::success(&result.mime_type));
            println!("{}\n{}", style::label("Content:"), result.content);
            println!();
        }
        // Warnings go to `stderr` for the same reason as in `extract_command`: the
        // batch text stream is content, not diagnostics.
        write_processing_warnings(&result.processing_warnings, &mut diagnostics)
            .context("Failed to write processing warnings")?;
    }
    write_batch_errors(&batch_result.errors, &mut diagnostics).context("Failed to write batch errors")?;
    fail_batch_errors(&batch_result.errors)?;
    Ok(())
}

#[expect(
    clippy::print_stdout,
    reason = "batch extraction results are the command's stdout result output"
)]
fn batch_toon_output(
    uris: &[String],
    file_configs_map: Option<&std::collections::HashMap<String, serde_json::Value>>,
    config: &ExtractionConfig,
    output_dir: Option<&Path>,
    content_output_dir: Option<&Path>,
) -> Result<()> {
    let total_t0 = Instant::now();
    let inputs = build_batch_inputs(uris, file_configs_map)?;
    let (output, per_file_ms) = run_json_batch_sync(inputs, config)?;
    let total_ms = total_t0.elapsed().as_secs_f64() * 1000.0;
    if let Some(dir) = content_output_dir {
        write_batch_documents(uris, &output.results, config, dir)?;
    }
    let dir = output_dir.unwrap_or(Path::new("."));
    for result in &output.results {
        if let Some(images) = &result.images {
            write_extracted_images(images, dir)?;
        }
    }
    let envelope = BatchEnvelope {
        results: output.results,
        total_ms,
        per_file_ms,
        errors: output.errors,
    };
    println!(
        "{}",
        serde_toon::to_string(&envelope).context("Failed to serialize batch extraction results to TOON")?
    );
    let mut diagnostics = std::io::stderr().lock();
    write_batch_errors(&envelope.errors, &mut diagnostics).context("Failed to write batch errors")?;
    fail_batch_errors(&envelope.errors)?;
    Ok(())
}

/// The batch text output joins documents under headers, which a binary document cannot be.
fn refuse_binary_text_output<'a>(
    config: &'a ExtractionConfig,
    inputs: &'a [ExtractInput],
    writes_files: bool,
) -> Result<()> {
    let per_file_binary_format = inputs.iter().find_map(|input| {
        let xberg::OutputFormat::Custom(name) = input.config.as_ref()?.output_format.as_ref()? else {
            return None;
        };
        (name == super::DOCX_CONTENT_FORMAT || name == super::PDF_CONTENT_FORMAT).then_some(name.as_str())
    });
    if let Some(binary_format) = super::requested_binary_format(config).or(per_file_binary_format)
        && !writes_files
    {
        anyhow::bail!(
            "--content-format {binary_format} produces one binary document per file, which the text \
             output cannot hold; use --format json, where each result's `content` is the \
             base64-encoded {}",
            binary_format.to_uppercase()
        );
    }
    Ok(())
}

fn write_batch_documents(
    uris: &[String],
    results: &[ExtractedDocument],
    config: &ExtractionConfig,
    output_dir: &Path,
) -> Result<()> {
    let mut destinations = Vec::with_capacity(results.len());
    let mut seen = std::collections::HashSet::with_capacity(results.len());
    for result in results {
        let source_index = result
            .metadata
            .additional
            .get("source_index")
            .and_then(serde_json::Value::as_u64)
            .and_then(|index| usize::try_from(index).ok())
            .context("Batch extraction result has no valid source_index for file output")?;
        let uri = uris
            .get(source_index)
            .with_context(|| format!("Batch extraction returned invalid source index {source_index}"))?;
        let stem = Path::new(uri)
            .file_stem()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .with_context(|| format!("Cannot derive a safe output name from input '{uri}'"))?;
        let extension = content_extension(result, config);
        let destination = output_dir.join(format!("{stem}.{extension}"));
        if !seen.insert(destination.clone()) {
            anyhow::bail!(
                "Batch output name collision for '{}'; rename the inputs so each output name is unique",
                destination.display()
            );
        }
        if destination.exists() {
            anyhow::bail!("Output file already exists: '{}'", destination.display());
        }
        destinations.push(destination);
    }

    for (result, destination) in results.iter().zip(destinations) {
        let binary_format = result
            .metadata
            .output_format
            .as_deref()
            .filter(|name| *name == super::DOCX_CONTENT_FORMAT || *name == super::PDF_CONTENT_FORMAT);
        super::write_document_file(result, binary_format, &destination)?;
    }
    Ok(())
}

fn content_extension(result: &ExtractedDocument, config: &ExtractionConfig) -> &'static str {
    let format = result
        .metadata
        .output_format
        .as_deref()
        .unwrap_or(match &config.output_format {
            xberg::OutputFormat::Plain => "plain",
            xberg::OutputFormat::Markdown => "markdown",
            xberg::OutputFormat::Djot => "djot",
            xberg::OutputFormat::Html => "html",
            xberg::OutputFormat::Json => "json",
            xberg::OutputFormat::DocTags => "doctags",
            xberg::OutputFormat::Custom(_) => "plain",
        });
    match format {
        "markdown" => "md",
        "djot" => "dj",
        "html" => "html",
        "json" => "json",
        "doctags" => "doctags",
        "docx" => "docx",
        "pdf" => "pdf",
        _ => "txt",
    }
}

/// Run batch extraction using the synchronous batch API for non-JSON output paths.
fn run_batch_sync(inputs: Vec<ExtractInput>, config: &ExtractionConfig) -> Result<ExtractionResult> {
    let input_count = inputs.len();
    // Describe the attempted operation only; the returned `ExtractionResult` retains every
    // per-input failure with its original source and index. ~keep
    block_on_extract_batch(inputs, config).with_context(|| format!("Failed to batch extract {input_count} inputs"))
}

#[cfg(test)]
mod binary_output_tests {
    use super::*;
    use xberg::{FileExtractionConfig, OutputFormat};

    #[test]
    fn batch_text_output_rejects_a_per_file_docx_override() {
        let inputs = vec![ExtractInput {
            config: Some(FileExtractionConfig {
                output_format: Some(OutputFormat::Custom(super::super::DOCX_CONTENT_FORMAT.to_string())),
                ..Default::default()
            }),
            ..Default::default()
        }];

        let error = refuse_binary_text_output(&ExtractionConfig::default(), &inputs, false)
            .expect_err("a per-file DOCX result cannot be joined into text output");
        assert!(error.to_string().contains("--format json"), "{error}");
    }

    #[test]
    fn batch_text_output_rejects_a_per_file_pdf_override() {
        let inputs = vec![ExtractInput {
            config: Some(FileExtractionConfig {
                output_format: Some(OutputFormat::Custom(super::super::PDF_CONTENT_FORMAT.to_string())),
                ..Default::default()
            }),
            ..Default::default()
        }];

        let error = refuse_binary_text_output(&ExtractionConfig::default(), &inputs, false)
            .expect_err("a per-file PDF result cannot be joined into text output");
        assert!(error.to_string().contains("--format json"), "{error}");
    }

    #[test]
    fn batch_text_output_accepts_binary_content_when_each_result_is_written_to_a_file() {
        let config = ExtractionConfig {
            output_format: OutputFormat::Custom(super::super::PDF_CONTENT_FORMAT.to_string()),
            ..Default::default()
        };

        refuse_binary_text_output(&config, &[], true).expect("file output can hold one binary document per result");
    }
}

/// Return one timing per input, keyed by the core engine's `source_index` metadata.
///
/// A source can yield multiple documents (for example, recursive URL extraction).
/// The first result carrying a source index defines that input's timing; later
/// results for the same source do not replace it. Results without a source index
/// are auxiliary and do not add entries to this input-aligned vector.
fn batch_per_file_timings(
    results: &[ExtractedDocument],
    errors: &[ExtractionErrorItem],
    input_count: usize,
) -> Result<Vec<Option<f64>>> {
    let mut timings = vec![None; input_count];
    let mut failed_inputs = vec![false; input_count];
    for error in errors {
        let failed = failed_inputs
            .get_mut(error.index)
            .with_context(|| format!("Batch extraction returned invalid error index {}", error.index))?;
        *failed = true;
    }
    for result in results {
        let Some(source_index) = result.metadata.additional.get("source_index") else {
            continue;
        };
        let source_index = source_index
            .as_u64()
            .and_then(|index| usize::try_from(index).ok())
            .context("Batch extraction result has an invalid source_index")?;
        let slot = timings
            .get_mut(source_index)
            .with_context(|| format!("Batch extraction returned invalid source index {source_index}"))?;
        if slot.is_some() {
            continue;
        }
        let timing = result
            .metadata
            .extraction_duration_ms
            .context("Batch extraction result is missing extraction_duration_ms")? as f64;
        *slot = Some(timing);
    }

    timings
        .into_iter()
        .enumerate()
        .map(|(index, timing)| {
            if timing.is_some() || failed_inputs[index] {
                Ok(timing)
            } else {
                anyhow::bail!("Batch extraction omitted timing for input {index}")
            }
        })
        .collect()
}

fn run_json_batch_sync(
    inputs: Vec<ExtractInput>,
    config: &ExtractionConfig,
) -> Result<(ExtractionResult, Vec<Option<f64>>)> {
    let input_count = inputs.len();
    // See `run_batch_sync` above: describe the attempted operation, not a guessed diagnosis.
    let output = block_on_extract_batch(inputs, config)
        .with_context(|| format!("Failed to batch extract {input_count} inputs"))?;
    let per_file_ms = batch_per_file_timings(&output.results, &output.errors, input_count)?;
    Ok((output, per_file_ms))
}

fn write_batch_errors<W: Write>(errors: &[ExtractionErrorItem], out: &mut W) -> std::io::Result<()> {
    for error in errors {
        writeln!(
            out,
            "error [input {}: {:?}]: {:?}",
            error.index, error.source, error.message
        )?;
    }
    Ok(())
}

fn fail_batch_errors(errors: &[ExtractionErrorItem]) -> Result<()> {
    if errors.is_empty() {
        return Ok(());
    }
    anyhow::bail!("Batch extraction completed with {} error(s).", errors.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn timed_batch_result(source_index: Option<serde_json::Value>, duration_ms: Option<u64>) -> ExtractedDocument {
        let mut result = ExtractedDocument::default();
        if let Some(source_index) = source_index {
            result.metadata.additional.insert("source_index".into(), source_index);
        }
        result.metadata.extraction_duration_ms = duration_ms;
        result
    }

    #[test]
    fn json_batch_extracts_in_input_order_with_one_timing_per_input() {
        let dir = tempdir().unwrap();
        let first = dir.path().join("first.txt");
        let second = dir.path().join("second.txt");
        std::fs::write(&first, "first document").unwrap();
        std::fs::write(&second, "second document").unwrap();
        let uris = vec![first.display().to_string(), second.display().to_string()];
        let inputs = build_batch_inputs(&uris, None).unwrap();
        let config = ExtractionConfig {
            max_concurrent_extractions: Some(2),
            ..ExtractionConfig::default()
        };

        let (output, per_file_ms) = run_json_batch_sync(inputs, &config).unwrap();
        let results = output.results;

        assert_eq!(results.len(), uris.len());
        assert_eq!(per_file_ms.len(), uris.len());
        assert!(
            per_file_ms
                .iter()
                .all(|elapsed_ms| elapsed_ms.is_some_and(|value| value >= 0.0))
        );
        let source_indices: Vec<u64> = results
            .iter()
            .map(|result| result.metadata.additional["source_index"].as_u64().unwrap())
            .collect();
        assert_eq!(source_indices, vec![0, 1]);

        let mut reordered_results = results.clone();
        reordered_results[0].metadata.extraction_duration_ms = Some(11);
        reordered_results[1].metadata.extraction_duration_ms = Some(22);
        reordered_results.reverse();
        assert_eq!(
            batch_per_file_timings(&reordered_results, &[], 2).unwrap(),
            vec![Some(11.0), Some(22.0)]
        );

        let contents: Vec<String> = results.into_iter().map(|result| result.content).collect();
        assert_eq!(contents, vec!["first document", "second document"]);
    }

    #[test]
    fn batch_per_file_timings_accepts_empty_batch() {
        assert_eq!(batch_per_file_timings(&[], &[], 0).unwrap(), Vec::<Option<f64>>::new());
    }

    #[test]
    fn batch_per_file_timings_rejects_missing_source_index() {
        let error = batch_per_file_timings(&[timed_batch_result(None, Some(1))], &[], 1).unwrap_err();
        assert!(error.to_string().contains("omitted timing for input 0"));
    }

    #[test]
    fn batch_per_file_timings_rejects_invalid_source_index() {
        let error = batch_per_file_timings(&[timed_batch_result(Some(serde_json::json!("zero")), Some(1))], &[], 1)
            .unwrap_err();
        assert!(error.to_string().contains("invalid source_index"));
    }

    #[test]
    fn batch_per_file_timings_rejects_out_of_range_source_index() {
        let error =
            batch_per_file_timings(&[timed_batch_result(Some(serde_json::json!(2)), Some(1))], &[], 1).unwrap_err();
        assert!(error.to_string().contains("invalid source index 2"));
    }

    #[test]
    fn batch_per_file_timings_rejects_missing_duration() {
        let error =
            batch_per_file_timings(&[timed_batch_result(Some(serde_json::json!(0)), None)], &[], 1).unwrap_err();
        assert!(error.to_string().contains("missing extraction_duration_ms"));
    }

    #[test]
    fn batch_per_file_timings_uses_first_result_for_each_source() {
        let results = vec![
            timed_batch_result(Some(serde_json::json!(1)), Some(30)),
            timed_batch_result(Some(serde_json::json!(0)), Some(10)),
            timed_batch_result(Some(serde_json::json!(0)), None),
            timed_batch_result(None, None),
        ];

        assert_eq!(
            batch_per_file_timings(&results, &[], 2).unwrap(),
            vec![Some(10.0), Some(30.0)]
        );
    }

    #[test]
    fn json_batch_preserves_partial_batch_errors() {
        let inputs = vec![ExtractInput::from_uri(
            "/definitely/missing/xberg-batch-input.txt".to_string(),
        )];

        let (output, per_file_ms) = run_json_batch_sync(inputs, &ExtractionConfig::default()).unwrap();

        assert!(output.results.is_empty());
        assert_eq!(output.errors.len(), 1);
        assert_eq!(output.errors[0].index, 0);
        assert_eq!(per_file_ms, vec![None]);
    }

    #[test]
    fn batch_error_diagnostics_escape_control_characters() {
        let errors = vec![ExtractionErrorItem {
            index: 4,
            code: 1,
            error_type: "test".to_string(),
            source: "source\n\x1b[31m".to_string(),
            message: "message\r\x1b[2J".to_string(),
        }];
        let mut rendered = Vec::new();

        write_batch_errors(&errors, &mut rendered).unwrap();

        let rendered = String::from_utf8(rendered).unwrap();
        assert_eq!(
            rendered,
            "error [input 4: \"source\\n\\u{1b}[31m\"]: \"message\\r\\u{1b}[2J\"\n"
        );
        assert!(!rendered.contains('\u{1b}'));
    }

    #[test]
    fn json_batch_applies_per_file_chunking_override() {
        let dir = tempdir().unwrap();
        let first = dir.path().join("first.txt");
        let second = dir.path().join("second.txt");
        let content = "alpha beta gamma delta epsilon zeta eta theta";
        std::fs::write(&first, content).unwrap();
        std::fs::write(&second, content).unwrap();
        let uris = vec![first.display().to_string(), second.display().to_string()];
        let file_configs = std::collections::HashMap::from([(
            uris[1].clone(),
            serde_json::json!({"chunking": {"max_chars": 12, "max_overlap": 0}}),
        )]);

        let inputs = build_batch_inputs(&uris, Some(&file_configs)).unwrap();
        let (output, _) = run_json_batch_sync(inputs, &ExtractionConfig::default()).unwrap();
        let results = output.results;

        assert!(results[0].chunks.is_none());
        assert!(results[1].chunks.as_ref().is_some_and(|chunks| chunks.len() > 1));
    }
}
