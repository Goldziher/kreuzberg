use pyo3::prelude::*;
use std::collections::HashMap;

use kreuzberg::types::{ExcelSheet as CoreExcelSheet, ExcelWorkbook as CoreExcelWorkbook};

#[pyclass(name = "ExcelSheet", module = "kreuzberg._internal_bindings")]
#[derive(Clone)]
pub struct PyExcelSheet {
    inner: CoreExcelSheet,
}

#[pymethods]
impl PyExcelSheet {
    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[getter]
    fn markdown(&self) -> String {
        self.inner.markdown.clone()
    }

    #[getter]
    fn row_count(&self) -> usize {
        self.inner.row_count
    }

    #[getter]
    fn col_count(&self) -> usize {
        self.inner.col_count
    }

    #[getter]
    fn cell_count(&self) -> usize {
        self.inner.cell_count
    }

    fn __repr__(&self) -> String {
        format!(
            "ExcelSheet(name='{}', row_count={}, col_count={}, cell_count={})",
            self.inner.name, self.inner.row_count, self.inner.col_count, self.inner.cell_count
        )
    }
}

impl From<CoreExcelSheet> for PyExcelSheet {
    fn from(inner: CoreExcelSheet) -> Self {
        Self { inner }
    }
}

#[pyclass(name = "ExcelWorkbook", module = "kreuzberg._internal_bindings")]
#[derive(Clone)]
pub struct PyExcelWorkbook {
    inner: CoreExcelWorkbook,
}

#[pymethods]
impl PyExcelWorkbook {
    #[getter]
    fn sheets(&self) -> Vec<PyExcelSheet> {
        self.inner
            .sheets
            .iter()
            .map(|s| PyExcelSheet::from(s.clone()))
            .collect()
    }

    #[getter]
    fn metadata(&self) -> HashMap<String, String> {
        self.inner.metadata.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "ExcelWorkbook(sheets={}, metadata_keys={})",
            self.inner.sheets.len(),
            self.inner.metadata.len()
        )
    }
}

impl From<CoreExcelWorkbook> for PyExcelWorkbook {
    fn from(inner: CoreExcelWorkbook) -> Self {
        Self { inner }
    }
}
