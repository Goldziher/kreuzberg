//! Result type bindings
//!
//! Provides Python-friendly wrappers around extraction result types.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

// ============================================================================
// ExtractionResult
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
///
/// Example:
///     >>> from kreuzberg import extract_file_sync, ExtractionConfig
///     >>> result = extract_file_sync("document.pdf", None, ExtractionConfig())
///     >>> print(result.content)
///     >>> print(result.metadata)
///     >>> print(len(result.tables))
#[pyclass(name = "ExtractionResult", module = "kreuzberg")]
pub struct ExtractionResult {
    #[pyo3(get)]
    pub content: String,

    #[pyo3(get)]
    pub mime_type: String,

    metadata: Py<PyDict>,
    tables: Py<PyList>,
}

#[pymethods]
impl ExtractionResult {
    #[getter]
    fn metadata<'py>(&self, py: Python<'py>) -> Bound<'py, PyDict> {
        self.metadata.bind(py).clone()
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
    /// - metadata HashMap -> PyDict
    /// - tables Vec -> PyList
    /// - serde_json::Value -> Python objects
    pub fn from_rust(result: kreuzberg::ExtractionResult, py: Python) -> PyResult<Self> {
        // Convert metadata HashMap -> PyDict
        let metadata = PyDict::new(py);
        for (key, value) in result.metadata {
            // Convert serde_json::Value to Python object
            let py_value = serde_json_to_py(&value, py)?;
            metadata.set_item(key, py_value)?;
        }

        // Convert tables Vec -> PyList
        let tables = PyList::empty(py);
        for table in result.tables {
            tables.append(ExtractedTable::from_rust(table, py)?)?;
        }

        Ok(Self {
            content: result.content,
            mime_type: result.mime_type,
            metadata: metadata.unbind(),
            tables: tables.unbind(),
        })
    }
}

// ============================================================================
// ExtractedTable
// ============================================================================

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
        // Convert cells Vec<Vec<String>> -> PyList of PyLists
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

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert serde_json::Value to Python objects.
///
/// This handles all JSON types and converts them to appropriate Python types:
/// - null -> None
/// - bool -> bool
/// - number -> int or float
/// - string -> str
/// - array -> list
/// - object -> dict
fn serde_json_to_py(value: &serde_json::Value, py: Python) -> PyResult<Py<PyAny>> {
    use pyo3::IntoPyObject;
    use serde_json::Value;

    match value {
        Value::Null => Ok(py.None()),
        Value::Bool(b) => {
            let obj = b.into_pyobject(py)?;
            Ok(obj.as_any().clone().unbind())
        }
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                let obj = i.into_pyobject(py)?;
                Ok(obj.as_any().clone().unbind())
            } else if let Some(f) = n.as_f64() {
                let obj = f.into_pyobject(py)?;
                Ok(obj.as_any().clone().unbind())
            } else {
                // Fallback for u64 or other number types
                let obj = n.to_string().into_pyobject(py)?;
                Ok(obj.as_any().clone().unbind())
            }
        }
        Value::String(s) => {
            let obj = s.into_pyobject(py)?;
            Ok(obj.as_any().clone().unbind())
        }
        Value::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(serde_json_to_py(item, py)?)?;
            }
            Ok(list.into_any().unbind())
        }
        Value::Object(obj) => {
            let dict = PyDict::new(py);
            for (k, v) in obj {
                dict.set_item(k, serde_json_to_py(v, py)?)?;
            }
            Ok(dict.into_any().unbind())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serde_json_to_py() {
        Python::initialize();
        Python::attach(|py| {
            // Test null
            let null_val = serde_json::Value::Null;
            let py_null = serde_json_to_py(&null_val, py).unwrap();
            assert!(py_null.is_none(py));

            // Test bool
            let bool_val = serde_json::Value::Bool(true);
            let py_bool = serde_json_to_py(&bool_val, py).unwrap();
            assert!(py_bool.extract::<bool>(py).unwrap());

            // Test number (int)
            let int_val = serde_json::json!(42);
            let py_int = serde_json_to_py(&int_val, py).unwrap();
            assert_eq!(py_int.extract::<i64>(py).unwrap(), 42);

            // Test number (float)
            let float_val = serde_json::json!(3.14);
            let py_float = serde_json_to_py(&float_val, py).unwrap();
            assert!((py_float.extract::<f64>(py).unwrap() - 3.14).abs() < 0.001);

            // Test string
            let str_val = serde_json::json!("test");
            let py_str = serde_json_to_py(&str_val, py).unwrap();
            assert_eq!(py_str.extract::<String>(py).unwrap(), "test");

            // Test array
            let arr_val = serde_json::json!([1, 2, 3]);
            let py_arr = serde_json_to_py(&arr_val, py).unwrap();
            let py_list = py_arr.downcast_bound::<PyList>(py).unwrap();
            assert_eq!(py_list.len(), 3);

            // Test object
            let obj_val = serde_json::json!({"key": "value"});
            let py_obj = serde_json_to_py(&obj_val, py).unwrap();
            let py_dict = py_obj.downcast_bound::<PyDict>(py).unwrap();
            assert_eq!(
                py_dict.get_item("key").unwrap().unwrap().extract::<String>().unwrap(),
                "value"
            );
        });
    }
}
