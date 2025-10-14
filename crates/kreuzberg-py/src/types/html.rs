use pyo3::prelude::*;
use std::collections::HashMap;

use kreuzberg::extraction::html::{
    ExtractedInlineImage as CoreExtractedInlineImage, HtmlExtractionResult as CoreHtmlResult,
};

#[pyclass(name = "ExtractedInlineImage", module = "kreuzberg._internal_bindings")]
#[derive(Clone)]
pub struct PyExtractedInlineImage {
    inner: CoreExtractedInlineImage,
}

#[pymethods]
impl PyExtractedInlineImage {
    #[getter]
    fn data(&self) -> Vec<u8> {
        self.inner.data.clone()
    }

    #[getter]
    fn format(&self) -> String {
        self.inner.format.clone()
    }

    #[getter]
    fn filename(&self) -> Option<String> {
        self.inner.filename.clone()
    }

    #[getter]
    fn description(&self) -> Option<String> {
        self.inner.description.clone()
    }

    #[getter]
    fn dimensions(&self) -> Option<(u32, u32)> {
        self.inner.dimensions
    }

    #[getter]
    fn attributes(&self) -> HashMap<String, String> {
        self.inner.attributes.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "ExtractedInlineImage(format='{}', size={}, dimensions={:?})",
            self.inner.format,
            self.inner.data.len(),
            self.inner.dimensions
        )
    }
}

impl From<CoreExtractedInlineImage> for PyExtractedInlineImage {
    fn from(inner: CoreExtractedInlineImage) -> Self {
        Self { inner }
    }
}

#[pyclass(name = "HtmlExtractionResult", module = "kreuzberg._internal_bindings")]
#[derive(Clone)]
pub struct PyHtmlExtractionResult {
    inner: CoreHtmlResult,
}

#[pymethods]
impl PyHtmlExtractionResult {
    #[getter]
    fn markdown(&self) -> String {
        self.inner.markdown.clone()
    }

    #[getter]
    fn images(&self) -> Vec<PyExtractedInlineImage> {
        self.inner
            .images
            .iter()
            .map(|img| PyExtractedInlineImage::from(img.clone()))
            .collect()
    }

    #[getter]
    fn warnings(&self) -> Vec<String> {
        self.inner.warnings.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "HtmlExtractionResult(markdown_length={}, images={}, warnings={})",
            self.inner.markdown.len(),
            self.inner.images.len(),
            self.inner.warnings.len()
        )
    }
}

impl From<CoreHtmlResult> for PyHtmlExtractionResult {
    fn from(inner: CoreHtmlResult) -> Self {
        Self { inner }
    }
}
