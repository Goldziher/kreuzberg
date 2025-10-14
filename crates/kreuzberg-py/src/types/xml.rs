use pyo3::prelude::*;

use kreuzberg::types::XmlExtractionResult as CoreXmlResult;

#[pyclass(name = "XmlExtractionResult", module = "kreuzberg._internal_bindings")]
#[derive(Clone)]
pub struct PyXmlExtractionResult {
    inner: CoreXmlResult,
}

#[pymethods]
impl PyXmlExtractionResult {
    #[getter]
    fn content(&self) -> String {
        self.inner.content.clone()
    }

    #[getter]
    fn element_count(&self) -> usize {
        self.inner.element_count
    }

    #[getter]
    fn unique_elements(&self) -> Vec<String> {
        self.inner.unique_elements.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "XmlExtractionResult(content_length={}, element_count={}, unique_elements={})",
            self.inner.content.len(),
            self.inner.element_count,
            self.inner.unique_elements.len()
        )
    }
}

impl From<CoreXmlResult> for PyXmlExtractionResult {
    fn from(inner: CoreXmlResult) -> Self {
        Self { inner }
    }
}
