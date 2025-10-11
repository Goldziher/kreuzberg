use crate::pptx::streaming::extractor::StreamingPptxExtractorDTO;
use crate::pptx::types::PptxExtractionResultDTO;
use pyo3::prelude::*;

#[pyclass]
pub struct PptxExtractorDTO {
    inner: StreamingPptxExtractorDTO,
}

#[pymethods]
impl PptxExtractorDTO {
    #[new]
    pub fn new(extract_images: Option<bool>) -> Self {
        Self {
            inner: StreamingPptxExtractorDTO::new(extract_images, None),
        }
    }

    pub fn extract_from_path(&self, path: String) -> PyResult<PptxExtractionResultDTO> {
        self.inner.extract_from_path(path)
    }

    pub fn extract_from_bytes(&self, data: &[u8]) -> PyResult<PptxExtractionResultDTO> {
        let temp_path = "/tmp/temp_pptx_extract.pptx";
        std::fs::write(temp_path, data)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to write temp file: {}", e)))?;

        let result = self.extract_from_path(temp_path.to_string())?;

        let _ = std::fs::remove_file(temp_path);

        Ok(result)
    }
}
