//! Result type bindings
//!
//! Provides Python-friendly wrappers around extraction result types.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

// ============================================================================
// ============================================================================

/// Extraction result containing content, metadata, and tables.
///
/// This is the primary return type for all extraction operations.
///
/// Attributes:
///     content (str): Extracted text content
///     mime_type (str): MIME type of the extracted document
///     metadata (dict): Document metadata as key-value pairs
///     tables (list[ExtractedTable]): Extracted tables
///     detected_languages (list[dict] | None): Detected languages with confidence scores
///
/// Example:
///     >>> from kreuzberg import extract_file_sync, ExtractionConfig
///     >>> result = extract_file_sync("document.pdf", None, ExtractionConfig())
///     >>> print(result.content)
///     >>> print(result.metadata)
///     >>> print(len(result.tables))
///     >>> if result.detected_languages:
///     ...     print(result.detected_languages)
#[pyclass(name = "ExtractionResult", module = "kreuzberg")]
pub struct ExtractionResult {
    #[pyo3(get)]
    pub content: String,

    #[pyo3(get)]
    pub mime_type: String,

    metadata: Py<PyDict>,
    tables: Py<PyList>,

    #[pyo3(get)]
    pub detected_languages: Option<Py<PyList>>,
}

#[pymethods]
impl ExtractionResult {
    #[getter]
    fn metadata<'py>(&self, py: Python<'py>) -> Bound<'py, PyDict> {
        self.metadata.bind(py).clone()
    }

    #[setter]
    fn set_metadata(&mut self, _py: Python<'_>, value: Bound<'_, PyDict>) -> PyResult<()> {
        self.metadata = value.unbind();
        Ok(())
    }

    #[getter]
    fn tables<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        self.tables.bind(py).clone()
    }

    fn __repr__(&self) -> String {
        Python::attach(|py| {
            format!(
                "ExtractionResult(mime_type='{}', content_length={}, tables_count={})",
                self.mime_type,
                self.content.len(),
                self.tables.bind(py).len()
            )
        })
    }

    fn __str__(&self) -> String {
        format!("ExtractionResult: {} characters", self.content.len())
    }
}

impl ExtractionResult {
    /// Convert from Rust ExtractionResult to Python ExtractionResult.
    ///
    /// This performs efficient conversion of:
    /// - metadata HashMap -> PyDict (using pythonize for optimal performance)
    /// - tables Vec -> PyList
    /// - detected_languages Vec -> PyList
    /// - serde_json::Value -> Python objects
    pub fn from_rust(result: kreuzberg::ExtractionResult, py: Python) -> PyResult<Self> {
        let metadata_py = pythonize::pythonize(py, &result.metadata)?;
        let metadata = metadata_py.downcast::<PyDict>()?.clone().unbind();

        let tables = PyList::empty(py);
        for table in result.tables {
            tables.append(ExtractedTable::from_rust(table, py)?)?;
        }

        let detected_languages = if let Some(langs) = result.detected_languages {
            let lang_list = PyList::new(py, langs)?;
            Some(lang_list.unbind())
        } else {
            None
        };

        Ok(Self {
            content: result.content,
            mime_type: result.mime_type,
            metadata,
            tables: tables.unbind(),
            detected_languages,
        })
    }
}

/// Extracted table with cells and markdown representation.
///
/// Attributes:
///     cells (list[list[str]]): Table data as nested lists (rows of columns)
///     markdown (str): Markdown representation of the table
///     page_number (int): Page number where table was found
///
/// Example:
///     >>> result = extract_file_sync("document.pdf", None, ExtractionConfig())
///     >>> for table in result.tables:
///     ...     print(f"Table on page {table.page_number}:")
///     ...     print(table.markdown)
///     ...     print(f"Dimensions: {len(table.cells)} rows x {len(table.cells[0])} cols")
#[pyclass(name = "ExtractedTable", module = "kreuzberg")]
pub struct ExtractedTable {
    cells: Py<PyList>,

    #[pyo3(get)]
    pub markdown: String,

    #[pyo3(get)]
    pub page_number: usize,
}

#[pymethods]
impl ExtractedTable {
    #[getter]
    fn cells<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        self.cells.bind(py).clone()
    }

    fn __repr__(&self) -> String {
        Python::attach(|py| {
            let rows = self.cells.bind(py).len();
            let cols = if rows > 0 {
                self.cells
                    .bind(py)
                    .get_item(0)
                    .ok()
                    .and_then(|first_row| first_row.downcast::<PyList>().ok().map(|list| list.len()))
                    .unwrap_or(0)
            } else {
                0
            };
            format!(
                "ExtractedTable(rows={}, cols={}, page={})",
                rows, cols, self.page_number
            )
        })
    }

    fn __str__(&self) -> String {
        format!("Table on page {} ({} chars)", self.page_number, self.markdown.len())
    }
}

impl ExtractedTable {
    /// Convert from Rust Table to Python ExtractedTable.
    pub fn from_rust(table: kreuzberg::Table, py: Python) -> PyResult<Self> {
        let cells = PyList::empty(py);
        for row in table.cells {
            let py_row = PyList::new(py, row)?;
            cells.append(py_row)?;
        }

        Ok(Self {
            cells: cells.unbind(),
            markdown: table.markdown,
            page_number: table.page_number,
        })
    }
}
