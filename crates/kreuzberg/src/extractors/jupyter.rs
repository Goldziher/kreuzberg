//! Jupyter Notebook extractor for .ipynb files.
//!
//! This extractor provides native Rust parsing of Jupyter notebooks,
//! extracting:
//! - Notebook metadata (kernelspec, language_info, nbformat)
//! - Cell content (markdown and code cells in order)
//! - Cell outputs (text, HTML, images)
//! - Cell metadata (execution_count, tags)
//!
//! Requires the `office` feature.

#[cfg(feature = "office")]
use crate::Result;
#[cfg(feature = "office")]
use crate::core::config::ExtractionConfig;
#[cfg(feature = "office")]
use crate::plugins::{DocumentExtractor, Plugin};
#[cfg(feature = "office")]
use crate::types::{ExtractionResult, Metadata};
#[cfg(feature = "office")]
use async_trait::async_trait;
#[cfg(feature = "office")]
use serde_json::{Value, json};
#[cfg(feature = "office")]
use std::collections::HashMap;

/// Jupyter Notebook extractor.
///
/// Extracts content from Jupyter notebook JSON files, including:
/// - Notebook metadata (kernel, language, nbformat version)
/// - Cell content (code and markdown)
/// - Cell outputs (text, HTML, etc.)
/// - Cell-level metadata (tags, execution counts)
#[cfg(feature = "office")]
pub struct JupyterExtractor;

#[cfg(feature = "office")]
impl JupyterExtractor {
    /// Create a new Jupyter extractor.
    pub fn new() -> Self {
        Self
    }

    /// Extract content from a Jupyter notebook.
    fn extract_notebook(content: &[u8]) -> Result<(String, HashMap<String, Value>)> {
        let notebook: Value = serde_json::from_slice(content)
            .map_err(|e| crate::KreuzbergError::parsing(format!("Failed to parse JSON: {}", e)))?;

        let mut extracted_content = String::new();
        let mut metadata = HashMap::new();

        if let Some(notebook_metadata) = notebook.get("metadata").and_then(|m| m.as_object()) {
            if let Some(kernelspec) = notebook_metadata.get("kernelspec")
                && let Some(name) = kernelspec.get("name").and_then(|n| n.as_str())
            {
                extracted_content.push_str(&format!("Kernelspec: {}\n", name));
                metadata.insert("kernelspec".to_string(), kernelspec.clone());
            }

            if let Some(language_info) = notebook_metadata.get("language_info")
                && let Some(name) = language_info.get("name").and_then(|n| n.as_str())
            {
                extracted_content.push_str(&format!("Language: {}\n", name));
                metadata.insert("language_info".to_string(), language_info.clone());
            }
        }

        if let Some(nbformat) = notebook.get("nbformat") {
            extracted_content.push_str(&format!("NBFormat: {}\n", nbformat));
            metadata.insert("nbformat".to_string(), nbformat.clone());
        }

        extracted_content.push('\n');

        if let Some(cells) = notebook.get("cells").and_then(|c| c.as_array()) {
            for (cell_idx, cell) in cells.iter().enumerate() {
                Self::extract_cell(cell, cell_idx, &mut extracted_content, &mut metadata)?;
            }
        }

        Ok((extracted_content, metadata))
    }

    /// Extract content from a single cell.
    fn extract_cell(
        cell: &Value,
        cell_idx: usize,
        content: &mut String,
        _metadata: &mut HashMap<String, Value>,
    ) -> Result<()> {
        let cell_type = cell.get("cell_type").and_then(|t| t.as_str()).unwrap_or("unknown");

        let cell_id = cell.get("id").and_then(|id| id.as_str());

        if let Some(id) = cell_id {
            content.push_str(&format!(":::: {{#{} .cell .{}}}\n", id, cell_type));
        } else {
            content.push_str(&format!(":::: {{#cell_{} .cell .{}}}\n", cell_idx, cell_type));
        }

        if let Some(cell_metadata) = cell.get("metadata").and_then(|m| m.as_object())
            && let Some(tags) = cell_metadata.get("tags").and_then(|t| t.as_array())
        {
            let tag_strs: Vec<String> = tags
                .iter()
                .filter_map(|tag| tag.as_str().map(|s| s.to_string()))
                .collect();
            if !tag_strs.is_empty() {
                content.push_str(&format!(" tags=[{}]", tag_strs.join(", ")));
            }
        }
        content.push('\n');

        match cell_type {
            "markdown" => Self::extract_markdown_cell(cell, content)?,
            "code" => Self::extract_code_cell(cell, content)?,
            "raw" => Self::extract_raw_cell(cell, content)?,
            _ => {
                content.push_str(&format!("Unknown cell type: {}\n", cell_type));
            }
        }

        content.push_str("::::\n\n");
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
    fn extract_code_cell(cell: &Value, content: &mut String) -> Result<()> {
        if let Some(exec_count) = cell.get("execution_count")
            && !exec_count.is_null()
        {
            content.push_str(&format!("::: {{execution_count={}}}\n", exec_count));
        }

        if let Some(source) = cell.get("source") {
            let cell_text = Self::extract_source(source);
            content.push_str("```python\n");
            content.push_str(&cell_text);
            content.push_str("```\n");
        }

        if let Some(outputs) = cell.get("outputs").and_then(|o| o.as_array()) {
            for output in outputs {
                Self::extract_output(output, content)?;
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
            Value::Array(arr) => arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(""),
            _ => String::new(),
        }
    }

    /// Extract output from a cell.
    fn extract_output(output: &Value, content: &mut String) -> Result<()> {
        let output_type = output.get("output_type").and_then(|t| t.as_str()).unwrap_or("unknown");

        content.push_str(&format!("::: {{.output .{}", output_type));

        if let Some(exec_count) = output.get("execution_count")
            && !exec_count.is_null()
        {
            content.push_str(&format!(" execution_count={}", exec_count));
        }

        content.push_str("}\n");

        match output_type {
            "stream" => Self::extract_stream_output(output, content)?,
            "execute_result" | "display_data" => Self::extract_data_output(output, content)?,
            "error" => Self::extract_error_output(output, content)?,
            _ => {
                content.push_str(&format!("Unknown output type: {}\n", output_type));
            }
        }

        content.push_str(":::\n");
        Ok(())
    }

    /// Extract stream output (stdout, stderr).
    fn extract_stream_output(output: &Value, content: &mut String) -> Result<()> {
        if let Some(name) = output.get("name").and_then(|n| n.as_str()) {
            content.push_str(&format!("Stream: {}\n", name));
        }

        if let Some(text) = output.get("text") {
            let text_content = Self::extract_source(text);
            content.push_str(&text_content);
        }

        Ok(())
    }

    /// Extract data output (execute_result or display_data).
    fn extract_data_output(output: &Value, content: &mut String) -> Result<()> {
        if let Some(data) = output.get("data").and_then(|d| d.as_object()) {
            let mime_types = vec![
                "text/markdown",
                "text/html",
                "image/svg+xml",
                "image/png",
                "image/jpeg",
                "application/json",
                "text/plain",
            ];

            for mime_type in mime_types {
                if let Some(mime_content) = data.get(mime_type) {
                    content.push_str(&format!("MIME: {}\n", mime_type));
                    let mime_text = Self::extract_source(mime_content);
                    if !mime_text.is_empty() {
                        content.push_str(&mime_text);
                        content.push('\n');
                    }
                }
            }
        }

        Ok(())
    }

    /// Extract error output.
    fn extract_error_output(output: &Value, content: &mut String) -> Result<()> {
        if let Some(ename) = output.get("ename").and_then(|e| e.as_str()) {
            content.push_str(&format!("Error: {}\n", ename));
        }

        if let Some(evalue) = output.get("evalue").and_then(|e| e.as_str()) {
            content.push_str(&format!("Value: {}\n", evalue));
        }

        if let Some(traceback) = output.get("traceback").and_then(|t| t.as_array()) {
            for line in traceback {
                if let Some(line_str) = line.as_str() {
                    content.push_str(line_str);
                    content.push('\n');
                }
            }
        }

        Ok(())
    }
}

#[cfg(feature = "office")]
impl Default for JupyterExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "office")]
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
        "Kreuzberg Team"
    }
}

#[cfg(feature = "office")]
#[async_trait]
impl DocumentExtractor for JupyterExtractor {
    #[cfg_attr(
        feature = "otel",
        tracing::instrument(
            skip(self, content, _config),
            fields(
                extractor.name = self.name(),
                content.size_bytes = content.len(),
            )
        )
    )]
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let (extracted_content, additional_metadata) = Self::extract_notebook(content)?;

        let mut metadata_additional = HashMap::new();
        for (key, value) in additional_metadata {
            metadata_additional.insert(key, json!(value));
        }

        Ok(ExtractionResult {
            content: extracted_content,
            mime_type: mime_type.to_string(),
            metadata: Metadata {
                additional: metadata_additional,
                ..Default::default()
            },
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        })
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

    #[test]
    fn test_jupyter_extractor_plugin_interface() {
        let extractor = JupyterExtractor::new();
        assert_eq!(extractor.name(), "jupyter-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 50);
        assert!(extractor.supported_mime_types().contains(&"application/x-ipynb+json"));
    }
}
