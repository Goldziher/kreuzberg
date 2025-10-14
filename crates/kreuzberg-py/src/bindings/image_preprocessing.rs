use crate::error::to_py_err;
use numpy::{PyArray3, PyArrayMethods};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};

/// Normalize image DPI using pure Rust implementation
///
/// Takes numpy array (height, width, 3) and returns normalized array + metadata (MessagePack)
#[pyfunction]
#[pyo3(signature = (image_array, config_msgpack, dpi_info=None))]
pub fn normalize_image_dpi_msgpack<'py>(
    py: Python<'py>,
    image_array: &Bound<'py, PyArray3<u8>>,
    config_msgpack: &[u8],
    dpi_info: Option<&Bound<'py, PyDict>>,
) -> PyResult<(Bound<'py, PyArray3<u8>>, Bound<'py, PyBytes>)> {
    // Extract array dimensions and data
    let array_view = unsafe { image_array.as_array() };
    let (height, width, channels) = array_view.dim();

    if channels != 3 {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Expected 3 channels (RGB), got {}",
            channels
        )));
    }

    // Convert ndarray to flat RGB data (row-major)
    let mut rgb_data = Vec::with_capacity(height * width * 3);
    for y in 0..height {
        for x in 0..width {
            rgb_data.push(array_view[[y, x, 0]]);
            rgb_data.push(array_view[[y, x, 1]]);
            rgb_data.push(array_view[[y, x, 2]]);
        }
    }

    // Deserialize config from MessagePack
    let config: kreuzberg::ExtractionConfig = rmp_serde::from_slice(config_msgpack).map_err(|e| to_py_err(e.into()))?;

    // Extract DPI from dict if provided
    let current_dpi = dpi_info.and_then(|dpi_dict| {
        dpi_dict
            .get_item("dpi")
            .ok()
            .flatten()
            .and_then(|val| val.extract::<f64>().ok())
    });

    // Call pure Rust implementation
    let result =
        kreuzberg::image::normalize_image_dpi(&rgb_data, width, height, &config, current_dpi).map_err(to_py_err)?;

    // Convert result back to numpy array
    let (result_width, result_height) = result.dimensions;
    let result_array = PyArray3::<u8>::zeros(py, (result_height, result_width, 3), false);

    unsafe {
        let slice = result_array.as_slice_mut().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Failed to get array slice: {}", e))
        })?;

        // Copy RGB data back to numpy array (row-major)
        for y in 0..result_height {
            for x in 0..result_width {
                let src_idx = (y * result_width + x) * 3;
                let dst_idx = (y * result_width + x) * 3;
                slice[dst_idx] = result.rgb_data[src_idx];
                slice[dst_idx + 1] = result.rgb_data[src_idx + 1];
                slice[dst_idx + 2] = result.rgb_data[src_idx + 2];
            }
        }
    }

    // Serialize metadata to MessagePack
    let metadata_msgpack = rmp_serde::to_vec_named(&result.metadata).map_err(|e| to_py_err(e.into()))?;

    Ok((result_array, PyBytes::new(py, &metadata_msgpack)))
}

/// Calculate optimal DPI for image preprocessing
#[pyfunction]
#[pyo3(signature = (page_width, page_height, target_dpi, max_dimension, min_dpi=72, max_dpi=600))]
pub fn calculate_optimal_dpi(
    page_width: f64,
    page_height: f64,
    target_dpi: i32,
    max_dimension: i32,
    min_dpi: i32,
    max_dpi: i32,
) -> i32 {
    kreuzberg::image::calculate_optimal_dpi(page_width, page_height, target_dpi, max_dimension, min_dpi, max_dpi)
}
