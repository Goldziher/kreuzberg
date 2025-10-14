use pyo3::prelude::*;

use crate::error::to_py_err;
use crate::types::PyExcelWorkbook;

#[pyfunction]
pub fn read_excel_bytes(py: Python<'_>, data: &[u8], file_extension: &str) -> PyResult<PyExcelWorkbook> {
    // Release GIL during heavy computation
    let result = py
        .detach(|| kreuzberg::extraction::read_excel_bytes(data, file_extension))
        .map_err(to_py_err)?;

    // Convert Rust type directly to Python type (zero serialization overhead)
    Ok(PyExcelWorkbook::from(result))
}

#[pyfunction]
pub fn read_excel_file(py: Python<'_>, file_path: &str) -> PyResult<PyExcelWorkbook> {
    // Release GIL during heavy computation
    let result = py
        .detach(|| kreuzberg::extraction::read_excel_file(file_path))
        .map_err(to_py_err)?;

    // Convert Rust type directly to Python type (zero serialization overhead)
    Ok(PyExcelWorkbook::from(result))
}
