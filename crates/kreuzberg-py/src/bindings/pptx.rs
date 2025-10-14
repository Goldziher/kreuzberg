use pyo3::prelude::*;
use pyo3::types::PyBytes;

use crate::error::to_py_err;

#[pyfunction]
pub fn extract_pptx_from_path_msgpack<'py>(
    py: Python<'py>,
    path: &str,
    extract_images: bool,
) -> PyResult<Bound<'py, PyBytes>> {
    // Release GIL during computation
    let result = py
        .detach(|| kreuzberg::extraction::extract_pptx_from_path(path, extract_images))
        .map_err(to_py_err)?;

    // Serialize to MessagePack using named encoding (map-based, compatible with msgspec)
    let msgpack_bytes = rmp_serde::encode::to_vec_named(&result)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

    Ok(PyBytes::new(py, &msgpack_bytes))
}

#[pyfunction]
pub fn extract_pptx_from_bytes_msgpack<'py>(
    py: Python<'py>,
    data: &[u8],
    extract_images: bool,
) -> PyResult<Bound<'py, PyBytes>> {
    // Release GIL during computation
    let result = py
        .detach(|| kreuzberg::extraction::extract_pptx_from_bytes(data, extract_images))
        .map_err(to_py_err)?;

    // Serialize to MessagePack using named encoding (map-based, compatible with msgspec)
    let msgpack_bytes = rmp_serde::encode::to_vec_named(&result)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

    Ok(PyBytes::new(py, &msgpack_bytes))
}
