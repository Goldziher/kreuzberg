//! Convenience batch extractor wrapping streaming implementation

use crate::pptx::streaming::extractor::StreamingPptxExtractor;
use crate::pptx::types::PptxExtractionResult;
use pyo3::prelude::*;

/// Batch PPTX extractor (convenience wrapper around streaming extractor)
#[pyclass]
pub struct PptxExtractor {
    inner: StreamingPptxExtractor,
}

#[pymethods]
impl PptxExtractor {
    #[new]
    pub fn new(extract_images: Option<bool>) -> Self {
        Self {
            inner: StreamingPptxExtractor::new(extract_images, None),
        }
    }

    /// Extract PPTX from file path
    pub fn extract_from_path(&self, path: String) -> PyResult<PptxExtractionResult> {
        self.inner.extract_from_path(path)
    }

    /// Extract PPTX from bytes (writes to temp file)
    pub fn extract_from_bytes(&self, data: &[u8]) -> PyResult<PptxExtractionResult> {
        let temp_path = "/tmp/temp_pptx_extract.pptx";
        std::fs::write(temp_path, data)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to write temp file: {}", e)))?;

        let result = self.extract_from_path(temp_path.to_string())?;

        // Clean up temp file
        let _ = std::fs::remove_file(temp_path);

        Ok(result)
    }
}
