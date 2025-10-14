use pyo3::prelude::*;

use kreuzberg::types::TextExtractionResult as CoreTextResult;

#[pyclass(name = "TextExtractionResult", module = "kreuzberg._internal_bindings")]
#[derive(Clone)]
pub struct PyTextExtractionResult {
    inner: CoreTextResult,
}

#[pymethods]
impl PyTextExtractionResult {
    #[getter]
    fn content(&self) -> String {
        self.inner.content.clone()
    }

    #[getter]
    fn line_count(&self) -> usize {
        self.inner.line_count
    }

    #[getter]
    fn word_count(&self) -> usize {
        self.inner.word_count
    }

    #[getter]
    fn character_count(&self) -> usize {
        self.inner.character_count
    }

    #[getter]
    fn headers(&self) -> Option<Vec<String>> {
        self.inner.headers.clone()
    }

    #[getter]
    fn links(&self) -> Option<Vec<(String, String)>> {
        self.inner.links.clone()
    }

    #[getter]
    fn code_blocks(&self) -> Option<Vec<(String, String)>> {
        self.inner.code_blocks.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "TextExtractionResult(content_length={}, line_count={}, word_count={})",
            self.inner.content.len(),
            self.inner.line_count,
            self.inner.word_count
        )
    }
}

impl From<CoreTextResult> for PyTextExtractionResult {
    fn from(inner: CoreTextResult) -> Self {
        Self { inner }
    }
}
