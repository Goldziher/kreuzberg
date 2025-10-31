//! Result type bindings
//!
//! Provides Python-friendly wrappers around extraction result types.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::plugins::json_value_to_py;

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

    images: Option<Py<PyList>>,

    chunks: Option<Py<PyList>>,
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

    #[getter]
    fn images<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyList>> {
        self.images.as_ref().map(|img| img.bind(py).clone())
    }

    #[getter]
    fn chunks<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyList>> {
        self.chunks.as_ref().map(|chunks| chunks.bind(py).clone())
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
        let metadata_json = serde_json::to_value(&result.metadata).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Failed to serialize metadata: {}", e))
        })?;
        let metadata_py = json_value_to_py(py, &metadata_json)?;
        let metadata = metadata_py.cast_into::<PyDict>()?.clone().unbind();

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

        let images = if let Some(imgs) = result.images {
            let img_list = PyList::empty(py);
            for img in imgs {
                let img_dict = PyDict::new(py);
                img_dict.set_item("data", pyo3::types::PyBytes::new(py, &img.data))?;
                img_dict.set_item("format", &img.format)?;
                img_dict.set_item("image_index", img.image_index)?;

                if let Some(page) = img.page_number {
                    img_dict.set_item("page_number", page)?;
                }
                if let Some(width) = img.width {
                    img_dict.set_item("width", width)?;
                }
                if let Some(height) = img.height {
                    img_dict.set_item("height", height)?;
                }
                if let Some(colorspace) = &img.colorspace {
                    img_dict.set_item("colorspace", colorspace)?;
                }
                if let Some(bits) = img.bits_per_component {
                    img_dict.set_item("bits_per_component", bits)?;
                }
                img_dict.set_item("is_mask", img.is_mask)?;
                if let Some(desc) = &img.description {
                    img_dict.set_item("description", desc)?;
                }

                // Recursively convert nested OCR result
                if let Some(ocr) = img.ocr_result {
                    let ocr_py = Self::from_rust(*ocr, py)?;
                    img_dict.set_item("ocr_result", ocr_py)?;
                }

                img_list.append(img_dict)?;
            }
            Some(img_list.unbind())
        } else {
            None
        };

        let chunks = if let Some(chnks) = result.chunks {
            let chunk_list = PyList::empty(py);
            for chunk in chnks {
                let chunk_dict = PyDict::new(py);
                chunk_dict.set_item("content", &chunk.content)?;

                // Add embedding if present
                if let Some(embedding) = chunk.embedding {
                    let emb_list = PyList::new(py, embedding)?;
                    chunk_dict.set_item("embedding", emb_list)?;
                } else {
                    chunk_dict.set_item("embedding", py.None())?;
                }

                // Add metadata
                let chunk_metadata_json = serde_json::to_value(&chunk.metadata).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "Failed to serialize chunk metadata: {}",
                        e
                    ))
                })?;
                let chunk_metadata_py = json_value_to_py(py, &chunk_metadata_json)?;
                chunk_dict.set_item("metadata", chunk_metadata_py)?;

                chunk_list.append(chunk_dict)?;
            }
            Some(chunk_list.unbind())
        } else {
            None
        };

        Ok(Self {
            content: result.content,
            mime_type: result.mime_type,
            metadata,
            tables: tables.unbind(),
            detected_languages,
            images,
            chunks,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    fn prepare_python() {
        static INIT: Once = Once::new();
        INIT.call_once(Python::initialize);
    }

    fn with_py<F, R>(f: F) -> R
    where
        F: FnOnce(Python<'_>) -> R,
    {
        prepare_python();
        Python::attach(f)
    }

    #[test]
    fn test_from_rust_populates_basic_fields() {
        with_py(|py| {
            let rust_result = kreuzberg::ExtractionResult {
                content: "hello".to_string(),
                mime_type: "text/plain".to_string(),
                metadata: kreuzberg::Metadata::default(),
                tables: Vec::new(),
                detected_languages: Some(vec!["en".to_string()]),
                chunks: None,
                images: None,
            };

            let py_result = ExtractionResult::from_rust(rust_result, py).expect("conversion should succeed");
            assert_eq!(py_result.content, "hello");
            assert_eq!(py_result.mime_type, "text/plain");
            assert!(py_result.metadata(py).is_empty());
            assert_eq!(py_result.tables(py).len(), 0);
            assert!(py_result.detected_languages.is_some());
            assert_eq!(py_result.__str__(), "ExtractionResult: 5 characters");
            let repr = py_result.__repr__();
            assert!(repr.contains("mime_type='text/plain'"));
        });
    }

    #[test]
    fn test_metadata_setter_overrides_dictionary() {
        with_py(|py| {
            let mut rust_result = kreuzberg::ExtractionResult {
                content: "data".to_string(),
                mime_type: "text/plain".to_string(),
                metadata: kreuzberg::Metadata::default(),
                tables: Vec::new(),
                detected_languages: None,
                chunks: None,
                images: None,
            };
            rust_result
                .metadata
                .additional
                .insert("source".to_string(), serde_json::json!("original"));

            let mut py_result = ExtractionResult::from_rust(rust_result, py).expect("conversion should succeed");
            let new_metadata = PyDict::new(py);
            new_metadata.set_item("source", "override").unwrap();
            py_result
                .set_metadata(py, new_metadata.clone())
                .expect("setter should accept dict");
            let metadata = py_result.metadata(py);
            let source_item = metadata
                .get_item("source")
                .expect("lookup should succeed")
                .expect("source key");
            let source: String = source_item.extract().expect("string value");
            assert_eq!(source, "override");
        });
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
                    .and_then(|first_row| first_row.cast_into::<PyList>().ok().map(|list| list.len()))
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
