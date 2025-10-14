use pyo3::prelude::*;
use pyo3::types::PyBytes;

use crate::error::to_py_err;

#[pyfunction]
pub fn parse_json_msgpack<'py>(
    py: Python<'py>,
    data: &[u8],
    config_msgpack: Option<&[u8]>,
) -> PyResult<Bound<'py, PyBytes>> {
    // Deserialize config if provided
    let config = if let Some(cfg_bytes) = config_msgpack {
        Some(
            rmp_serde::from_slice::<kreuzberg::extraction::structured::JsonExtractionConfig>(cfg_bytes)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?,
        )
    } else {
        None
    };

    // Release GIL during computation
    let result = py
        .detach(|| kreuzberg::extraction::parse_json(data, config))
        .map_err(to_py_err)?;

    // Serialize to MessagePack using named encoding (map-based, compatible with msgspec)
    let msgpack_bytes = rmp_serde::encode::to_vec_named(&result)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

    Ok(PyBytes::new(py, &msgpack_bytes))
}

#[pyfunction]
pub fn parse_yaml_msgpack<'py>(py: Python<'py>, data: &[u8]) -> PyResult<Bound<'py, PyBytes>> {
    // Release GIL during computation
    let result = py
        .detach(|| kreuzberg::extraction::parse_yaml(data))
        .map_err(to_py_err)?;

    // Serialize to MessagePack using named encoding (map-based, compatible with msgspec)
    let msgpack_bytes = rmp_serde::encode::to_vec_named(&result)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

    Ok(PyBytes::new(py, &msgpack_bytes))
}

#[pyfunction]
pub fn parse_toml_msgpack<'py>(py: Python<'py>, data: &[u8]) -> PyResult<Bound<'py, PyBytes>> {
    // Release GIL during computation
    let result = py
        .detach(|| kreuzberg::extraction::parse_toml(data))
        .map_err(to_py_err)?;

    // Serialize to MessagePack using named encoding (map-based, compatible with msgspec)
    let msgpack_bytes = rmp_serde::encode::to_vec_named(&result)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

    Ok(PyBytes::new(py, &msgpack_bytes))
}
