//! Shared test helpers for integration tests.
//!
//! This module provides common utilities for loading test files,
//! making assertions, and setting up test environments.

#![allow(dead_code)]

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use xberg::{
    ErrorMetadata, ExtractInput, ExtractedDocument, ExtractionConfig, ExtractionErrorItem, FileExtractionConfig,
    Metadata, Result, XbergError, extract, extract_batch,
};

/// Batch URI input used by integration tests.
#[derive(Debug, Clone)]
pub struct UriBatchInput {
    pub path: PathBuf,
    pub config: Option<FileExtractionConfig>,
}

/// In-memory bytes input used by integration tests.
#[derive(Debug, Clone)]
pub struct BytesInput {
    pub content: Vec<u8>,
    pub mime_type: String,
    pub config: Option<FileExtractionConfig>,
}

/// Extract a URI-backed document through the public unified API and return the single result.
pub async fn extract_uri_document(
    path: impl AsRef<Path>,
    mime_type: Option<&str>,
    config: &ExtractionConfig,
) -> Result<ExtractedDocument> {
    let mut input = ExtractInput::from_uri(path.as_ref().to_string_lossy().into_owned());
    input.mime_type = mime_type.map(str::to_string);
    single_result(extract(input, config).await)
}

/// Extract bytes through the public unified API and return the single result.
pub async fn extract_bytes_document(
    content: &[u8],
    mime_type: &str,
    config: &ExtractionConfig,
) -> Result<ExtractedDocument> {
    single_result(extract(ExtractInput::from_bytes(content.to_vec(), mime_type, None), config).await)
}

/// Blocking URI-backed extraction adapter for synchronous integration tests.
pub fn extract_uri_document_blocking(
    path: impl AsRef<Path>,
    mime_type: Option<&str>,
    config: &ExtractionConfig,
) -> Result<ExtractedDocument> {
    runtime()?.block_on(extract_uri_document(path, mime_type, config))
}

/// Blocking bytes extraction adapter for synchronous integration tests.
pub fn extract_bytes_document_blocking(
    content: &[u8],
    mime_type: &str,
    config: &ExtractionConfig,
) -> Result<ExtractedDocument> {
    runtime()?.block_on(extract_bytes_document(content, mime_type, config))
}

/// Extract URI-backed inputs through the public unified batch API and return result-shaped items.
pub async fn extract_uri_documents(
    items: Vec<UriBatchInput>,
    config: &ExtractionConfig,
) -> Result<Vec<ExtractedDocument>> {
    let inputs = items
        .into_iter()
        .map(|item| {
            let mut input = ExtractInput::from_uri(item.path.to_string_lossy().into_owned());
            input.config = item.config;
            input
        })
        .collect::<Vec<_>>();
    let input_count = inputs.len();
    batch_results(extract_batch(inputs, config).await?, input_count)
}

/// Extract byte inputs through the public unified batch API and return result-shaped items.
pub async fn extract_bytes_documents(
    items: Vec<BytesInput>,
    config: &ExtractionConfig,
) -> Result<Vec<ExtractedDocument>> {
    let inputs = items
        .into_iter()
        .map(|item| {
            let mut input = ExtractInput::from_bytes(item.content, item.mime_type, None);
            input.config = item.config;
            input
        })
        .collect::<Vec<_>>();
    let input_count = inputs.len();
    batch_results(extract_batch(inputs, config).await?, input_count)
}

/// Blocking URI-backed batch extraction adapter for synchronous integration tests.
pub fn extract_uri_documents_blocking(
    items: Vec<UriBatchInput>,
    config: &ExtractionConfig,
) -> Result<Vec<ExtractedDocument>> {
    runtime()?.block_on(extract_uri_documents(items, config))
}

/// Blocking bytes batch extraction adapter for synchronous integration tests.
pub fn extract_bytes_documents_blocking(
    items: Vec<BytesInput>,
    config: &ExtractionConfig,
) -> Result<Vec<ExtractedDocument>> {
    runtime()?.block_on(extract_bytes_documents(items, config))
}

fn runtime() -> Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(XbergError::from)
}

fn single_result(output: Result<xberg::ExtractionResult>) -> Result<ExtractedDocument> {
    let mut output = output?;
    if let Some(error) = output.errors.into_iter().next() {
        return Err(XbergError::Other(error.message));
    }
    if output.results.len() != 1 {
        return Err(XbergError::Other(format!(
            "expected one extraction result, got {}",
            output.results.len()
        )));
    }
    Ok(output.results.remove(0))
}

fn batch_results(output: xberg::ExtractionResult, input_count: usize) -> Result<Vec<ExtractedDocument>> {
    let mut slots = vec![None; input_count];
    for result in output.results {
        let index = source_index(&result).unwrap_or_else(|| first_empty_slot(&slots));
        if let Some(slot) = slots.get_mut(index) {
            *slot = Some(result);
        }
    }
    for error in output.errors {
        if let Some(slot) = slots.get_mut(error.index) {
            *slot = Some(error_extraction_result(&error, None));
        }
    }
    Ok(slots.into_iter().flatten().collect())
}

fn source_index(result: &ExtractedDocument) -> Option<usize> {
    result
        .metadata
        .additional
        .get("source_index")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn first_empty_slot(slots: &[Option<ExtractedDocument>]) -> usize {
    slots.iter().position(Option::is_none).unwrap_or(slots.len())
}

fn error_extraction_result(error: &ExtractionErrorItem, elapsed_ms: Option<u64>) -> ExtractedDocument {
    let metadata = Metadata {
        error: Some(ErrorMetadata {
            error_type: error.error_type.clone(),
            message: error.message.clone(),
        }),
        extraction_duration_ms: elapsed_ms,
        ..Default::default()
    };

    let mut result = ExtractedDocument::default();
    result.content = format!("Error: {}", error.message);
    result.mime_type = Cow::Borrowed("text/plain");
    result.metadata = metadata;
    result
}

/// Get the test_documents directory path.
///
/// This assumes the test is running from the workspace root.
pub fn get_test_documents_dir() -> PathBuf {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    workspace_root.join("test_documents")
}

/// Get the full path to a test file.
///
/// # Arguments
///
/// * `relative_path` - Path relative to test_documents/
pub fn get_test_file_path(relative_path: &str) -> PathBuf {
    get_test_documents_dir().join(relative_path)
}

/// Assert that extraction result contains non-empty content.
///
/// This is a common assertion for most extraction tests - we want
/// to verify that *something* was extracted, even if we don't know
/// the exact content.
pub fn assert_non_empty_content(result: &ExtractedDocument) {
    assert!(
        !result.content.trim().is_empty(),
        "Extraction result should have non-empty content, got: '{}'",
        result.content
    );
}

/// Assert that extraction result has expected MIME type.
pub fn assert_mime_type(result: &ExtractedDocument, expected: &str) {
    assert_eq!(
        result.mime_type, expected,
        "Expected MIME type '{}', got '{}'",
        expected, result.mime_type
    );
}

/// Skip test if file doesn't exist (for optional test files).
///
/// Returns true if test should be skipped.
pub fn skip_if_missing(relative_path: &str) -> bool {
    let path = get_test_file_path(relative_path);
    if !path.exists() {
        tracing::debug!("Skipping test: file not found at {}", path.display());
        return true;
    }
    false
}

/// Check if test documents directory exists and has files.
///
/// This is useful for CI environments where test_documents might
/// be a git submodule that hasn't been initialized.
pub fn test_documents_available() -> bool {
    let dir = get_test_documents_dir();
    dir.exists() && dir.read_dir().map(|mut d| d.next().is_some()).unwrap_or(false)
}

/// Assert that content length is above a minimum threshold.
///
/// This is useful for smoke testing - ensuring substantial content
/// was extracted without needing to verify exact text.
pub fn assert_min_content_length(result: &ExtractedDocument, min_length: usize) {
    assert!(
        result.content.len() >= min_length,
        "Expected content length >= {}, got {}. Content preview: '{}'",
        min_length,
        result.content.len(),
        result.content.chars().take(200).collect::<String>()
    );
}

/// Assert that content contains at least one of the given substrings.
pub fn assert_content_contains_any(result: &ExtractedDocument, substrings: &[&str]) {
    let found = substrings.iter().any(|s| result.content.contains(s));
    assert!(
        found,
        "Expected content to contain at least one of {:?}, but found none",
        substrings
    );
}

/// Assert that extraction result has at least one table.
pub fn assert_has_tables(result: &ExtractedDocument) {
    assert!(
        !result.tables.is_empty(),
        "Expected result to have tables, but found none"
    );
}

/// Create a test configuration with OCR enabled.
pub fn test_config_with_ocr() -> xberg::core::config::ExtractionConfig {
    use xberg::core::config::{ExtractionConfig, OcrConfig};

    ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        force_ocr: false,
        ..Default::default()
    }
}

#[cfg(feature = "pdf")]
pub mod pdf_helpers {
    use xberg::core::config::ExtractionConfig;
    use xberg::pdf::hierarchy::BoundingBox;

    /// Create a bounding box with simple coordinates.
    pub fn create_bounding_box(left: f32, top: f32, right: f32, bottom: f32) -> BoundingBox {
        BoundingBox {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Create a default extraction configuration for testing hierarchy extraction.
    ///
    /// # Returns
    ///
    /// A new ExtractionConfig with PDF hierarchy options enabled
    pub fn create_hierarchy_extraction_config() -> ExtractionConfig {
        ExtractionConfig::default()
    }
}

/// A custom renderer whose output is one fixed line for every document.
///
/// Register it with `register_renderer` and ask for `OutputFormat::Custom(name)`: `content` is
/// then `line`, so a test sees what a custom renderer's output does and does not carry.
pub struct FixedLineRenderer {
    pub name: &'static str,
    pub line: &'static str,
}

impl xberg::plugins::Plugin for FixedLineRenderer {
    fn name(&self) -> &str {
        self.name
    }
}

impl xberg::plugins::Renderer for FixedLineRenderer {
    fn render_result(&self, _result: &ExtractedDocument) -> Result<String> {
        Ok(self.line.to_string())
    }
}

/// A custom renderer whose output is the pages of the document it receives.
///
/// Register it with `register_renderer` and ask for `OutputFormat::Custom(name)`: `content` is
/// then [`pages_written`] for the pages the renderer received, so a test sees the page content a
/// registered renderer reads.
pub struct PageWritingRenderer {
    pub name: &'static str,
}

/// One line per page, `page <number>: <content>`, with the content in its Rust debug form.
pub fn pages_written(pages: &[xberg::types::PageContent]) -> String {
    pages
        .iter()
        .map(|page| format!("page {}: {:?}", page.page_number, page.content))
        .collect::<Vec<_>>()
        .join("\n")
}

impl xberg::plugins::Plugin for PageWritingRenderer {
    fn name(&self) -> &str {
        self.name
    }
}

impl xberg::plugins::Renderer for PageWritingRenderer {
    fn render_result(&self, result: &ExtractedDocument) -> Result<String> {
        Ok(pages_written(result.pages.as_deref().unwrap_or_default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_test_documents_dir() {
        let dir = get_test_documents_dir();
        assert!(dir.to_string_lossy().ends_with("test_documents"));
    }

    #[test]
    fn test_test_documents_available() {
        let available = test_documents_available();
        if !available {
            tracing::debug!("Warning: test_documents directory not available");
            tracing::debug!("This is expected in CI without git submodules initialized");
        }
    }
}
