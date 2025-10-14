use pyo3::prelude::*;

use crate::error::to_py_err;

#[pyfunction]
pub fn table_from_arrow_to_markdown(arrow_bytes: &[u8]) -> PyResult<String> {
    kreuzberg::extraction::table_from_arrow_to_markdown(arrow_bytes).map_err(to_py_err)
}
