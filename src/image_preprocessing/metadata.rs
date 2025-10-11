use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, Clone)]
pub struct ImagePreprocessingMetadataDTO {
    #[pyo3(get)]
    pub original_dimensions: (u32, u32),
    #[pyo3(get)]
    pub original_dpi: (f64, f64),
    #[pyo3(get)]
    pub target_dpi: i32,
    #[pyo3(get)]
    pub scale_factor: f64,
    #[pyo3(get)]
    pub auto_adjusted: bool,
    #[pyo3(get)]
    pub final_dpi: i32,
    #[pyo3(get)]
    pub new_dimensions: Option<(u32, u32)>,
    #[pyo3(get)]
    pub resample_method: String,
    #[pyo3(get)]
    pub dimension_clamped: bool,
    #[pyo3(get)]
    pub calculated_dpi: Option<i32>,
    #[pyo3(get)]
    pub skipped_resize: bool,
    #[pyo3(get)]
    pub resize_error: Option<String>,
}
