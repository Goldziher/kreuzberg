//! Jupyter Notebook extractor for .ipynb files.
//!
//! This extractor provides native Rust parsing of Jupyter notebooks,
//! extracting:
//! - Notebook metadata (kernelspec, language_info, nbformat)
//! - Cell content (markdown and code cells in order)
//! - Cell outputs (text, HTML, images)
//! - Cell metadata (execution_count, tags)
//!
//! Requires the `notebook` feature.

#[cfg(feature = "notebook")]
use crate::Result;
#[cfg(feature = "notebook")]
use crate::core::config::{ExtractionConfig, JupyterCellRendering};
#[cfg(feature = "notebook")]
use crate::extractors::myst::{TextNotebook, TextNotebookCellType, preprocess_myst};
#[cfg(feature = "notebook")]
use crate::extractors::security::SecurityBudget;
#[cfg(feature = "notebook")]
use crate::plugins::{InternalDocumentExtractor, Plugin};
#[cfg(feature = "notebook")]
use crate::types::ProcessingWarning;
#[cfg(feature = "notebook")]
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
#[cfg(feature = "notebook")]
use crate::types::internal_builder::InternalDocumentBuilder;
#[cfg(feature = "notebook")]
use crate::types::{ExtractedImage, Metadata};
#[cfg(feature = "notebook")]
use ahash::AHashMap;
#[cfg(feature = "notebook")]
use async_trait::async_trait;
#[cfg(feature = "notebook")]
use base64::Engine;
#[cfg(feature = "notebook")]
use bytes::Bytes;
#[cfg(feature = "notebook")]
use serde_json::{Value, json};
#[cfg(feature = "notebook")]
use std::borrow::Cow;

#[cfg(feature = "notebook")]
type NotebookContent = (
    String,
    AHashMap<Cow<'static, str>, Value>,
    Vec<ExtractedImage>,
    Value,
    Vec<ProcessingWarning>,
    Vec<ExtractedImage>,
);

/// Image MIME types recognized in output `data` bundles and cell
/// `attachments`, in the order they are checked. Raster formats are listed
/// first (matching the pre-#160 behavior for outputs), with `image/svg+xml`
/// appended last since it requires different handling (raw markup rather
/// than base64-decoded raster bytes) for outputs, though attachments always
/// decode base64 regardless of mimetype.
#[cfg(feature = "notebook")]
const SUPPORTED_IMAGE_MIME_TYPES: &[&str] = &["image/png", "image/jpeg", "image/gif", "image/webp", "image/svg+xml"];
#[cfg(feature = "notebook")]
const BASE64_ENCODED_BLOCK_SIZE: usize = 4;
#[cfg(feature = "notebook")]
const BASE64_DECODED_BLOCK_SIZE: usize = 3;

/// Jupyter Notebook extractor.
///
/// Extracts content from Jupyter notebook JSON files, including:
/// - Notebook metadata (kernel, language, nbformat version)
/// - Cell content (code and markdown)
/// - Cell outputs (text, HTML, etc.)
/// - Cell-level metadata (tags, execution counts)
#[cfg_attr(alef, alef(skip))]
#[cfg(feature = "notebook")]
pub struct JupyterExtractor;

/// One output's richest text-bearing representation, tagged with how it
/// must be rendered.
#[cfg(feature = "notebook")]
enum OutputRepresentation {
    /// Raw HTML markup. Must be emitted verbatim (not as a normal
    /// paragraph) because backslash-escaping markdown special characters
    /// (as paragraph rendering does) would corrupt the tags.
    RawHtml(String),
    /// Plain prose, from `text/markdown` or `text/plain`. Rendered as an
    /// ordinary paragraph, subject to the document's usual escaping.
    Text(String),
    /// LaTeX math, from `text/latex`. Rendered as a formula so the equation
    /// reaches the document's formula list.
    Latex(String),
}

#[cfg(feature = "notebook")]
#[derive(Default)]
struct CellVisibility {
    hide_input: bool,
    hide_output: bool,
    suppressed: bool,
}

#[cfg(feature = "notebook")]
impl JupyterExtractor {
    /// Create a new Jupyter extractor.
    pub(crate) fn new() -> Self {
        Self
    }

    /// Render a MyST/Jupytext text-notebook model through the same path as an
    /// `.ipynb` document. ~keep
    pub(crate) fn render_text_notebook(
        notebook: TextNotebook,
        mime_type: &str,
        config: &ExtractionConfig,
        budget: &mut SecurityBudget,
    ) -> Result<InternalDocument> {
        let mut notebook_json = Self::text_notebook_json(notebook);
        let metadata = Self::take_text_notebook_metadata(&mut notebook_json);
        let mut document = Self::render_notebook_json(notebook_json, mime_type, config, budget)?;
        for (key, value) in &metadata {
            document
                .metadata
                .additional
                .insert(Cow::Owned(key.clone()), value.clone());
        }
        document.metadata.title = metadata.get("title").and_then(Value::as_str).map(str::to_owned);
        Ok(document)
    }

    fn take_text_notebook_metadata(notebook: &mut Value) -> serde_json::Map<String, Value> {
        let Some(metadata) = notebook.get_mut("metadata").and_then(Value::as_object_mut) else {
            return serde_json::Map::new();
        };
        let original = std::mem::take(metadata);
        for key in ["kernelspec", "language_info"] {
            if let Some(value) = original.get(key) {
                metadata.insert(key.into(), value.clone());
            }
        }
        original
    }

    fn text_notebook_json(notebook: TextNotebook) -> Value {
        let cells = notebook.cells.into_iter().map(|cell| {
            let mut metadata = serde_json::Map::new();
            if !cell.tags.is_empty() {
                metadata.insert("tags".into(), json!(cell.tags));
            }
            if let Some(language) = cell.language {
                metadata.insert("language".into(), json!(language));
            }
            let mut value = json!({
                "cell_type": cell.cell_type.as_str(),
                "metadata": metadata,
                "source": cell.source,
            });
            if cell.cell_type == TextNotebookCellType::Code {
                value["execution_count"] = Value::Null;
                value["outputs"] = Value::Array(Vec::new());
            }
            value
        });
        json!({
            "cells": cells.collect::<Vec<_>>(),
            "metadata": notebook.metadata,
            "nbformat": 4,
            "nbformat_minor": 5,
        })
    }

    /// Extract content from a Jupyter notebook.
    #[cfg(test)]
    fn extract_notebook(content: &[u8], plain: bool) -> Result<NotebookContent> {
        let notebook: Value = serde_json::from_slice(content)
            .map_err(|e| crate::XbergError::parsing(format!("Failed to parse JSON: {}", e)))?;
        let mut budget = SecurityBudget::from_limits(&crate::extractors::security::SecurityLimits::default());
        budget.account_text(content.len())?;
        Self::extract_notebook_value(notebook, plain, true, &mut budget)
    }

    fn extract_notebook_value(
        notebook: Value,
        plain: bool,
        include_outputs: bool,
        budget: &mut SecurityBudget,
    ) -> Result<NotebookContent> {
        let mut extracted_content = String::new();
        let mut metadata = AHashMap::new();
        let mut images = Vec::new();
        let mut attachment_images = Vec::new();
        let mut warnings = Vec::new();

        if let Some(notebook_metadata) = notebook.get("metadata").and_then(|m| m.as_object()) {
            if let Some(kernelspec) = notebook_metadata.get("kernelspec") {
                metadata.insert(Cow::Borrowed("kernelspec"), kernelspec.clone());
            }

            if let Some(language_info) = notebook_metadata.get("language_info") {
                metadata.insert(Cow::Borrowed("language_info"), language_info.clone());

                if let Some(obj) = language_info.as_object() {
                    if let Some(name) = obj.get("name") {
                        metadata.insert(Cow::Borrowed("language_name"), name.clone());
                    }
                    if let Some(version) = obj.get("version") {
                        metadata.insert(Cow::Borrowed("language_version"), version.clone());
                    }
                    if let Some(mimetype) = obj.get("mimetype") {
                        metadata.insert(Cow::Borrowed("language_mimetype"), mimetype.clone());
                    }
                }
            }
        }

        if let Some(nbformat) = notebook.get("nbformat") {
            metadata.insert(Cow::Borrowed("nbformat"), nbformat.clone());
        }
        if let Some(nbformat_minor) = notebook.get("nbformat_minor") {
            metadata.insert(Cow::Borrowed("nbformat_minor"), nbformat_minor.clone());
        }

        if let Some(cells) = notebook.get("cells").and_then(|c| c.as_array()) {
            metadata.insert(Cow::Borrowed("cell_count"), json!(cells.len()));
        }

        if let Some(cells) = notebook.get("cells").and_then(|c| c.as_array()) {
            let mut cells_meta: Vec<Value> = Vec::with_capacity(cells.len());
            for (cell_idx, cell) in cells.iter().enumerate() {
                budget.step()?;
                cells_meta.push(Self::cell_metadata(cell, cell_idx));

                Self::extract_cell(
                    cell,
                    cell_idx,
                    &mut extracted_content,
                    &mut images,
                    plain,
                    include_outputs,
                    budget,
                )?;
                Self::extract_cell_attachments(cell, cell_idx, &mut attachment_images, &mut warnings, budget)?;
            }
            metadata.insert(Cow::Borrowed("cells"), json!(cells_meta));
        }

        Ok((
            extracted_content,
            metadata,
            images,
            notebook,
            warnings,
            attachment_images,
        ))
    }

    /// Extract a markdown/raw cell's `attachments` map into `images`.
    ///
    /// Per nbformat, `attachments` is a map of filename -> {mimetype: base64
    /// data}, referenced from cell source via `attachment:<filename>` URIs.
    /// Only markdown and raw cells carry this field. Each attachment is
    /// expected to carry exactly one mimetype; if none of the mimetypes on
    /// an attachment are a supported image type, a `ProcessingWarning`
    /// naming the attachment file is recorded instead of silently dropping
    /// it.
    fn extract_cell_attachments(
        cell: &Value,
        cell_idx: usize,
        images: &mut Vec<ExtractedImage>,
        warnings: &mut Vec<ProcessingWarning>,
        budget: &mut SecurityBudget,
    ) -> Result<()> {
        let Some(attachments) = cell.get("attachments").and_then(|a| a.as_object()) else {
            return Ok(());
        };

        for (filename, mime_map) in attachments {
            budget.step()?;
            let Some(mime_map) = mime_map.as_object() else {
                continue;
            };

            let supported = SUPPORTED_IMAGE_MIME_TYPES
                .iter()
                .find_map(|mime_type| mime_map.get(*mime_type).map(|value| (*mime_type, value)));

            let Some((mime_type, value)) = supported else {
                warnings.push(ProcessingWarning {
                    source: Cow::Borrowed("jupyter"),
                    message: Cow::Owned(format!(
                        "Cell {} attachment '{}' has no supported image MIME type; skipped",
                        cell_idx, filename
                    )),
                });
                continue;
            };

            let base64_str = Self::extract_source(value);
            let cleaned = base64_str.replace(['\n', '\r'], "");
            let Some(decoded) = Self::decode_base64_bounded(&cleaned, budget)? else {
                warnings.push(ProcessingWarning {
                    source: Cow::Borrowed("jupyter"),
                    message: Cow::Owned(format!(
                        "Cell {} attachment '{}' could not be base64-decoded; skipped",
                        cell_idx, filename
                    )),
                });
                continue;
            };

            let format = mime_type.trim_start_matches("image/").replace("svg+xml", "svg");
            let (image_kind, kind_confidence) =
                crate::extraction::image_kind::classify(crate::extraction::image_kind::ImageClassifyInput {
                    bytes: &decoded,
                    format: &format,
                    width: None,
                    height: None,
                    colorspace: None,
                    bits_per_component: None,
                    is_mask: false,
                });

            images.push(ExtractedImage {
                data: Bytes::from(decoded),
                format: Cow::Owned(format),
                image_index: images.len() as u32,
                page_number: Some((cell_idx + 1) as u32),
                width: None,
                height: None,
                colorspace: None,
                bits_per_component: None,
                is_mask: false,
                description: Some(format!("Notebook cell {} attachment: {}", cell_idx, filename)),
                ocr_result: None,
                bounding_box: None,
                source_path: None,
                image_kind: Some(image_kind),
                kind_confidence: Some(kind_confidence),
                cluster_id: None,
                caption: None,
                qr_codes: None,
                data_base64: None,
            });
        }
        Ok(())
    }

    fn cell_metadata(cell: &Value, cell_idx: usize) -> Value {
        let cell_type = cell
            .get("cell_type")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown");
        let mut metadata = serde_json::Map::new();
        metadata.insert("index".into(), json!(cell_idx));
        metadata.insert("cell_type".into(), json!(cell_type));

        for key in ["id", "execution_count"] {
            if let Some(value) = cell.get(key) {
                metadata.insert(key.into(), value.clone());
            }
        }
        if let Some(tags) = cell
            .get("metadata")
            .and_then(|value| value.get("tags"))
            .and_then(|value| value.as_array())
            && !tags.is_empty()
        {
            metadata.insert("tags".into(), Value::Array(tags.clone()));
        }
        if let Some(user_expressions) = cell
            .get("metadata")
            .and_then(|value| value.get("user_expressions"))
            .and_then(Value::as_array)
            && !user_expressions.is_empty()
        {
            metadata.insert("user_expressions".into(), Value::Array(user_expressions.clone()));
        }
        if let Some(language) = cell
            .get("metadata")
            .and_then(|value| value.get("language"))
            .and_then(Value::as_str)
        {
            metadata.insert("language".into(), json!(language));
        }
        if let Some(outputs) = cell.get("outputs").and_then(|value| value.as_array())
            && !outputs.is_empty()
        {
            metadata.insert(
                "outputs".into(),
                Value::Array(outputs.iter().enumerate().map(Self::output_metadata).collect()),
            );
        }

        Value::Object(metadata)
    }

    fn output_metadata((output_idx, output): (usize, &Value)) -> Value {
        let mut metadata = serde_json::Map::new();
        metadata.insert("index".into(), json!(output_idx));
        if let Some(output_type) = output.get("output_type") {
            metadata.insert("output_type".into(), output_type.clone());
        }
        for key in ["name", "execution_count", "ename", "evalue"] {
            if let Some(value) = output.get(key) {
                metadata.insert(key.into(), value.clone());
            }
        }
        if let Some(data) = output.get("data").and_then(|value| value.as_object()) {
            let mut mime_types: Vec<&str> = data.keys().map(String::as_str).collect();
            mime_types.sort_unstable();
            metadata.insert("mime_types".into(), json!(mime_types));
        }
        Value::Object(metadata)
    }

    /// Extract content from a single cell.
    fn extract_cell(
        cell: &Value,
        cell_idx: usize,
        content: &mut String,
        images: &mut Vec<ExtractedImage>,
        plain: bool,
        include_outputs: bool,
        budget: &mut SecurityBudget,
    ) -> Result<()> {
        let cell_type = cell.get("cell_type").and_then(|t| t.as_str()).unwrap_or("unknown");

        match cell_type {
            "markdown" => Self::extract_markdown_cell(cell, content)?,
            "code" => Self::extract_code_cell(cell, cell_idx, content, images, plain, include_outputs, budget)?,
            "raw" => Self::extract_raw_cell(cell, content)?,
            _ => {}
        }

        if !content.ends_with('\n') {
            content.push('\n');
        }
        content.push('\n');
        Ok(())
    }

    /// Extract markdown cell content.
    fn extract_markdown_cell(cell: &Value, content: &mut String) -> Result<()> {
        if let Some(source) = cell.get("source") {
            let cell_text = Self::extract_source(source);
            content.push_str(&cell_text);
        }
        Ok(())
    }

    /// Extract code cell content and outputs.
    fn extract_code_cell(
        cell: &Value,
        cell_idx: usize,
        content: &mut String,
        images: &mut Vec<ExtractedImage>,
        plain: bool,
        include_outputs: bool,
        budget: &mut SecurityBudget,
    ) -> Result<()> {
        if let Some(source) = cell.get("source") {
            let cell_text = Self::extract_source(source);
            content.push_str(&cell_text);
            if !cell_text.ends_with('\n') {
                content.push('\n');
            }
        }

        if include_outputs && let Some(outputs) = cell.get("outputs").and_then(|o| o.as_array()) {
            for output in outputs {
                budget.step()?;
                Self::extract_output(output, cell_idx, content, images, plain, budget)?;
            }
        }

        Ok(())
    }

    /// Extract raw cell content.
    fn extract_raw_cell(cell: &Value, content: &mut String) -> Result<()> {
        if let Some(source) = cell.get("source") {
            let cell_text = Self::extract_source(source);
            content.push_str(&cell_text);
        }
        Ok(())
    }

    /// Extract source content from various formats.
    ///
    /// Source can be either a string or an array of strings.
    fn extract_source(source: &Value) -> String {
        match source {
            Value::String(s) => s.clone(),
            Value::Array(arr) => arr.iter().filter_map(|v| v.as_str()).collect::<String>(),
            _ => String::new(),
        }
    }

    /// Text a kernel wrote for a terminal, as a notebook frontend shows it: ANSI escape sequences
    /// (the colours of an IPython traceback or of a Julia `DataFrame` in `text/plain`) are dropped,
    /// and a carriage return moves back to the start of the line, so the text after it overwrites the
    /// text before it, as progress bars expect.
    fn terminal_text(text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut line: Vec<char> = Vec::new();
        let mut column = 0;
        let mut chars = text.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                // An escape sequence is ESC and a byte in `@`..=`_`, two of which start a longer one
                '\u{1b}' => match chars.next_if(|c| ('@'..='_').contains(c)) {
                    // CSI: parameter and intermediate bytes up to a final byte in `@`..=`~`
                    Some('[') => while chars.next().is_some_and(|c| !('@'..='~').contains(&c)) {},
                    // OSC: up to BEL or ST (`ESC \`)
                    Some(']') => {
                        while let Some(c) = chars.next() {
                            if c == '\u{7}' || (c == '\u{1b}' && chars.next_if_eq(&'\\').is_some()) {
                                break;
                            }
                        }
                    }
                    _ => {}
                },
                '\r' if chars.peek() == Some(&'\n') => {}
                '\r' => column = 0,
                '\n' => {
                    out.extend(line.drain(..));
                    out.push('\n');
                    column = 0;
                }
                _ => {
                    if column < line.len() {
                        line[column] = ch;
                    } else {
                        line.push(ch);
                    }
                    column += 1;
                }
            }
        }
        out.extend(line);
        out
    }

    /// Extract output from a cell.
    fn extract_output(
        output: &Value,
        cell_idx: usize,
        content: &mut String,
        images: &mut Vec<ExtractedImage>,
        plain: bool,
        budget: &mut SecurityBudget,
    ) -> Result<()> {
        let output_type = output.get("output_type").and_then(|t| t.as_str()).unwrap_or("unknown");

        match output_type {
            "stream" => Self::extract_stream_output(output, content)?,
            "execute_result" | "display_data" | "update_display_data" => {
                Self::extract_data_output(output, cell_idx, content, images, plain, budget)?;
            }
            "error" => Self::extract_error_output(output, content, budget)?,
            _ => {}
        }

        Ok(())
    }

    /// Extract stream output (stdout, stderr).
    fn extract_stream_output(output: &Value, content: &mut String) -> Result<()> {
        if let Some(text) = output.get("text") {
            let text_content = Self::terminal_text(&Self::extract_source(text));
            content.push_str(&text_content);
        }

        Ok(())
    }

    /// Extract data output (execute_result or display_data).
    ///
    /// Prioritizes text/plain for quality scoring. For raster image types,
    /// decodes base64 data and populates the images collection.
    fn extract_data_output(
        output: &Value,
        cell_idx: usize,
        content: &mut String,
        images: &mut Vec<ExtractedImage>,
        plain_mode: bool,
        budget: &mut SecurityBudget,
    ) -> Result<()> {
        if let Some(data) = output.get("data").and_then(|d| d.as_object()) {
            if let Some(plain) = data.get("text/plain") {
                let text = Self::terminal_text(&Self::extract_source(plain));
                if !text.is_empty() {
                    content.push_str(&text);
                    if !text.ends_with('\n') {
                        content.push('\n');
                    }
                }
            }

            if !plain_mode {
                for mime_type in &["text/markdown", "text/html"] {
                    if let Some(mime_content) = data.get(*mime_type) {
                        let mime_text = Self::extract_source(mime_content);
                        if !mime_text.is_empty() {
                            content.push_str(&mime_text);
                            if !mime_text.ends_with('\n') {
                                content.push('\n');
                            }
                        }
                    }
                }
            }

            for mime_type in &["image/png", "image/jpeg", "image/gif", "image/webp"] {
                if let Some(image_value) = data.get(*mime_type) {
                    let base64_str = Self::extract_source(image_value);
                    let cleaned = base64_str.replace(['\n', '\r'], "");
                    if let Some(decoded) = Self::decode_base64_bounded(&cleaned, budget)? {
                        let format = match *mime_type {
                            "image/png" => "png",
                            "image/jpeg" => "jpeg",
                            "image/gif" => "gif",
                            "image/webp" => "webp",
                            _ => "unknown",
                        };

                        let (image_kind, kind_confidence) = crate::extraction::image_kind::classify(
                            crate::extraction::image_kind::ImageClassifyInput {
                                bytes: &decoded,
                                format,
                                width: None,
                                height: None,
                                colorspace: None,
                                bits_per_component: None,
                                is_mask: false,
                            },
                        );

                        images.push(ExtractedImage {
                            data: Bytes::from(decoded),
                            format: Cow::Borrowed(format),
                            image_index: images.len() as u32,
                            page_number: Some((cell_idx + 1) as u32),
                            width: None,
                            height: None,
                            colorspace: None,
                            bits_per_component: None,
                            is_mask: false,
                            description: Some(format!("Notebook cell {} output", cell_idx)),
                            ocr_result: None,
                            bounding_box: None,
                            source_path: None,
                            image_kind: Some(image_kind),
                            kind_confidence: Some(kind_confidence),
                            cluster_id: None,
                            caption: None,
                            qr_codes: None,
                            data_base64: None,
                        });
                        content.push_str(&format!("[Image: {}]\n", mime_type));
                    }
                }
            }

            if let Some(svg_value) = data.get("image/svg+xml") {
                let svg_markup = Self::extract_source(svg_value);
                if !svg_markup.is_empty() {
                    budget.account_text(svg_markup.len())?;
                    let svg_bytes = svg_markup.into_bytes();
                    let (image_kind, kind_confidence) =
                        crate::extraction::image_kind::classify(crate::extraction::image_kind::ImageClassifyInput {
                            bytes: &svg_bytes,
                            format: "svg",
                            width: None,
                            height: None,
                            colorspace: None,
                            bits_per_component: None,
                            is_mask: false,
                        });

                    images.push(ExtractedImage {
                        data: Bytes::from(svg_bytes),
                        format: Cow::Borrowed("svg"),
                        image_index: images.len() as u32,
                        page_number: Some((cell_idx + 1) as u32),
                        width: None,
                        height: None,
                        colorspace: None,
                        bits_per_component: None,
                        is_mask: false,
                        description: Some(format!("Notebook cell {} output", cell_idx)),
                        ocr_result: None,
                        bounding_box: None,
                        source_path: None,
                        image_kind: Some(image_kind),
                        kind_confidence: Some(kind_confidence),
                        cluster_id: None,
                        caption: None,
                        qr_codes: None,
                        data_base64: None,
                    });
                    content.push_str("[Image: image/svg+xml]\n");
                }
            }

            if let Some(json_content) = data.get("application/json")
                && let Ok(formatted) = serde_json::to_string_pretty(json_content)
            {
                budget.account_text(formatted.len())?;
                content.push_str(&formatted);
                content.push('\n');
            }
        }

        Ok(())
    }

    fn base64_decoded_upper_bound(encoded_len: usize) -> usize {
        encoded_len
            .saturating_add(BASE64_ENCODED_BLOCK_SIZE - 1)
            .saturating_div(BASE64_ENCODED_BLOCK_SIZE)
            .saturating_mul(BASE64_DECODED_BLOCK_SIZE)
    }

    fn decode_base64_bounded(encoded: &str, budget: &mut SecurityBudget) -> Result<Option<Vec<u8>>> {
        let mut prospective_budget = budget.clone();
        prospective_budget.account_text(Self::base64_decoded_upper_bound(encoded.len()))?;
        let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(encoded) else {
            return Ok(None);
        };
        *budget = prospective_budget;
        Ok(Some(decoded))
    }

    /// Push the richest text-bearing representation of a single output onto
    /// `builder` as one element, or push nothing if the output carries none.
    ///
    /// Precedence for `execute_result`/`display_data`/`update_display_data`,
    /// most to least rich (unless `plain` restricts output to plain text, in
    /// which case only `text/plain` is considered):
    /// 0. `text/latex` — the exact equation, emitted as a formula element. A
    ///    tool that ships it has already decided the output is math.
    /// 1. `text/html` — carries the most structure/semantics; some outputs
    ///    (e.g. a bare `display(HTML(...))` call) have *only* this
    ///    representation, so it must not be skipped in favor of a
    ///    less-structured one that happens to also be present.
    /// 2. `text/markdown` — less structure than HTML but more than plain text.
    /// 3. `text/plain` — always available as a fallback for reprs that only
    ///    implement `__repr__`.
    ///
    /// `stream` outputs use their `text` field directly. `error` outputs
    /// render the exception name/value plus the *full* traceback (not just
    /// the name/value), since the traceback is often the only actionable
    /// diagnostic for a failed cell.
    fn push_output_element(
        builder: &mut InternalDocumentBuilder,
        output: &Value,
        plain: bool,
        budget: &mut SecurityBudget,
    ) -> Result<()> {
        let output_type = output.get("output_type").and_then(|t| t.as_str()).unwrap_or("");
        match output_type {
            "stream" => Self::push_stream_element(builder, output, budget)?,
            "execute_result" | "display_data" | "update_display_data" => {
                Self::push_data_element(builder, output, plain, budget)?;
            }
            "error" => Self::push_error_element(builder, output, budget)?,
            _ => {}
        }
        Ok(())
    }

    fn push_stream_element(
        builder: &mut InternalDocumentBuilder,
        output: &Value,
        budget: &mut SecurityBudget,
    ) -> Result<()> {
        let text = Self::terminal_text(&Self::extract_source(output.get("text").unwrap_or(&Value::Null)));
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            budget.account_text(trimmed.len())?;
            builder.push_paragraph(trimmed, vec![], None, None);
        }
        Ok(())
    }

    fn push_data_element(
        builder: &mut InternalDocumentBuilder,
        output: &Value,
        plain: bool,
        budget: &mut SecurityBudget,
    ) -> Result<()> {
        let Some(data) = output.get("data").and_then(Value::as_object) else {
            return Ok(());
        };
        match Self::richest_output_representation(data, plain) {
            Some(OutputRepresentation::RawHtml(value)) => Self::push_raw_html(builder, &value, budget)?,
            Some(OutputRepresentation::Text(value)) => Self::push_text(builder, &value, budget)?,
            Some(OutputRepresentation::Latex(value)) => Self::push_latex(builder, &value, budget)?,
            None => {}
        }
        Ok(())
    }

    fn push_raw_html(builder: &mut InternalDocumentBuilder, value: &str, budget: &mut SecurityBudget) -> Result<()> {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            budget.account_text(trimmed.len())?;
            builder.push_raw_block("html", trimmed, None);
        }
        Ok(())
    }

    fn push_text(builder: &mut InternalDocumentBuilder, value: &str, budget: &mut SecurityBudget) -> Result<()> {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            budget.account_text(trimmed.len())?;
            builder.push_paragraph(trimmed, vec![], None, None);
        }
        Ok(())
    }

    fn push_latex(builder: &mut InternalDocumentBuilder, value: &str, budget: &mut SecurityBudget) -> Result<()> {
        let bare = crate::extraction::derive::strip_math_delimiters(value);
        if !bare.is_empty() {
            budget.account_text(bare.len())?;
            builder.push_formula(bare, None, None);
        }
        Ok(())
    }

    fn push_error_element(
        builder: &mut InternalDocumentBuilder,
        output: &Value,
        budget: &mut SecurityBudget,
    ) -> Result<()> {
        let text = Self::collect_error_text(output, budget)?;
        if !text.is_empty() {
            budget.account_text(text.len())?;
            builder.push_paragraph(&text, vec![], None, None);
        }
        Ok(())
    }

    /// Select the single richest text-bearing representation from an
    /// output's `data` bundle. See `push_output_element` for the precedence.
    fn richest_output_representation(
        data: &serde_json::Map<String, Value>,
        plain: bool,
    ) -> Option<OutputRepresentation> {
        if !plain {
            // A tool that ships `text/latex` has already decided the output is
            // math, and the LaTeX states the equation exactly. Nothing richer
            // can be recovered from the HTML or the repr of the same result.
            if let Some(latex) = data.get("text/latex") {
                let text = Self::extract_source(latex);
                if !text.trim().is_empty() {
                    return Some(OutputRepresentation::Latex(text));
                }
            }
            if let Some(html) = data.get("text/html") {
                let text = Self::extract_source(html);
                if !text.is_empty() {
                    return Some(OutputRepresentation::RawHtml(text));
                }
            }
            if let Some(markdown) = data.get("text/markdown") {
                let text = Self::extract_source(markdown);
                if !text.is_empty() {
                    return Some(OutputRepresentation::Text(text));
                }
            }
        }

        let plain_value = data.get("text/plain")?;
        let text = Self::terminal_text(&Self::extract_source(plain_value));
        if text.is_empty() {
            None
        } else {
            Some(OutputRepresentation::Text(text))
        }
    }

    /// Render an `error` output's exception name/value and full traceback as
    /// a single text block.
    fn collect_error_text(output: &Value, budget: &mut SecurityBudget) -> Result<String> {
        let ename = output.get("ename").and_then(|e| e.as_str()).unwrap_or("Unknown");
        let evalue = output.get("evalue").and_then(|e| e.as_str()).unwrap_or("");
        let mut text = format!("Error ({}): {}", ename, Self::terminal_text(evalue));

        if let Some(traceback) = output.get("traceback").and_then(|t| t.as_array()) {
            text.push('\n');
            text.push_str("Traceback:");
            for line in traceback {
                budget.step()?;
                if let Some(line_str) = line.as_str() {
                    text.push('\n');
                    text.push_str(&Self::terminal_text(line_str));
                }
            }
        }

        Ok(text)
    }

    fn prepare_notebook(
        notebook: &mut Value,
        apply_cell_tags: bool,
        plain: bool,
        budget: &mut SecurityBudget,
    ) -> Result<()> {
        let Some(cells) = notebook.get_mut("cells").and_then(Value::as_array_mut) else {
            return Ok(());
        };
        for cell in cells {
            budget.step()?;
            if apply_cell_tags {
                Self::apply_cell_visibility_tags(cell, budget)?;
            }
            Self::prepare_markdown_cell(cell, plain, budget)?;
        }
        Ok(())
    }

    fn apply_cell_visibility_tags(cell: &mut Value, budget: &mut SecurityBudget) -> Result<()> {
        let visibility = Self::cell_visibility(cell, budget)?;
        let Some(object) = cell.as_object_mut() else {
            return Ok(());
        };
        if visibility.hide_input {
            object.insert("source".into(), Value::Array(Vec::new()));
            object.remove("attachments");
        }
        if visibility.hide_output {
            object.insert("outputs".into(), Value::Array(Vec::new()));
        }
        if visibility.suppressed {
            Self::redact_saved_expression_payloads(object, !visibility.hide_input, budget)?;
        }
        Ok(())
    }

    fn cell_visibility(cell: &Value, budget: &mut SecurityBudget) -> Result<CellVisibility> {
        let tags = cell
            .get("metadata")
            .and_then(|metadata| metadata.get("tags"))
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let mut visibility = CellVisibility::default();
        for tag in tags.iter().filter_map(Value::as_str) {
            budget.step()?;
            match tag {
                "remove-cell" | "remove_cell" | "hide-cell" | "hide_cell" => {
                    visibility.hide_input = true;
                    visibility.hide_output = true;
                }
                "remove-input" | "remove_input" | "hide-input" | "hide_input" => visibility.hide_input = true,
                "remove-output" | "remove_output" | "hide-output" | "hide_output" => visibility.hide_output = true,
                _ => {}
            }
        }
        visibility.suppressed = visibility.hide_input || visibility.hide_output;
        Ok(visibility)
    }

    fn redact_saved_expression_payloads(
        cell: &mut serde_json::Map<String, Value>,
        retain_identifier: bool,
        budget: &mut SecurityBudget,
    ) -> Result<()> {
        let Some(expressions) = cell
            .get_mut("metadata")
            .and_then(Value::as_object_mut)
            .and_then(|metadata| metadata.get_mut("user_expressions"))
            .and_then(Value::as_array_mut)
        else {
            return Ok(());
        };
        for expression in expressions {
            budget.step()?;
            let identifier = expression.get("expression").cloned();
            let status = expression
                .get("result")
                .and_then(|result| result.get("status"))
                .cloned();
            let mut redacted = serde_json::Map::new();
            if retain_identifier && let Some(identifier) = identifier {
                redacted.insert("expression".into(), identifier);
            }
            if let Some(status) = status {
                redacted.insert("result".into(), json!({"status": status}));
            }
            *expression = Value::Object(redacted);
        }
        Ok(())
    }

    fn prepare_markdown_cell(cell: &mut Value, plain: bool, budget: &mut SecurityBudget) -> Result<()> {
        if cell.get("cell_type").and_then(Value::as_str) != Some("markdown") {
            return Ok(());
        }
        let source = Self::extract_source(cell.get("source").unwrap_or(&Value::Null));
        if source.is_empty() {
            return Ok(());
        }
        let resolved = Self::resolve_saved_expressions(cell, &source, plain, budget)?;
        cell["source"] = Value::String(preprocess_myst(&resolved, budget)?);
        Ok(())
    }

    fn resolve_saved_expressions<'a>(
        cell: &Value,
        source: &'a str,
        plain: bool,
        budget: &mut SecurityBudget,
    ) -> Result<Cow<'a, str>> {
        let Some(expressions) = cell
            .get("metadata")
            .and_then(|metadata| metadata.get("user_expressions"))
            .and_then(Value::as_array)
        else {
            return Ok(Cow::Borrowed(source));
        };
        let mut saved = AHashMap::new();
        for expression in expressions {
            budget.step()?;
            if let Some((name, value)) = Self::saved_expression(expression, plain) {
                saved.insert(name, value);
            }
        }
        Self::replace_eval_roles(source, &saved, budget)
    }

    fn saved_expression(expression: &Value, plain: bool) -> Option<(&str, String)> {
        let name = expression.get("expression")?.as_str()?;
        Some((name, Self::saved_expression_text(expression, plain)?))
    }

    fn replace_eval_roles<'a>(
        source: &'a str,
        saved: &AHashMap<&str, String>,
        budget: &mut SecurityBudget,
    ) -> Result<Cow<'a, str>> {
        const PREFIX: &str = "{eval}`";
        if !source.contains(PREFIX) {
            return Ok(Cow::Borrowed(source));
        }
        let mut output = String::with_capacity(source.len());
        let mut remaining = source;
        while let Some(start) = remaining.find(PREFIX) {
            Self::push_budgeted(&mut output, &remaining[..start], budget)?;
            let role = &remaining[start + PREFIX.len()..];
            let Some(end) = role.find('`') else {
                Self::push_budgeted(&mut output, &remaining[start..], budget)?;
                return Ok(Cow::Owned(output));
            };
            let expression = &role[..end];
            if let Some(value) = saved.get(expression) {
                Self::push_budgeted(&mut output, value, budget)?;
            } else {
                Self::push_budgeted(&mut output, &remaining[start..start + PREFIX.len() + end + 1], budget)?;
            }
            remaining = &role[end + 1..];
        }
        Self::push_budgeted(&mut output, remaining, budget)?;
        Ok(Cow::Owned(output))
    }

    fn push_budgeted(output: &mut String, value: &str, budget: &mut SecurityBudget) -> Result<()> {
        budget.account_text(value.len())?;
        output.push_str(value);
        Ok(())
    }

    fn saved_expression_text(expression: &Value, plain: bool) -> Option<String> {
        let result = expression.get("result")?;
        if result
            .get("status")
            .and_then(Value::as_str)
            .is_some_and(|status| status != "ok")
        {
            return None;
        }
        let data = result.get("data")?.as_object()?;
        if !plain {
            for mime_type in ["text/markdown", "text/latex"] {
                if let Some(value) = data.get(mime_type) {
                    let text = Self::extract_source(value);
                    if !text.is_empty() {
                        return Some(text);
                    }
                }
            }
        }
        let text = Self::terminal_text(&Self::extract_source(data.get("text/plain")?));
        (!text.is_empty()).then_some(text)
    }

    fn render_notebook_json(
        mut notebook: Value,
        mime_type: &str,
        config: &ExtractionConfig,
        budget: &mut SecurityBudget,
    ) -> Result<InternalDocument> {
        let plain = matches!(config.output_format, crate::core::config::OutputFormat::Plain);
        Self::prepare_notebook(&mut notebook, config.apply_notebook_cell_tags, plain, budget)?;
        let content = Self::extract_notebook_value(
            notebook,
            plain,
            config.jupyter_cell_rendering.includes_outputs(),
            budget,
        )?;
        Self::assemble_document(content, mime_type, config, plain, budget)
    }

    fn assemble_document(
        content: NotebookContent,
        mime_type: &str,
        config: &ExtractionConfig,
        plain: bool,
        budget: &mut SecurityBudget,
    ) -> Result<InternalDocument> {
        let (_, metadata, output_images, notebook, warnings, attachment_images) = content;
        let language = metadata
            .get(&Cow::Borrowed("language_name"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let additional = metadata.into_iter().collect();
        let mut document = Self::build_internal_document_with_budget(
            &notebook,
            config.jupyter_cell_rendering,
            plain,
            &output_images,
            &attachment_images,
            budget,
        )?
        .unwrap_or_else(|| InternalDocumentBuilder::new("jupyter").build());
        for (index, image) in document.images.iter_mut().enumerate() {
            image.image_index = index as u32;
        }
        document.mime_type = mime_type.to_string();
        document.metadata = Metadata {
            language,
            additional,
            ..Default::default()
        };
        document.processing_warnings.extend(warnings);
        Ok(document)
    }

    /// Build an `InternalDocument` from the already-parsed notebook JSON.
    ///
    /// Markdown cells are split into headings and paragraphs. Code cells
    /// become code blocks followed by any output paragraphs. `plain`
    /// mirrors `ExtractionConfig::output_format` being `Plain`
    /// and suppresses richer (markdown/html) output representations in
    /// favor of `text/plain` only.
    #[cfg(test)]
    fn build_internal_document(
        notebook: &Value,
        rendering: JupyterCellRendering,
        plain: bool,
    ) -> Option<InternalDocument> {
        let mut budget = SecurityBudget::from_limits(&crate::extractors::security::SecurityLimits::default());
        Self::build_internal_document_with_budget(notebook, rendering, plain, &[], &[], &mut budget)
            .ok()
            .flatten()
    }

    fn attachment_placeholder_text(document: &InternalDocument, filename: &str) -> Option<String> {
        let destination = format!("attachment:{filename}");
        let uri = document
            .uris
            .iter()
            .find(|uri| uri.kind == crate::types::UriKind::Image && uri.url == destination)?;
        Some(match uri.label.as_deref() {
            Some(label) => format!("[Image: {label} ({destination})]"),
            None => format!("[Image: {destination}]"),
        })
    }

    fn attach_cell_images(cell_doc: &mut InternalDocument, attachment_images: &[ExtractedImage], page_number: u32) {
        for image in attachment_images
            .iter()
            .filter(|image| image.page_number == Some(page_number))
        {
            let filename = image
                .description
                .as_deref()
                .and_then(|description| description.split_once(" attachment: ").map(|(_, filename)| filename));
            let placeholder_text = filename.and_then(|filename| Self::attachment_placeholder_text(cell_doc, filename));
            let image_index = cell_doc.push_image(image.clone());
            let placeholder = placeholder_text.and_then(|placeholder_text| {
                cell_doc
                    .elements
                    .iter_mut()
                    .find(|element| matches!(element.kind, ElementKind::Paragraph) && element.text == placeholder_text)
            });
            if let Some(element) = placeholder {
                element.kind = ElementKind::Image { image_index };
                element.text = image.description.clone().unwrap_or_default();
            } else {
                cell_doc.push_element(InternalElement::text(
                    ElementKind::Image { image_index },
                    image.description.clone().unwrap_or_default(),
                    0,
                ));
            }
        }
    }

    fn code_cell_attributes(cell: &Value, cell_language: Option<&str>) -> AHashMap<String, String> {
        let mut attrs = AHashMap::new();
        if let Some(language) = cell_language {
            attrs.insert("language".to_string(), language.to_string());
        }
        if let Some(exec_count) = cell.get("execution_count") {
            match exec_count {
                Value::Number(n) => {
                    attrs.insert("execution_count".to_string(), n.to_string());
                }
                Value::Null => {
                    attrs.insert("execution_count".to_string(), "null".to_string());
                }
                _ => {}
            }
        }
        if let Some(tags) = cell
            .get("metadata")
            .and_then(|m| m.get("tags"))
            .and_then(|t| t.as_array())
            && !tags.is_empty()
        {
            let tag_strs: Vec<&str> = tags.iter().filter_map(|v| v.as_str()).collect();
            attrs.insert("tags".to_string(), tag_strs.join(","));
        }
        attrs
    }

    fn build_internal_document_with_budget(
        notebook: &Value,
        rendering: JupyterCellRendering,
        plain: bool,
        output_images: &[ExtractedImage],
        attachment_images: &[ExtractedImage],
        budget: &mut SecurityBudget,
    ) -> Result<Option<InternalDocument>> {
        let Some(cells) = notebook.get("cells").and_then(Value::as_array) else {
            return Ok(None);
        };

        let kernel_lang = notebook
            .get("metadata")
            .and_then(|m| m.get("kernelspec"))
            .and_then(|k| k.get("language"))
            .and_then(|l| l.as_str())
            .or_else(|| {
                notebook
                    .get("metadata")
                    .and_then(|m| m.get("language_info"))
                    .and_then(|l| l.get("name"))
                    .and_then(|n| n.as_str())
            });

        let mut builder = InternalDocumentBuilder::new("jupyter");

        for (cell_idx, cell) in cells.iter().enumerate() {
            budget.step()?;
            let cell_type = cell.get("cell_type").and_then(|t| t.as_str()).unwrap_or("unknown");
            let source_text = Self::extract_source(cell.get("source").unwrap_or(&Value::Null));
            let trimmed = source_text.trim();

            // A cell is only genuinely empty (and therefore droppable) if it
            // also has nothing else to contribute: a code cell's `source`
            // may be legitimately cleared (privacy/size stripping) while its
            // saved `outputs` still carry real content (#159), and a
            // markdown/raw cell's `source` may be empty while it still
            // references image `attachments` (#160).
            let has_code_outputs = cell_type == "code"
                && cell
                    .get("outputs")
                    .and_then(|o| o.as_array())
                    .is_some_and(|arr| !arr.is_empty());
            let has_attachments = cell
                .get("attachments")
                .and_then(|a| a.as_object())
                .is_some_and(|m| !m.is_empty());

            if trimmed.is_empty() && !has_code_outputs && !has_attachments {
                continue;
            }

            match cell_type {
                "markdown" => {
                    budget.account_text(trimmed.len())?;
                    let events: Vec<pulldown_cmark::Event> =
                        pulldown_cmark::Parser::new_ext(trimmed, crate::extractors::markdown::markdown_options())
                            .collect();
                    let mut cell_doc =
                        crate::extractors::markdown::MarkdownExtractor::build_internal_document(&events, &None);
                    let page_number = (cell_idx + 1) as u32;
                    Self::attach_cell_images(&mut cell_doc, attachment_images, page_number);
                    builder.append_document(cell_doc);
                }
                "code" => {
                    if rendering.includes_source() && !trimmed.is_empty() {
                        budget.account_text(trimmed.len())?;
                        let cell_language = cell
                            .get("metadata")
                            .and_then(|metadata| metadata.get("language"))
                            .and_then(Value::as_str)
                            .or(kernel_lang);
                        let idx = builder.push_code(trimmed, cell_language, None, None);
                        let attrs = Self::code_cell_attributes(cell, cell_language);
                        if !attrs.is_empty() {
                            builder.set_attributes(idx, attrs);
                        }
                    }

                    if rendering.includes_outputs()
                        && let Some(outputs) = cell.get("outputs").and_then(|o| o.as_array())
                    {
                        for output in outputs {
                            budget.step()?;
                            Self::push_output_element(&mut builder, output, plain, budget)?;
                        }
                    }
                    let page_number = (cell_idx + 1) as u32;
                    for image in output_images
                        .iter()
                        .filter(|image| image.page_number == Some(page_number))
                    {
                        builder.push_image(
                            image.description.as_deref(),
                            image.clone(),
                            image.page_number,
                            image.bounding_box,
                        );
                    }
                }
                _ => {
                    if !trimmed.is_empty() {
                        budget.account_text(trimmed.len())?;
                        builder.push_paragraph(trimmed, vec![], None, None);
                    }
                }
            }
        }

        Ok(Some(builder.build()))
    }

    /// Extract error output, preserving ename, evalue, and traceback in content.
    fn extract_error_output(output: &Value, content: &mut String, budget: &mut SecurityBudget) -> Result<()> {
        let ename = output.get("ename").and_then(|e| e.as_str()).unwrap_or("Unknown");
        let evalue = output.get("evalue").and_then(|e| e.as_str()).unwrap_or("");

        content.push_str(&format!("Error ({}): {}\n", ename, Self::terminal_text(evalue)));

        if let Some(traceback) = output.get("traceback").and_then(|t| t.as_array()) {
            content.push_str("Traceback:\n");
            for line in traceback {
                budget.step()?;
                if let Some(line_str) = line.as_str() {
                    content.push_str(&Self::terminal_text(line_str));
                    content.push('\n');
                }
            }
        }

        Ok(())
    }
}

#[cfg(feature = "notebook")]
impl Default for JupyterExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "notebook")]
impl Plugin for JupyterExtractor {
    fn name(&self) -> &str {
        "jupyter-extractor"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn description(&self) -> &str {
        "Extracts content from Jupyter notebooks (.ipynb files)"
    }

    fn author(&self) -> &str {
        "Xberg Team"
    }
}

#[cfg(feature = "notebook")]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl InternalDocumentExtractor for JupyterExtractor {
    #[cfg_attr(
        feature = "otel",
        tracing::instrument(
            skip(self, content, config),
            fields(
                extractor.name = self.name(),
                content.size_bytes = content.len(),
            )
        )
    )]
    async fn extract_content(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let mut budget = SecurityBudget::from_config(config);
        budget.account_text(content.len())?;
        // A UTF-8 byte order mark is not part of the JSON, and `serde_json` rejects it. ~keep
        let content = content.strip_prefix(b"\xEF\xBB\xBF".as_slice()).unwrap_or(content);
        let notebook = serde_json::from_slice(content)
            .map_err(|error| crate::XbergError::parsing(format!("Failed to parse JSON: {error}")))?;
        Self::render_notebook_json(notebook, mime_type, config, &mut budget)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/x-ipynb+json"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractors::security::SecurityLimits;
    use crate::plugins::InternalDocumentExtractor;

    fn test_budget() -> SecurityBudget {
        SecurityBudget::from_limits(&SecurityLimits::default())
    }

    #[tokio::test]
    async fn should_create_image_elements_for_output_and_attachment_images() {
        let image = base64::engine::general_purpose::STANDARD.encode([0x89, 0x50, 0x4e, 0x47]);
        let inline_image = format!("![inline](data:image/png;base64,{image})\n\n");
        let notebook = json!({
            "cells": [
                {
                    "cell_type": "markdown",
                    "metadata": {},
                    "source": [inline_image, "![label](attachment:label.png)"],
                    "attachments": {"label.png": {"image/png": image}}
                },
                {
                    "cell_type": "code",
                    "execution_count": 1,
                    "metadata": {},
                    "source": ["show()"],
                    "outputs": [{
                        "output_type": "display_data",
                        "metadata": {},
                        "data": {"image/png": image}
                    }]
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        });
        let config = ExtractionConfig {
            ocr: Some(crate::core::config::OcrConfig::default()),
            ..Default::default()
        };

        let doc = JupyterExtractor::new()
            .extract_content(
                serde_json::to_vec(&notebook).unwrap().as_slice(),
                "application/x-ipynb+json",
                &config,
            )
            .await
            .expect("notebook extraction must succeed");

        let image_indices: Vec<u32> = doc
            .elements
            .iter()
            .filter_map(|element| match element.kind {
                ElementKind::Image { image_index } => Some(image_index),
                _ => None,
            })
            .collect();
        assert_eq!(doc.images.len(), 3);
        assert_eq!(image_indices, vec![0, 1, 2]);
        assert!(image_indices.iter().all(|index| (*index as usize) < doc.images.len()));
    }

    #[tokio::test]
    async fn should_match_attachment_destinations_exactly_when_filenames_share_a_prefix() {
        let image = base64::engine::general_purpose::STANDARD.encode([0x89, 0x50, 0x4e, 0x47]);
        let notebook = json!({
            "cells": [{
                "cell_type": "markdown",
                "metadata": {},
                "source": [
                    "[download](attachment:plot.png)\n\n",
                    "![long](attachment:plot.png.backup)\n\n",
                    "![colon](<attachment:plot: detail.png>)\n\n",
                    "![short](attachment:plot.png)"
                ],
                "attachments": {
                    "plot.png": {"image/png": image},
                    "plot.png.backup": {"image/png": image},
                    "plot: detail.png": {"image/png": image}
                }
            }],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        });
        let config = ExtractionConfig {
            ocr: Some(crate::core::config::OcrConfig::default()),
            ..Default::default()
        };

        let doc = JupyterExtractor::new()
            .extract_content(
                serde_json::to_vec(&notebook).unwrap().as_slice(),
                "application/x-ipynb+json",
                &config,
            )
            .await
            .expect("notebook extraction must succeed");

        let image_indices: Vec<u32> = doc
            .elements
            .iter()
            .filter_map(|element| match element.kind {
                ElementKind::Image { image_index } => Some(image_index),
                _ => None,
            })
            .collect();
        assert_eq!(image_indices, vec![1, 2, 0]);
        assert_eq!(
            doc.elements
                .iter()
                .filter(|element| {
                    matches!(element.kind, ElementKind::Paragraph) && element.text.contains("attachment:")
                })
                .count(),
            0
        );
    }

    #[test]
    fn test_jupyter_extractor_plugin_interface() {
        let extractor = JupyterExtractor::new();
        assert_eq!(extractor.name(), "jupyter-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 50);
        assert!(extractor.supported_mime_types().contains(&"application/x-ipynb+json"));
    }

    #[test]
    fn test_extract_execution_count_and_tags() {
        let notebook_json = r#"{
            "cells": [
                {
                    "cell_type": "code",
                    "id": "code-cell",
                    "source": ["print('hello')"],
                    "execution_count": 5,
                    "outputs": [{
                        "output_type": "execute_result",
                        "execution_count": 5,
                        "data": {"text/plain": ["hello"]},
                        "metadata": {}
                    }],
                    "metadata": {"tags": ["test-tag", "important"]}
                }
            ],
            "metadata": {
                "kernelspec": {"name": "python3", "language": "python"},
                "language_info": {"name": "python", "version": "3.10.0", "mimetype": "text/x-python"}
            },
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let (_, metadata, _, _, _, _) = JupyterExtractor::extract_notebook(notebook_json.as_bytes(), false).unwrap();

        let cells = metadata.get(&Cow::Borrowed("cells"));
        assert!(cells.is_some(), "Should have cells metadata array");
        let cells_arr = cells.unwrap().as_array().expect("cells should be an array");
        assert_eq!(cells_arr.len(), 1);
        let cell0 = &cells_arr[0];
        assert_eq!(cell0["index"], json!(0));
        assert_eq!(cell0["cell_type"], json!("code"));
        assert_eq!(cell0["id"], json!("code-cell"));
        assert_eq!(cell0["execution_count"], json!(5));
        assert_eq!(cell0["tags"], json!(["test-tag", "important"]));
        assert_eq!(cell0["outputs"][0]["output_type"], json!("execute_result"));
        assert_eq!(cell0["outputs"][0]["execution_count"], json!(5));
        assert_eq!(cell0["outputs"][0]["mime_types"], json!(["text/plain"]));

        assert_eq!(metadata.get(&Cow::Borrowed("cell_count")), Some(&json!(1)));

        assert_eq!(metadata.get(&Cow::Borrowed("language_name")), Some(&json!("python")));
        assert_eq!(metadata.get(&Cow::Borrowed("language_version")), Some(&json!("3.10.0")));
        assert_eq!(
            metadata.get(&Cow::Borrowed("language_mimetype")),
            Some(&json!("text/x-python"))
        );

        assert_eq!(metadata.get(&Cow::Borrowed("nbformat_minor")), Some(&json!(5)));
    }

    #[test]
    fn test_extract_error_output_content() {
        let notebook_json = r#"{
            "cells": [
                {
                    "cell_type": "code",
                    "source": ["1/0"],
                    "execution_count": 1,
                    "outputs": [
                        {
                            "output_type": "error",
                            "ename": "ZeroDivisionError",
                            "evalue": "division by zero",
                            "traceback": ["Traceback line 1", "Traceback line 2"]
                        }
                    ],
                    "metadata": {}
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 0
        }"#;

        let (content, _, _, _, _, _) = JupyterExtractor::extract_notebook(notebook_json.as_bytes(), false).unwrap();

        assert!(
            content.contains("Error (ZeroDivisionError): division by zero"),
            "Should contain error name and value"
        );
        assert!(content.contains("Traceback:"), "Should contain traceback header");
        assert!(content.contains("Traceback line 1"), "Should contain traceback lines");
    }

    fn rendering_sample() -> Value {
        serde_json::from_str(
            r#"{
            "cells": [
                {
                    "cell_type": "code",
                    "source": ["print('hello world')"],
                    "execution_count": 1,
                    "outputs": [
                        {"output_type": "stream", "name": "stdout", "text": ["hello world\n"]}
                    ],
                    "metadata": {}
                }
            ],
            "metadata": {"kernelspec": {"name": "python3", "language": "python"}},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#,
        )
        .unwrap()
    }

    #[test]
    fn terminal_text_drops_escape_sequences_and_applies_carriage_returns() {
        // IPython traceback colours, a 256-colour run and an OSC 8 hyperlink
        assert_eq!(
            JupyterExtractor::terminal_text("\u{1b}[0;31mZeroDivisionError\u{1b}[0m: \u{1b}[38;5;241m1\u{1b}[39m"),
            "ZeroDivisionError: 1"
        );
        assert_eq!(
            JupyterExtractor::terminal_text("\u{1b}]8;;https://example.com\u{1b}\\docs\u{1b}]8;;\u{7} here"),
            "docs here"
        );
        // A progress bar redrawn with `\r` keeps its last state; a shorter redraw keeps the rest
        assert_eq!(
            JupyterExtractor::terminal_text(" 33%|###  | 1/3\r100%|#####| 3/3\ndone\r\n"),
            "100%|#####| 3/3\ndone\n"
        );
        assert_eq!(JupyterExtractor::terminal_text("abcdef\rXY"), "XYcdef");
        // A lone ESC before ordinary text, and an unterminated sequence, drop nothing else
        assert_eq!(JupyterExtractor::terminal_text("a\u{1b}\nb\u{1b}[1"), "a\nb");
    }

    #[test]
    fn should_drop_ansi_escapes_from_the_plain_text_of_a_rich_output() {
        // IJulia writes a `DataFrame`'s `text/plain` with bold headers and grey type rows
        let notebook_json = r#"{
            "cells": [
                {
                    "cell_type": "code",
                    "source": ["df"],
                    "execution_count": 1,
                    "outputs": [
                        {
                            "output_type": "display_data",
                            "data": {"text/plain": ["\u001b[1m2×1 DataFrame\u001b[0m\n\u001b[1m Row \u001b[0m│\u001b[1m region \u001b[0m\n     │\u001b[90m String \u001b[0m"]},
                            "metadata": {}
                        },
                        {
                            "output_type": "execute_result",
                            "execution_count": 1,
                            "data": {"text/plain": ["\u001b[32mok\u001b[39m"]},
                            "metadata": {}
                        }
                    ],
                    "metadata": {}
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;
        let notebook: Value = serde_json::from_str(notebook_json).unwrap();

        let (content, _, _, _, _, _) = JupyterExtractor::extract_notebook(notebook_json.as_bytes(), false).unwrap();
        let doc = JupyterExtractor::build_internal_document(&notebook, JupyterCellRendering::Both, false).unwrap();
        let texts: Vec<&str> = doc.elements.iter().map(|e| e.text.as_str()).collect();

        for text in [content.as_str(), &texts.join("\n")] {
            assert!(!text.contains('\u{1b}') && !text.contains("[1m"), "got: {text:?}");
            assert!(
                text.contains("2×1 DataFrame\n Row │ region \n     │ String"),
                "got: {text:?}"
            );
            assert!(text.contains("ok"), "got: {text:?}");
        }
    }

    #[test]
    fn should_render_stream_and_error_outputs_as_a_notebook_shows_them() {
        let notebook_json = r#"{
            "cells": [
                {
                    "cell_type": "code",
                    "source": ["run()"],
                    "execution_count": 1,
                    "outputs": [
                        {"output_type": "stream", "name": "stderr", "text": [" 50%|##   | 1/2\r100%|#####| 2/2\n"]},
                        {
                            "output_type": "error",
                            "ename": "ValueError",
                            "evalue": "bad \u001b[1mvalue\u001b[0m",
                            "traceback": ["\u001b[0;31mValueError\u001b[0m: bad value"]
                        }
                    ],
                    "metadata": {}
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;
        let notebook: Value = serde_json::from_str(notebook_json).unwrap();

        let (content, _, _, _, _, _) = JupyterExtractor::extract_notebook(notebook_json.as_bytes(), false).unwrap();
        let doc = JupyterExtractor::build_internal_document(&notebook, JupyterCellRendering::Both, false).unwrap();
        let texts: Vec<&str> = doc.elements.iter().map(|e| e.text.as_str()).collect();

        for text in [content.as_str(), &texts.join("\n")] {
            assert!(!text.contains('\u{1b}') && !text.contains("[0;31m"), "got: {text:?}");
            assert!(!text.contains("50%"), "got: {text:?}");
            assert!(text.contains("100%|#####| 2/2"), "got: {text:?}");
            assert!(text.contains("Error (ValueError): bad value"), "got: {text:?}");
            assert!(text.contains("ValueError: bad value"), "got: {text:?}");
        }
    }

    #[test]
    fn test_rendering_source_emits_code_without_outputs() {
        let doc = JupyterExtractor::build_internal_document(&rendering_sample(), JupyterCellRendering::Source, false)
            .unwrap();
        assert!(
            doc.elements
                .iter()
                .any(|e| matches!(e.kind, ElementKind::Code) && e.text.contains("print('hello world')")),
            "source rendering keeps the code cell"
        );
        assert!(
            !doc.elements.iter().any(|e| e.text.contains("[output_type:")),
            "source rendering suppresses saved outputs"
        );
    }

    #[test]
    fn test_rendering_outputs_emits_outputs_without_code() {
        let doc = JupyterExtractor::build_internal_document(&rendering_sample(), JupyterCellRendering::Outputs, false)
            .unwrap();
        assert!(
            !doc.elements.iter().any(|e| matches!(e.kind, ElementKind::Code)),
            "outputs rendering suppresses the code source"
        );
        assert!(
            doc.elements.iter().any(|e| e.text.contains("hello world")),
            "outputs rendering keeps the saved output text"
        );
        assert!(
            !doc.elements.iter().any(|e| e.text.contains("[output_type:")),
            "outputs rendering does not expose diagnostic markers"
        );
    }

    #[test]
    fn test_rendering_both_emits_code_and_outputs() {
        let doc =
            JupyterExtractor::build_internal_document(&rendering_sample(), JupyterCellRendering::Both, false).unwrap();
        assert!(
            doc.elements.iter().any(|e| matches!(e.kind, ElementKind::Code)),
            "both rendering keeps the code source"
        );
        assert!(doc.elements.iter().any(|e| e.text.contains("hello world")));
        assert!(!doc.elements.iter().any(|e| e.text.contains("[output_type:")));
    }

    /// A `text/latex` output is the equation itself. It becomes a formula, and
    /// the `text/plain` repr beside it does not also appear.
    #[test]
    fn test_latex_output_becomes_a_formula() {
        use crate::types::internal::ElementKind;

        let notebook: Value = serde_json::from_str(
            r##"{
            "cells": [
                {"cell_type": "code", "source": ["sympy.sqrt(2)"], "metadata": {}, "execution_count": 1,
                 "outputs": [{"output_type": "execute_result", "execution_count": 1, "metadata": {},
                              "data": {"text/latex": ["$\\displaystyle \\sqrt{2}$"], "text/plain": ["sqrt(2)"]}}]}
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"##,
        )
        .unwrap();
        let doc = JupyterExtractor::build_internal_document(&notebook, JupyterCellRendering::Outputs, false).unwrap();

        let formulas: Vec<&str> = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Formula))
            .map(|e| e.text.as_str())
            .collect();
        assert_eq!(formulas, vec!["\\displaystyle \\sqrt{2}"], "delimiters are stripped");
        assert!(
            !doc.elements.iter().any(|e| e.text.contains("sqrt(2)")),
            "the plain repr of the same result does not appear twice"
        );
    }

    /// Plain output keeps the `text/plain` repr, as it does for every other
    /// rich representation.
    #[test]
    fn test_latex_output_falls_back_to_plain_text() {
        use crate::types::internal::ElementKind;

        let notebook: Value = serde_json::from_str(
            r##"{
            "cells": [
                {"cell_type": "code", "source": ["sympy.sqrt(2)"], "metadata": {}, "execution_count": 1,
                 "outputs": [{"output_type": "execute_result", "execution_count": 1, "metadata": {},
                              "data": {"text/latex": ["$\\displaystyle \\sqrt{2}$"], "text/plain": ["sqrt(2)"]}}]}
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"##,
        )
        .unwrap();
        let doc = JupyterExtractor::build_internal_document(&notebook, JupyterCellRendering::Outputs, true).unwrap();

        assert!(!doc.elements.iter().any(|e| matches!(e.kind, ElementKind::Formula)));
        assert!(doc.elements.iter().any(|e| e.text.contains("sqrt(2)")));
    }

    /// Markdown cells go through the shared markdown parser, so display math in
    /// a cell already becomes a formula. This pins that path.
    #[test]
    fn test_markdown_cell_display_math_becomes_a_formula() {
        use crate::types::internal::ElementKind;

        let notebook: Value = serde_json::from_str(
            r##"{
            "cells": [
                {"cell_type": "markdown", "source": ["Einstein wrote $$E = mc^2$$ in 1905.\n\nInline $x^2$ stays."], "metadata": {}}
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"##,
        )
        .unwrap();
        let doc = JupyterExtractor::build_internal_document(&notebook, JupyterCellRendering::Both, false).unwrap();

        let formulas: Vec<&str> = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Formula))
            .map(|e| e.text.as_str())
            .collect();
        assert_eq!(
            formulas,
            vec!["E = mc^2"],
            "display math only; inline math stays in the text"
        );
        assert!(doc.elements.iter().any(|e| e.text.contains("$x^2$")));
    }

    #[test]
    fn test_markdown_cell_reuses_shared_parser() {
        let notebook: Value = serde_json::from_str(
            r##"{
            "cells": [
                {"cell_type": "markdown", "source": ["# Heading\n\nSome **bold** prose."], "metadata": {}}
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        }"##,
        )
        .unwrap();
        let doc = JupyterExtractor::build_internal_document(&notebook, JupyterCellRendering::Both, false).unwrap();
        assert!(
            doc.elements
                .iter()
                .any(|e| matches!(e.kind, ElementKind::Heading { .. }) && e.text.contains("Heading")),
            "markdown cells render through the shared MarkdownExtractor (heading element present)"
        );
    }

    #[test]
    fn should_prefer_text_notebook_cell_language_over_kernel_language() {
        let notebook = TextNotebook {
            metadata: json!({"kernelspec": {"language": "python"}})
                .as_object()
                .expect("notebook metadata is an object")
                .clone(),
            cells: vec![crate::extractors::myst::TextNotebookCell {
                cell_type: TextNotebookCellType::Code,
                source: "println(42)".to_string(),
                language: Some("julia".to_string()),
                tags: Vec::new(),
            }],
        };
        let notebook_json = JupyterExtractor::text_notebook_json(notebook);
        assert_eq!(notebook_json["cells"][0]["metadata"]["language"], "julia");

        let document = JupyterExtractor::build_internal_document(&notebook_json, JupyterCellRendering::Both, false)
            .expect("text notebook renders");
        let code = document
            .elements
            .iter()
            .find(|element| matches!(element.kind, ElementKind::Code))
            .expect("code cell renders as code");
        assert_eq!(
            code.attributes
                .as_ref()
                .and_then(|attrs| attrs.get("language"))
                .map(String::as_str),
            Some("julia")
        );
    }

    #[test]
    fn should_not_resolve_saved_eval_to_html() {
        let mut notebook = json!({
            "cells": [{
                "cell_type": "markdown",
                "source": "Value: {eval}`answer`",
                "metadata": {"user_expressions": [{
                    "expression": "answer",
                    "result": {
                        "status": "ok",
                        "data": {
                            "text/html": "<script>alert(1)</script>",
                            "text/markdown": "**safe**",
                            "text/plain": "safe"
                        }
                    }
                }]}
            }]
        });

        JupyterExtractor::prepare_notebook(&mut notebook, false, false, &mut test_budget())
            .expect("notebook preparation succeeds");

        assert_eq!(notebook["cells"][0]["source"], "Value: **safe**\n");
    }

    #[test]
    fn plain_mode_should_resolve_saved_eval_to_plain_text() {
        let mut notebook = json!({
            "cells": [{
                "cell_type": "markdown",
                "source": "Value: {eval}`answer`",
                "metadata": {"user_expressions": [{
                    "expression": "answer",
                    "result": {"status": "ok", "data": {
                        "text/html": "<strong>html</strong>",
                        "text/markdown": "**markdown**",
                        "text/latex": "$latex$",
                        "text/plain": "plain"
                    }}
                }]}
            }]
        });

        JupyterExtractor::prepare_notebook(&mut notebook, false, true, &mut test_budget())
            .expect("notebook preparation succeeds");

        assert_eq!(notebook["cells"][0]["source"], "Value: plain\n");
    }

    #[test]
    fn should_redact_saved_result_payload_for_hidden_cell() {
        let mut notebook = json!({
            "cells": [{
                "cell_type": "markdown",
                "source": "secret",
                "metadata": {
                    "tags": ["remove-cell"],
                    "user_expressions": [{
                        "expression": "secret",
                        "result": {"status": "ok", "data": {"text/plain": "credential"}}
                    }]
                }
            }]
        });

        JupyterExtractor::prepare_notebook(&mut notebook, true, false, &mut test_budget())
            .expect("notebook preparation succeeds");
        let metadata = JupyterExtractor::cell_metadata(&notebook["cells"][0], 0);

        assert!(metadata["user_expressions"][0].get("expression").is_none());
        assert_eq!(metadata["user_expressions"][0]["result"]["status"], "ok");
        assert!(metadata["user_expressions"][0]["result"].get("data").is_none());
    }

    #[tokio::test]
    async fn should_reject_saved_eval_expansion_over_content_limit() {
        let notebook = json!({
            "cells": [{
                "cell_type": "markdown",
                "source": "{eval}`large` {eval}`large` {eval}`large`",
                "metadata": {"user_expressions": [{
                    "expression": "large",
                    "result": {"status": "ok", "data": {"text/plain": "x".repeat(128)}}
                }]}
            }],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        });
        let content = serde_json::to_vec(&notebook).expect("notebook serializes");
        let config = ExtractionConfig {
            security_limits: Some(SecurityLimits {
                max_content_size: content.len() + 64,
                ..SecurityLimits::default()
            }),
            ..ExtractionConfig::default()
        };

        let error = JupyterExtractor::new()
            .extract_content(&content, "application/x-ipynb+json", &config)
            .await
            .expect_err("expanded eval content must consume the security budget");

        assert!(error.to_string().contains("Content too large"));
    }

    #[tokio::test]
    async fn should_reject_notebook_over_iteration_limit() {
        let notebook = json!({
            "cells": [
                {"cell_type": "markdown", "source": "one", "metadata": {}},
                {"cell_type": "markdown", "source": "two", "metadata": {}}
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        });
        let content = serde_json::to_vec(&notebook).expect("notebook serializes");
        let config = ExtractionConfig {
            security_limits: Some(SecurityLimits {
                max_content_size: content.len() * 4,
                max_iterations: 1,
                ..SecurityLimits::default()
            }),
            ..ExtractionConfig::default()
        };

        let error = JupyterExtractor::new()
            .extract_content(&content, "application/x-ipynb+json", &config)
            .await
            .expect_err("cell traversal must consume the iteration budget");

        assert!(error.to_string().contains("Too many iterations"));
    }

    #[test]
    fn invalid_base64_should_not_consume_content_budget() {
        let limits = SecurityLimits {
            max_content_size: 3,
            ..SecurityLimits::default()
        };
        let mut budget = SecurityBudget::from_limits(&limits);

        let decoded = JupyterExtractor::decode_base64_bounded("!!!!", &mut budget)
            .expect("invalid base64 is skipped without failing extraction");

        assert!(decoded.is_none());
        budget.account_text(3).expect("invalid base64 must not consume budget");
    }

    #[tokio::test]
    async fn source_rendering_should_skip_output_image_payloads() {
        let image = base64::engine::general_purpose::STANDARD.encode([0_u8; 128]);
        let notebook = json!({
            "cells": [{
                "cell_type": "code",
                "source": "x = 1",
                "metadata": {},
                "outputs": [{"output_type": "display_data", "data": {"image/png": image}}]
            }],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        });
        let content = serde_json::to_vec(&notebook).expect("notebook serializes");
        let mut config = ExtractionConfig {
            jupyter_cell_rendering: JupyterCellRendering::Source,
            ..ExtractionConfig::default()
        };
        config.security_limits = Some(SecurityLimits {
            max_content_size: content.len() + 16,
            ..SecurityLimits::default()
        });

        let document = JupyterExtractor::new()
            .extract_content(&content, "application/x-ipynb+json", &config)
            .await
            .expect("omitted output payload must not consume source-only budget");

        assert!(document.images.is_empty());
    }

    #[test]
    fn plain_output_should_use_plain_representation_from_mixed_bundle() {
        let data = json!({
            "text/html": "<strong>rich</strong>",
            "text/markdown": "**rich**",
            "text/latex": "$rich$",
            "text/plain": "plain"
        });

        let representation =
            JupyterExtractor::richest_output_representation(data.as_object().expect("data is an object"), true);

        assert!(matches!(representation, Some(OutputRepresentation::Text(text)) if text == "plain"));
    }
}
