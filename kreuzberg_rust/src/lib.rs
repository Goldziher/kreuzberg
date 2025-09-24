use pyo3::prelude::*;

mod image_preprocessing;

/// A Python module implemented in Rust for high-performance image preprocessing.
#[pymodule]
fn kreuzberg_rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(image_preprocessing::normalize_image_dpi_rust, m)?)?;
    m.add_class::<image_preprocessing::ImagePreprocessingMetadata>()?;
    m.add_class::<image_preprocessing::ExtractionConfig>()?;
    Ok(())
}
