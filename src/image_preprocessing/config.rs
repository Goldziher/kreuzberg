use pyo3::prelude::*;

/// Configuration for image extraction DTO (Data Transfer Object)
#[pyclass]
#[derive(Debug, Clone)]
pub struct ExtractionConfigDTO {
    #[pyo3(get, set)]
    pub target_dpi: i32,
    #[pyo3(get, set)]
    pub max_image_dimension: i32,
    #[pyo3(get, set)]
    pub auto_adjust_dpi: bool,
    #[pyo3(get, set)]
    pub min_dpi: i32,
    #[pyo3(get, set)]
    pub max_dpi: i32,
}

#[pymethods]
impl ExtractionConfigDTO {
    #[new]
    #[pyo3(signature = (target_dpi=300, max_image_dimension=4096, auto_adjust_dpi=false, min_dpi=72, max_dpi=600))]
    #[must_use]
    pub const fn new(
        target_dpi: i32,
        max_image_dimension: i32,
        auto_adjust_dpi: bool,
        min_dpi: i32,
        max_dpi: i32,
    ) -> Self {
        Self {
            target_dpi,
            max_image_dimension,
            auto_adjust_dpi,
            min_dpi,
            max_dpi,
        }
    }
}
