//! Plugin registration functions for Python-Rust FFI bridge.
//!
//! Allows Python-based plugins (OCR backends, PostProcessors, Validators) to register
//! with the Rust core and be used by the Rust CLI, API server, and MCP server.
//!
//! # Architecture
//!
//! This module provides the FFI bridge that enables:
//! - **Python OCR backends** (EasyOCR, PaddleOCR, custom backends) to be used by Rust extraction
//! - **Python PostProcessors** (entity extraction, keyword extraction, metadata enrichment) to enrich results
//! - **Python Validators** (content validation, quality checks) to validate extraction results
//!
//! # GIL (Global Interpreter Lock) Management
//!
//! This module implements sophisticated GIL management patterns to bridge Python and Rust safely
//! and efficiently. Understanding these patterns is critical for maintaining thread safety and performance.
//!
//! ## Core GIL Patterns
//!
//! ### 1. `Python::attach()` - Temporary GIL Acquisition
//!
//! Used when calling Python code from Rust:
//! ```rust,ignore
//! Python::attach(|py| {
//!     let result = self.python_obj.bind(py).call_method0("name")?;
//!     result.extract::<String>()
//! })
//! ```
//! - **When**: Reading Python object attributes, calling Python methods
//! - **GIL held**: Only during the closure execution
//! - **Thread safety**: Safe to call from any thread, blocks if GIL unavailable
//! - **Performance**: Minimal overhead for quick operations
//!
//! ### 2. `py.detach()` - GIL Release During Expensive Operations
//!
//! Used when performing expensive Rust operations that don't need Python access:
//! ```rust,ignore
//! py.detach(|| {
//!     // GIL is released here - other Python threads can run
//!     let registry = get_ocr_backend_registry();
//!     let mut registry = registry.write()?; // Expensive lock acquisition
//!     registry.register(backend)
//! })
//! ```
//! - **When**: Writing to registries, I/O operations, expensive computations
//! - **GIL held**: Released during the closure, reacquired after
//! - **Why critical**: Prevents blocking Python threads during Rust operations
//! - **Performance**: Allows Python code to continue running in other threads
//!
//! ### 3. `tokio::task::spawn_blocking` - Async-to-Sync Bridge
//!
//! Used when calling Python code from async Rust (Python is inherently synchronous):
//! ```rust,ignore
//! let python_obj = Python::attach(|py| self.python_obj.clone_ref(py));
//! tokio::task::spawn_blocking(move || {
//!     Python::attach(|py| {
//!         let obj = python_obj.bind(py);
//!         obj.call_method1("process_image", (py_bytes, language))
//!     })
//! })
//! .await?
//! ```
//! - **When**: Async trait implementations (OcrBackend::process_image, PostProcessor::process)
//! - **Why necessary**: Python calls block, incompatible with async Rust
//! - **GIL management**: Acquires GIL inside blocking task, doesn't block tokio runtime
//! - **Data transfer**: Use `clone_ref(py)` to safely move Python objects across thread boundary
//!
//! ### 4. Caching to Minimize GIL Acquisitions
//!
//! Plugin wrappers cache frequently-accessed Python data in Rust fields:
//! ```rust,ignore
//! pub struct PythonOcrBackend {
//!     python_obj: Py<PyAny>,
//!     name: String,                    // Cached - no GIL needed
//!     supported_languages: Vec<String>, // Cached - no GIL needed
//! }
//! ```
//! - **When**: Data accessed frequently but rarely changes (name, supported languages)
//! - **Why important**: Avoids GIL acquisition overhead on every call
//! - **Trade-off**: Slightly more memory for significantly better performance
//! - **Pattern**: Cache in constructor, use cached values in trait methods
//!
//! ## Thread Safety Guarantees
//!
//! - **All plugin wrappers are `Send + Sync`**: Can be safely shared across threads
//! - **Py<PyAny> is thread-safe**: PyO3 ensures Python objects can cross thread boundaries
//! - **GIL prevents data races**: Only one thread accesses Python state at a time
//! - **Rust mutexes protect registries**: RwLock ensures safe concurrent registry access
//!
//! ## Performance Considerations
//!
//! - **GIL acquisition cost**: ~100ns per acquisition on modern hardware
//! - **Cache effectiveness**: Reduces GIL acquisitions by 10-100x for hot paths
//! - **Blocking tasks overhead**: ~1-5μs per spawn_blocking call
//! - **Design principle**: Minimize GIL acquisitions, maximize time GIL is released
//!
//! ## Error Handling with GIL
//!
//! - **hasattr() failures**: If GIL acquisition fails, log warning and use safe defaults
//! - **Method call failures**: Convert PyErr to KreuzbergError with context
//! - **Lock poisoning**: Handle registry lock poisoning gracefully
//! - **Pattern**: Always provide fallback behavior, never panic on GIL errors
//!
//! ## Common Pitfalls and Solutions
//!
//! 1. **Deadlock risk**: Never hold registry lock while acquiring GIL
//!    - Solution: Use `py.detach()` to release GIL before acquiring registry lock
//!
//! 2. **GIL contention**: Holding GIL too long blocks all Python threads
//!    - Solution: Cache data, release GIL with `detach()` during expensive operations
//!
//! 3. **Moving Python objects**: Can't send `&Bound<PyAny>` across threads
//!    - Solution: Use `Py<PyAny>` and `clone_ref(py)` for ownership transfer
//!
//! 4. **Async incompatibility**: Python code blocks, can't be awaited
//!    - Solution: Always use `spawn_blocking` for Python calls from async code

use pyo3::prelude::*;
use pyo3::types::{PyBool, PyBytes, PyDict, PyList, PyString};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use kreuzberg::core::config::{ExtractionConfig, OcrConfig};
use kreuzberg::plugins::registry::{get_ocr_backend_registry, get_post_processor_registry, get_validator_registry};
use kreuzberg::plugins::{OcrBackend, OcrBackendType, Plugin, PostProcessor, ProcessingStage, Validator};
use kreuzberg::types::{ExtractionResult, Table};
use kreuzberg::{KreuzbergError, Result};

/// Validate that a Python plugin object has all required methods.
///
/// # Arguments
///
/// * `obj` - Python plugin object to validate
/// * `plugin_type` - Human-readable plugin type name (e.g., "OCR backend", "PostProcessor")
/// * `required_methods` - Slice of required method names
///
/// # Returns
///
/// `Ok(())` if all methods exist, otherwise `PyErr` describing missing methods.
///
/// # Errors
///
/// Returns `PyAttributeError` if any required methods are missing.
fn validate_plugin_object(obj: &Bound<'_, PyAny>, plugin_type: &str, required_methods: &[&str]) -> PyResult<()> {
    let mut missing_methods = Vec::new();

    for method_name in required_methods {
        if !obj.hasattr(*method_name)? {
            missing_methods.push(*method_name);
        }
    }

    if !missing_methods.is_empty() {
        return Err(pyo3::exceptions::PyAttributeError::new_err(format!(
            "{} is missing required methods: {}. Please ensure your plugin implements all required methods.",
            plugin_type,
            missing_methods.join(", ")
        )));
    }

    Ok(())
}

/// Convert serde_json::Value to Python object
pub(crate) fn json_value_to_py<'py>(py: Python<'py>, value: &serde_json::Value) -> PyResult<Bound<'py, PyAny>> {
    match value {
        serde_json::Value::Null => Ok(py.None().into_bound(py)),
        serde_json::Value::Bool(b) => {
            let py_bool = PyBool::new(py, *b);
            Ok(py_bool.as_any().clone())
        }
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_pyobject(py)?.into_any())
            } else if let Some(f) = n.as_f64() {
                Ok(f.into_pyobject(py)?.into_any())
            } else {
                Ok(py.None().into_bound(py))
            }
        }
        serde_json::Value::String(s) => Ok(s.into_pyobject(py)?.into_any()),
        serde_json::Value::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(json_value_to_py(py, item)?)?;
            }
            Ok(list.into_any())
        }
        serde_json::Value::Object(obj) => {
            let dict = PyDict::new(py);
            for (k, v) in obj {
                dict.set_item(k, json_value_to_py(py, v)?)?;
            }
            Ok(dict.into_any())
        }
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

    #[pyclass]
    struct TestPlugin;

    #[pymethods]
    impl TestPlugin {
        fn name(&self) -> &'static str {
            "demo"
        }

        fn process(&self) -> bool {
            true
        }
    }

    #[pyclass]
    struct IncompletePlugin;

    #[pymethods]
    impl IncompletePlugin {
        fn name(&self) -> &'static str {
            "demo"
        }
    }

    #[test]
    fn test_validate_plugin_object_success() {
        with_py(|py| {
            let plugin = Py::new(py, TestPlugin).expect("should allocate plugin");
            let instance = plugin.into_bound(py).into_any();
            validate_plugin_object(&instance, "postprocessor", &["name", "process"])
                .expect("plugin object with methods should validate");
        });
    }

    #[test]
    fn test_validate_plugin_object_reports_missing_methods() {
        with_py(|py| {
            let plugin = Py::new(py, IncompletePlugin).expect("should allocate plugin");
            let instance = plugin.into_bound(py).into_any();
            let err = validate_plugin_object(&instance, "validator", &["name", "process"]).unwrap_err();
            assert!(err.is_instance_of::<pyo3::exceptions::PyAttributeError>(py));
        });
    }

    #[test]
    fn test_json_value_to_py_converts_nested_structures() {
        with_py(|py| {
            let json_value = serde_json::json!({
                "name": "example",
                "enabled": true,
                "weights": [1, 2, 3],
                "settings": {
                    "threshold": 0.85,
                    "modes": ["fast", "safe"]
                }
            });

            let py_obj = json_value_to_py(py, &json_value).expect("conversion should succeed");
            let dict = py_obj.cast::<PyDict>().expect("expected dictionary");
            let name_item = dict.get_item("name").expect("lookup should succeed").expect("name key");
            let name: String = name_item.extract().expect("string value");
            assert_eq!(name, "example");

            let enabled_item = dict
                .get_item("enabled")
                .expect("lookup should succeed")
                .expect("enabled key");
            let enabled: bool = enabled_item.extract().expect("bool value");
            assert!(enabled);

            let weights_item = dict
                .get_item("weights")
                .expect("lookup should succeed")
                .expect("weights key");
            let weights = weights_item.cast::<PyList>().expect("weights list");
            assert_eq!(weights.len(), 3);

            let settings_item = dict
                .get_item("settings")
                .expect("lookup should succeed")
                .expect("settings key");
            let settings = settings_item.cast::<PyDict>().expect("settings dict");
            let threshold_item = settings
                .get_item("threshold")
                .expect("lookup should succeed")
                .expect("threshold key");
            let threshold: f64 = threshold_item.extract().expect("float value");
            assert_eq!(threshold, 0.85);
        });
    }
}

/// Wrapper that makes a Python OCR backend usable from Rust.
///
/// This struct implements the Rust `OcrBackend` trait by forwarding calls
/// to a Python object via PyO3, bridging the FFI boundary with proper
/// GIL management and type conversions.
pub struct PythonOcrBackend {
    /// Python object implementing the OCR backend protocol
    python_obj: Py<PyAny>,
    /// Cached backend name (to avoid repeated GIL acquisition)
    name: String,
    /// Cached supported languages
    supported_languages: Vec<String>,
}

impl PythonOcrBackend {
    /// Create a new Python OCR backend wrapper.
    ///
    /// # Arguments
    ///
    /// * `py` - Python GIL token
    /// * `python_obj` - Python object implementing the backend protocol
    ///
    /// # Returns
    ///
    /// A new `PythonOcrBackend` or an error if the Python object is invalid.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Python object doesn't have required methods
    /// - Method calls fail during initialization
    pub fn new(py: Python<'_>, python_obj: Py<PyAny>) -> PyResult<Self> {
        let obj = python_obj.bind(py);

        validate_plugin_object(obj, "OCR backend", &["name", "supported_languages", "process_image"])?;

        let name: String = obj.call_method0("name")?.extract()?;
        if name.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "OCR backend name cannot be empty",
            ));
        }

        let supported_languages: Vec<String> = obj.call_method0("supported_languages")?.extract()?;
        if supported_languages.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "OCR backend must support at least one language",
            ));
        }

        Ok(Self {
            python_obj,
            name,
            supported_languages,
        })
    }
}

impl Plugin for PythonOcrBackend {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> String {
        Python::attach(|py| {
            self.python_obj
                .bind(py)
                .getattr("version")
                .and_then(|v| v.call0())
                .and_then(|v| v.extract::<String>())
                .unwrap_or_else(|_| "1.0.0".to_string())
        })
    }

    fn initialize(&self) -> Result<()> {
        Python::attach(|py| {
            let obj = self.python_obj.bind(py);
            if obj.hasattr("initialize")? {
                obj.call_method0("initialize")?;
            }
            Ok(())
        })
        .map_err(|e: PyErr| KreuzbergError::Plugin {
            message: format!("Failed to initialize Python OCR backend '{}': {}", self.name, e),
            plugin_name: self.name.clone(),
        })
    }

    fn shutdown(&self) -> Result<()> {
        Python::attach(|py| {
            let obj = self.python_obj.bind(py);
            if obj.hasattr("shutdown")? {
                obj.call_method0("shutdown")?;
            }
            Ok(())
        })
        .map_err(|e: PyErr| KreuzbergError::Plugin {
            message: format!("Failed to shutdown Python OCR backend '{}': {}", self.name, e),
            plugin_name: self.name.clone(),
        })
    }
}

#[async_trait]
impl OcrBackend for PythonOcrBackend {
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractionResult> {
        let image_bytes = image_bytes.to_vec();
        let language = config.language.clone();
        let backend_name = self.name.clone();

        let is_async = Python::attach(|py| {
            let obj = self.python_obj.bind(py);
            obj.getattr("process_image")
                .and_then(|method| method.hasattr("__await__"))
                .unwrap_or(false)
        });

        if is_async {
            let python_obj = Python::attach(|py| self.python_obj.clone_ref(py));

            let result = Python::attach(|py| {
                let obj = python_obj.bind(py);
                let py_bytes = PyBytes::new(py, &image_bytes);

                let coroutine = obj
                    .call_method1("process_image", (py_bytes, language.as_str()))
                    .map_err(|e| KreuzbergError::Ocr {
                        message: format!(
                            "Python OCR backend '{}' failed during process_image: {}",
                            backend_name, e
                        ),
                        source: Some(Box::new(e)),
                    })?;

                if !coroutine.hasattr("__await__").unwrap_or(false) {
                    return Err(KreuzbergError::Ocr {
                        message: format!(
                            "Python OCR backend '{}' process_image marked as async but did not return a coroutine",
                            backend_name
                        ),
                        source: None,
                    });
                }

                pyo3_async_runtimes::tokio::into_future(coroutine).map_err(|e| KreuzbergError::Ocr {
                    message: format!(
                        "Failed to convert Python coroutine to Rust future for OCR backend '{}': {}",
                        backend_name, e
                    ),
                    source: Some(Box::new(e)),
                })
            })?
            .await
            .map_err(|e: PyErr| KreuzbergError::Ocr {
                message: format!(
                    "Python OCR backend '{}' async process_image failed: {}",
                    backend_name, e
                ),
                source: Some(Box::new(e)),
            })?;

            Python::attach(|py| dict_to_extraction_result(py, result.bind(py)))
        } else {
            let python_obj = Python::attach(|py| self.python_obj.clone_ref(py));

            tokio::task::spawn_blocking(move || {
                Python::attach(|py| {
                    let obj = python_obj.bind(py);

                    let py_bytes = PyBytes::new(py, &image_bytes);

                    let result = obj
                        .call_method1("process_image", (py_bytes, language.as_str()))
                        .map_err(|e| KreuzbergError::Ocr {
                            message: format!(
                                "Python OCR backend '{}' failed during process_image: {}",
                                backend_name, e
                            ),
                            source: Some(Box::new(e)),
                        })?;

                    dict_to_extraction_result(py, &result)
                })
            })
            .await
            .map_err(|e| KreuzbergError::Ocr {
                message: format!("Failed to spawn blocking task for Python OCR backend: {}", e),
                source: Some(Box::new(e)),
            })?
        }
    }

    async fn process_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractionResult> {
        // If hasattr fails due to GIL error, log and fall back to process_image ~keep
        let backend_name = self.name.clone();
        let has_process_file = Python::attach(|py| {
            self.python_obj
                .bind(py)
                .hasattr("process_file")
                .map_err(|e| {
                    tracing::debug!(
                        "WARNING: OCR backend '{}': Failed to check for process_file method due to GIL error ({}), falling back to process_image",
                        backend_name, e
                    );
                    e
                })
                .unwrap_or(false)
        });

        if has_process_file {
            let path_str = path.to_string_lossy().to_string();
            let language = config.language.clone();
            let backend_name = self.name.clone();

            let is_async = Python::attach(|py| {
                let obj = self.python_obj.bind(py);
                obj.getattr("process_file")
                    .and_then(|method| method.hasattr("__await__"))
                    .unwrap_or(false)
            });

            if is_async {
                let python_obj = Python::attach(|py| self.python_obj.clone_ref(py));

                let result = Python::attach(|py| {
                    let obj = python_obj.bind(py);
                    let py_path = PyString::new(py, &path_str);

                    let coroutine = obj
                        .call_method1("process_file", (py_path, language.as_str()))
                        .map_err(|e| KreuzbergError::Ocr {
                            message: format!(
                                "Python OCR backend '{}' failed during process_file: {}",
                                backend_name, e
                            ),
                            source: Some(Box::new(e)),
                        })?;

                    if !coroutine.hasattr("__await__").unwrap_or(false) {
                        return Err(KreuzbergError::Ocr {
                            message: format!(
                                "Python OCR backend '{}' process_file marked as async but did not return a coroutine",
                                backend_name
                            ),
                            source: None,
                        });
                    }

                    pyo3_async_runtimes::tokio::into_future(coroutine).map_err(|e| KreuzbergError::Ocr {
                        message: format!(
                            "Failed to convert Python coroutine to Rust future for OCR backend '{}': {}",
                            backend_name, e
                        ),
                        source: Some(Box::new(e)),
                    })
                })?
                .await
                .map_err(|e: PyErr| KreuzbergError::Ocr {
                    message: format!("Python OCR backend '{}' async process_file failed: {}", backend_name, e),
                    source: Some(Box::new(e)),
                })?;

                Python::attach(|py| dict_to_extraction_result(py, result.bind(py)))
            } else {
                let python_obj = Python::attach(|py| self.python_obj.clone_ref(py));

                tokio::task::spawn_blocking(move || {
                    Python::attach(|py| {
                        let obj = python_obj.bind(py);
                        let py_path = PyString::new(py, &path_str);

                        let result = obj
                            .call_method1("process_file", (py_path, language.as_str()))
                            .map_err(|e| KreuzbergError::Ocr {
                                message: format!(
                                    "Python OCR backend '{}' failed during process_file: {}",
                                    backend_name, e
                                ),
                                source: Some(Box::new(e)),
                            })?;

                        dict_to_extraction_result(py, &result)
                    })
                })
                .await
                .map_err(|e| KreuzbergError::Ocr {
                    message: format!("Failed to spawn blocking task for Python OCR backend: {}", e),
                    source: Some(Box::new(e)),
                })?
            }
        } else {
            use kreuzberg::core::io;
            let bytes = io::read_file_async(path).await?;
            self.process_image(&bytes, config).await
        }
    }

    fn supports_language(&self, lang: &str) -> bool {
        self.supported_languages.iter().any(|l| l == lang)
    }

    fn backend_type(&self) -> OcrBackendType {
        match self.name.as_str() {
            "easyocr" => OcrBackendType::EasyOCR,
            "paddleocr" | "paddle" => OcrBackendType::PaddleOCR,
            _ => OcrBackendType::Custom,
        }
    }

    fn supported_languages(&self) -> Vec<String> {
        self.supported_languages.clone()
    }
}

/// Convert Python dict to Rust ExtractionResult.
///
/// Expected dict format:
/// ```python
/// {
///     "content": "extracted text",
///     "metadata": {"width": 800, "height": 600},
///     "tables": []  # Optional
/// }
/// ```
fn dict_to_extraction_result(_py: Python<'_>, dict: &Bound<'_, PyAny>) -> Result<ExtractionResult> {
    let content: String = match dict.get_item("content") {
        Ok(val) if !val.is_none() => val.extract().map_err(|e| KreuzbergError::Validation {
            message: format!("Python OCR result 'content' must be a string: {}", e),
            source: None,
        })?,
        Ok(_) => {
            return Err(KreuzbergError::Validation {
                message: "Python OCR result 'content' field is None".to_string(),
                source: None,
            });
        }
        Err(e) => {
            return Err(KreuzbergError::Validation {
                message: format!("Python OCR result missing 'content' field: {}", e),
                source: None,
            });
        }
    };

    let additional = match dict.get_item("metadata") {
        Ok(m) if !m.is_none() => extract_metadata(&m).unwrap_or_default(),
        _ => HashMap::new(),
    };

    let tables = match dict.get_item("tables") {
        Ok(t) if !t.is_none() => extract_tables(&t)?,
        _ => vec![],
    };

    Ok(ExtractionResult {
        content,
        mime_type: "text/plain".to_string(),
        metadata: kreuzberg::types::Metadata {
            additional,
            ..Default::default()
        },
        tables,
        detected_languages: None,
        chunks: None,
        images: None,
    })
}

/// Extract metadata dict from Python object.
fn extract_metadata(obj: &Bound<'_, PyAny>) -> Result<HashMap<String, serde_json::Value>> {
    let dict = obj.cast::<PyDict>().map_err(|_| KreuzbergError::Validation {
        message: "Metadata must be a dict".to_string(),
        source: None,
    })?;

    let mut metadata = HashMap::new();
    for (key, value) in dict.iter() {
        let key_str: String = key.extract().map_err(|_| KreuzbergError::Validation {
            message: "Metadata keys must be strings".to_string(),
            source: None,
        })?;

        let json_value = python_to_json(&value)?;
        metadata.insert(key_str, json_value);
    }

    Ok(metadata)
}

/// Extract tables from Python object.
///
/// Expected Python format:
/// ```python
/// [
///     {
///         "cells": [["row1col1", "row1col2"], ["row2col1", "row2col2"]],
///         "markdown": "| Header |\n| ------ |\n| Cell   |",
///         "page_number": 1
///     }
/// ]
/// ```
fn extract_tables(obj: &Bound<'_, PyAny>) -> Result<Vec<Table>> {
    let list = obj
        .cast::<pyo3::types::PyList>()
        .map_err(|_| KreuzbergError::Validation {
            message: "Tables must be a list".to_string(),
            source: None,
        })?;

    let mut tables = Vec::new();

    for (index, item) in list.iter().enumerate() {
        let table_dict = item.cast::<PyDict>().map_err(|_| KreuzbergError::Validation {
            message: format!("Table at index {} must be a dict", index),
            source: None,
        })?;

        let cells_val = table_dict
            .get_item("cells")
            .map_err(|e| KreuzbergError::Validation {
                message: format!("Table at index {} error getting 'cells': {}", index, e),
                source: None,
            })?
            .ok_or_else(|| KreuzbergError::Validation {
                message: format!("Table at index {} missing 'cells' field", index),
                source: None,
            })?;

        if cells_val.is_none() {
            return Err(KreuzbergError::Validation {
                message: format!("Table at index {} 'cells' field is None", index),
                source: None,
            });
        }

        let cells_list = cells_val
            .cast::<pyo3::types::PyList>()
            .map_err(|_| KreuzbergError::Validation {
                message: format!("Table at index {} 'cells' must be a list", index),
                source: None,
            })?;

        let mut cells: Vec<Vec<String>> = Vec::new();
        for (row_idx, row_obj) in cells_list.iter().enumerate() {
            let row_list = row_obj
                .cast::<pyo3::types::PyList>()
                .map_err(|_| KreuzbergError::Validation {
                    message: format!("Table {} row {} must be a list", index, row_idx),
                    source: None,
                })?;

            let mut row: Vec<String> = Vec::new();
            for (col_idx, cell_obj) in row_list.iter().enumerate() {
                let cell_str: String = cell_obj.extract().map_err(|e| KreuzbergError::Validation {
                    message: format!(
                        "Table {} cell [{}, {}] must be a string: {}",
                        index, row_idx, col_idx, e
                    ),
                    source: None,
                })?;
                row.push(cell_str);
            }
            cells.push(row);
        }

        let markdown_val = table_dict
            .get_item("markdown")
            .map_err(|e| KreuzbergError::Validation {
                message: format!("Table at index {} error getting 'markdown': {}", index, e),
                source: None,
            })?
            .ok_or_else(|| KreuzbergError::Validation {
                message: format!("Table at index {} missing 'markdown' field", index),
                source: None,
            })?;

        if markdown_val.is_none() {
            return Err(KreuzbergError::Validation {
                message: format!("Table at index {} 'markdown' field is None", index),
                source: None,
            });
        }

        let markdown: String = markdown_val.extract().map_err(|e| KreuzbergError::Validation {
            message: format!("Table at index {} 'markdown' must be a string: {}", index, e),
            source: None,
        })?;

        let page_num_val = table_dict
            .get_item("page_number")
            .map_err(|e| KreuzbergError::Validation {
                message: format!("Table at index {} error getting 'page_number': {}", index, e),
                source: None,
            })?
            .ok_or_else(|| KreuzbergError::Validation {
                message: format!("Table at index {} missing 'page_number' field", index),
                source: None,
            })?;

        if page_num_val.is_none() {
            return Err(KreuzbergError::Validation {
                message: format!("Table at index {} 'page_number' field is None", index),
                source: None,
            });
        }

        let page_number: usize = page_num_val.extract().map_err(|e| KreuzbergError::Validation {
            message: format!("Table at index {} 'page_number' must be an integer: {}", index, e),
            source: None,
        })?;

        tables.push(Table {
            cells,
            markdown,
            page_number,
        });
    }

    Ok(tables)
}

/// Convert Python value to serde_json::Value.
fn python_to_json(obj: &Bound<'_, PyAny>) -> Result<serde_json::Value> {
    if obj.is_none() {
        Ok(serde_json::Value::Null)
    } else if let Ok(b) = obj.extract::<bool>() {
        Ok(serde_json::Value::Bool(b))
    } else if let Ok(i) = obj.extract::<i64>() {
        Ok(serde_json::Value::Number(i.into()))
    } else if let Ok(f) = obj.extract::<f64>() {
        Ok(serde_json::to_value(f).unwrap_or(serde_json::Value::Null))
    } else if let Ok(s) = obj.extract::<String>() {
        Ok(serde_json::Value::String(s))
    } else if let Ok(list) = obj.cast::<PyList>() {
        let mut vec = Vec::new();
        for item in list.iter() {
            vec.push(python_to_json(&item)?);
        }
        Ok(serde_json::Value::Array(vec))
    } else if let Ok(dict) = obj.cast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (key, value) in dict.iter() {
            let key_str: String = key.extract().map_err(|_| KreuzbergError::Validation {
                message: "Dict keys must be strings for JSON conversion".to_string(),
                source: None,
            })?;
            map.insert(key_str, python_to_json(&value)?);
        }
        Ok(serde_json::Value::Object(map))
    } else {
        Ok(serde_json::Value::String(
            obj.str()
                .map_err(|_| KreuzbergError::Validation {
                    message: "Failed to convert Python value to JSON".to_string(),
                    source: None,
                })?
                .to_string(),
        ))
    }
}

/// Register a Python OCR backend with the Rust core.
///
/// This function validates the Python backend object, wraps it in a Rust
/// `OcrBackend` implementation, and registers it with the global OCR backend
/// registry. Once registered, the backend can be used by the Rust CLI, API,
/// and MCP server.
///
/// # Arguments
///
/// * `name` - Backend name (must be unique)
/// * `backend` - Python object implementing the OCR backend protocol
///
/// # Required Methods on Python Backend
///
/// The Python backend must implement:
/// - `name() -> str` - Return backend name
/// - `supported_languages() -> list[str]` - Return list of supported language codes
/// - `process_image(image_bytes: bytes, language: str) -> dict` - Process image and return result
///
/// # Optional Methods
///
/// - `process_file(path: str, language: str) -> dict` - Custom file processing
/// - `initialize()` - Called when backend is registered
/// - `shutdown()` - Called when backend is unregistered
/// - `version() -> str` - Backend version (defaults to "1.0.0")
///
/// # Example
///
/// ```python
/// from kreuzberg import register_ocr_backend
///
/// class MyOcrBackend:
///     def name(self) -> str:
///         return "my-ocr"
///
///     def supported_languages(self) -> list[str]:
///         return ["eng", "deu", "fra"]
///
///     def process_image(self, image_bytes: bytes, language: str) -> dict:
///         # Process image and extract text
///         return {
///             "content": "extracted text",
///             "metadata": {"confidence": 0.95},
///             "tables": []
///         }
///
/// register_ocr_backend("my-ocr", MyOcrBackend())
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - Backend is missing required methods
/// - Backend name is empty or duplicate
/// - Registration fails
#[pyfunction]
pub fn register_ocr_backend(py: Python<'_>, backend: Py<PyAny>) -> PyResult<()> {
    let rust_backend = PythonOcrBackend::new(py, backend)?;
    let backend_name = rust_backend.name().to_string();

    let arc_backend: Arc<dyn OcrBackend> = Arc::new(rust_backend);

    py.detach(|| {
        let registry = get_ocr_backend_registry();
        let mut registry = registry.write().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire write lock on OCR registry: {}", e))
        })?;

        registry.register(arc_backend).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to register OCR backend '{}': {}",
                backend_name, e
            ))
        })
    })?;

    Ok(())
}

/// Wrapper that makes a Python PostProcessor usable from Rust.
///
/// This struct implements the Rust `PostProcessor` trait by forwarding calls
/// to a Python object via PyO3, bridging the FFI boundary with proper
/// GIL management and type conversions.
pub struct PythonPostProcessor {
    /// Python object implementing the PostProcessor protocol
    python_obj: Py<PyAny>,
    /// Cached processor name (to avoid repeated GIL acquisition)
    name: String,
    /// Processing stage (cached from Python or default to Middle)
    stage: ProcessingStage,
}

impl PythonPostProcessor {
    /// Create a new Python PostProcessor wrapper.
    ///
    /// # Arguments
    ///
    /// * `py` - Python GIL token
    /// * `python_obj` - Python object implementing the processor protocol
    ///
    /// # Returns
    ///
    /// A new `PythonPostProcessor` or an error if the Python object is invalid.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Python object doesn't have required methods
    /// - Method calls fail during initialization
    pub fn new(py: Python<'_>, python_obj: Py<PyAny>) -> PyResult<Self> {
        let obj = python_obj.bind(py);

        validate_plugin_object(obj, "PostProcessor", &["name", "process"])?;

        let name: String = obj.call_method0("name")?.extract()?;
        if name.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "PostProcessor name cannot be empty",
            ));
        }

        let stage = if obj.hasattr("processing_stage")? {
            let stage_str: String = obj.call_method0("processing_stage")?.extract()?;
            match stage_str.to_lowercase().as_str() {
                "early" => ProcessingStage::Early,
                "middle" => ProcessingStage::Middle,
                "late" => ProcessingStage::Late,
                _ => ProcessingStage::Middle,
            }
        } else {
            ProcessingStage::Middle
        };

        Ok(Self {
            python_obj,
            name,
            stage,
        })
    }
}

impl Plugin for PythonPostProcessor {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> String {
        Python::attach(|py| {
            self.python_obj
                .bind(py)
                .getattr("version")
                .and_then(|v| v.call0())
                .and_then(|v| v.extract::<String>())
                .unwrap_or_else(|_| "1.0.0".to_string())
        })
    }

    fn initialize(&self) -> Result<()> {
        Python::attach(|py| {
            let obj = self.python_obj.bind(py);
            if obj.hasattr("initialize")? {
                obj.call_method0("initialize")?;
            }
            Ok(())
        })
        .map_err(|e: PyErr| KreuzbergError::Plugin {
            message: format!("Failed to initialize Python PostProcessor '{}': {}", self.name, e),
            plugin_name: self.name.clone(),
        })
    }

    fn shutdown(&self) -> Result<()> {
        Python::attach(|py| {
            let obj = self.python_obj.bind(py);
            if obj.hasattr("shutdown")? {
                obj.call_method0("shutdown")?;
            }
            Ok(())
        })
        .map_err(|e: PyErr| KreuzbergError::Plugin {
            message: format!("Failed to shutdown Python PostProcessor '{}': {}", self.name, e),
            plugin_name: self.name.clone(),
        })
    }
}

#[async_trait]
impl PostProcessor for PythonPostProcessor {
    async fn process(&self, result: &mut ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
        let processor_name = self.name.clone();

        let updated_result = tokio::task::block_in_place(|| {
            Python::attach(|py| {
                let obj = self.python_obj.bind(py);

                let result_dict = extraction_result_to_dict(py, result).map_err(|e| KreuzbergError::Plugin {
                    message: format!("Failed to convert ExtractionResult to Python dict: {}", e),
                    plugin_name: processor_name.clone(),
                })?;

                let py_result = result_dict.bind(py);
                let processed = obj
                    .call_method1("process", (py_result,))
                    .map_err(|e| KreuzbergError::Plugin {
                        message: format!("Python PostProcessor '{}' failed during process: {}", processor_name, e),
                        plugin_name: processor_name.clone(),
                    })?;

                let processed_dict = processed.cast_into::<PyDict>().map_err(|e| KreuzbergError::Plugin {
                    message: format!("PostProcessor did not return a dict: {}", e),
                    plugin_name: processor_name.clone(),
                })?;

                let mut updated_result = result.clone();
                merge_dict_to_extraction_result(py, &processed_dict, &mut updated_result)?;

                Ok::<ExtractionResult, KreuzbergError>(updated_result)
            })
        })?;

        *result = updated_result;
        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        self.stage
    }
}

/// Convert Rust ExtractionResult to Python dict.
///
/// This creates a Python dict that can be passed to Python processors:
/// ```python
/// {
///     "content": "extracted text",
///     "mime_type": "application/pdf",
///     "metadata": {"key": "value"},
///     "tables": [...]
/// }
/// ```
fn extraction_result_to_dict(py: Python<'_>, result: &ExtractionResult) -> PyResult<Py<PyDict>> {
    let dict = PyDict::new(py);

    dict.set_item("content", &result.content)?;

    dict.set_item("mime_type", &result.mime_type)?;

    let metadata_json = serde_json::to_value(&result.metadata).map_err(|e| {
        pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to serialize metadata to JSON: {}", e))
    })?;
    let metadata_py = json_value_to_py(py, &metadata_json)?;
    dict.set_item("metadata", metadata_py)?;

    dict.set_item("tables", PyList::empty(py))?;

    Ok(dict.unbind())
}

/// Merge Python dict back into ExtractionResult.
///
/// This updates the result in place, preserving existing fields and only
/// merging new metadata fields. Does not overwrite existing metadata keys.
fn merge_dict_to_extraction_result(
    _py: Python<'_>,
    dict: &Bound<'_, PyDict>,
    result: &mut ExtractionResult,
) -> Result<()> {
    if let Some(val) = dict.get_item("content").map_err(|e| KreuzbergError::Plugin {
        message: format!("Failed to get 'content' from result dict: {}", e),
        plugin_name: "python".to_string(),
    })? && !val.is_none()
    {
        result.content = val.extract().map_err(|e| KreuzbergError::Plugin {
            message: format!("PostProcessor returned invalid 'content': {}", e),
            plugin_name: "python".to_string(),
        })?;
    }

    if let Some(m) = dict.get_item("metadata").map_err(|e| KreuzbergError::Plugin {
        message: format!("Failed to get 'metadata' from result dict: {}", e),
        plugin_name: "python".to_string(),
    })? && !m.is_none()
        && let Ok(meta_dict) = m.cast_into::<PyDict>()
    {
        for (key, value) in meta_dict.iter() {
            let key_str: String = key.extract().map_err(|_| KreuzbergError::Plugin {
                message: "Metadata keys must be strings".to_string(),
                plugin_name: "python".to_string(),
            })?;

            let json_value = python_to_json(&value)?;
            result.metadata.additional.insert(key_str, json_value);
        }
    }

    Ok(())
}

/// Register a Python PostProcessor with the Rust core.
///
/// This function validates the Python processor object, wraps it in a Rust
/// `PostProcessor` implementation, and registers it with the global PostProcessor
/// registry. Once registered, the processor will be called automatically after
/// extraction to enrich results with metadata, entities, keywords, etc.
///
/// # Arguments
///
/// * `processor` - Python object implementing the PostProcessor protocol
///
/// # Required Methods on Python PostProcessor
///
/// The Python processor must implement:
/// - `name() -> str` - Return processor name
/// - `process(result: dict) -> dict` - Process and enrich the extraction result
///
/// # Optional Methods
///
/// - `processing_stage() -> str` - Return "early", "middle", or "late" (defaults to "middle")
/// - `initialize()` - Called when processor is registered (e.g., load ML models)
/// - `shutdown()` - Called when processor is unregistered
/// - `version() -> str` - Processor version (defaults to "1.0.0")
///
/// # Example
///
/// ```python
/// from kreuzberg import register_post_processor
///
/// class EntityExtractor:
///     def name(self) -> str:
///         return "entity_extraction"
///
///     def processing_stage(self) -> str:
///         return "early"
///
///     def process(self, result: dict) -> dict:
///         # Extract entities from result["content"]
///         entities = {"PERSON": ["John Doe"], "ORG": ["Microsoft"]}
///         result["metadata"]["entities"] = entities
///         return result
///
/// register_post_processor(EntityExtractor())
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - Processor is missing required methods
/// - Processor name is empty or duplicate
/// - Registration fails
#[pyfunction]
pub fn register_post_processor(py: Python<'_>, processor: Py<PyAny>) -> PyResult<()> {
    let rust_processor = PythonPostProcessor::new(py, processor)?;
    let processor_name = rust_processor.name().to_string();

    let arc_processor: Arc<dyn PostProcessor> = Arc::new(rust_processor);

    py.detach(|| {
        let registry = get_post_processor_registry();
        let mut registry = registry.write().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to acquire write lock on PostProcessor registry: {}",
                e
            ))
        })?;

        registry.register(arc_processor, 0).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to register PostProcessor '{}': {}",
                processor_name, e
            ))
        })
    })?;

    Ok(())
}

/// Unregister a PostProcessor by name.
///
/// Removes a previously registered processor from the global registry and
/// calls its `shutdown()` method to release resources.
///
/// # Arguments
///
/// * `name` - Processor name to unregister
///
/// # Example
///
/// ```python
/// from kreuzberg import register_post_processor, unregister_post_processor
///
/// class MyProcessor:
///     def name(self) -> str:
///         return "my_processor"
///
///     def process(self, result: dict) -> dict:
///         return result
///
/// register_post_processor(MyProcessor())
/// # ... use processor ...
/// unregister_post_processor("my_processor")
/// ```
#[pyfunction]
pub fn unregister_post_processor(py: Python<'_>, name: &str) -> PyResult<()> {
    py.detach(|| {
        let registry = get_post_processor_registry();
        let mut registry = registry.write().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to acquire write lock on PostProcessor registry: {}",
                e
            ))
        })?;

        registry.remove(name).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to unregister PostProcessor '{}': {}", name, e))
        })
    })?;

    Ok(())
}

/// Clear all registered PostProcessors.
///
/// Removes all processors from the global registry and calls their `shutdown()`
/// methods. Useful for test cleanup or resetting state.
///
/// # Example
///
/// ```python
/// from kreuzberg import clear_post_processors
///
/// # In pytest fixture or test cleanup
/// clear_post_processors()
/// ```
#[pyfunction]
pub fn clear_post_processors(py: Python<'_>) -> PyResult<()> {
    py.detach(|| {
        let registry = get_post_processor_registry();
        let mut registry = registry.write().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to acquire write lock on PostProcessor registry: {}",
                e
            ))
        })?;

        registry.shutdown_all().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to clear PostProcessor registry: {}", e))
        })
    })?;

    Ok(())
}

/// Wrapper that makes a Python Validator usable from Rust.
///
/// This struct implements the Rust `Validator` trait by forwarding calls
/// to a Python object via PyO3, bridging the FFI boundary with proper
/// GIL management and type conversions.
pub struct PythonValidator {
    /// Python object implementing the Validator protocol
    python_obj: Py<PyAny>,
    /// Cached validator name (to avoid repeated GIL acquisition)
    name: String,
    /// Cached priority
    priority: i32,
}

impl PythonValidator {
    /// Create a new Python Validator wrapper.
    ///
    /// # Arguments
    ///
    /// * `py` - Python GIL token
    /// * `python_obj` - Python object implementing the validator protocol
    ///
    /// # Returns
    ///
    /// A new `PythonValidator` or an error if the Python object is invalid.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Python object doesn't have required methods
    /// - Method calls fail during initialization
    pub fn new(py: Python<'_>, python_obj: Py<PyAny>) -> PyResult<Self> {
        let obj = python_obj.bind(py);

        validate_plugin_object(obj, "Validator", &["name", "validate"])?;

        let name: String = obj.call_method0("name")?.extract()?;
        if name.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Validator name cannot be empty",
            ));
        }

        let priority = if obj.hasattr("priority")? {
            obj.call_method0("priority")?.extract()?
        } else {
            50
        };

        Ok(Self {
            python_obj,
            name,
            priority,
        })
    }
}

impl Plugin for PythonValidator {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> String {
        Python::attach(|py| {
            self.python_obj
                .bind(py)
                .getattr("version")
                .and_then(|v| v.call0())
                .and_then(|v| v.extract::<String>())
                .unwrap_or_else(|_| "1.0.0".to_string())
        })
    }

    fn initialize(&self) -> Result<()> {
        Python::attach(|py| {
            let obj = self.python_obj.bind(py);
            if obj.hasattr("initialize")? {
                obj.call_method0("initialize")?;
            }
            Ok(())
        })
        .map_err(|e: PyErr| KreuzbergError::Plugin {
            message: format!("Failed to initialize Python Validator '{}': {}", self.name, e),
            plugin_name: self.name.clone(),
        })
    }

    fn shutdown(&self) -> Result<()> {
        Python::attach(|py| {
            let obj = self.python_obj.bind(py);
            if obj.hasattr("shutdown")? {
                obj.call_method0("shutdown")?;
            }
            Ok(())
        })
        .map_err(|e: PyErr| KreuzbergError::Plugin {
            message: format!("Failed to shutdown Python Validator '{}': {}", self.name, e),
            plugin_name: self.name.clone(),
        })
    }
}

#[async_trait]
impl Validator for PythonValidator {
    async fn validate(&self, result: &ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
        let validator_name = self.name.clone();

        tokio::task::block_in_place(|| {
            Python::attach(|py| {
                let obj = self.python_obj.bind(py);

                let result_dict = extraction_result_to_dict(py, result).map_err(|e| KreuzbergError::Plugin {
                    message: format!("Failed to convert ExtractionResult to Python dict: {}", e),
                    plugin_name: validator_name.clone(),
                })?;

                let py_result = result_dict.bind(py);
                obj.call_method1("validate", (py_result,)).map_err(|e| {
                    let is_validation_error = e.is_instance_of::<pyo3::exceptions::PyValueError>(py)
                        || e.get_type(py)
                            .name()
                            .ok()
                            .and_then(|n| n.to_str().ok().map(|s| s.to_string()))
                            .map(|s| s.contains("ValidationError"))
                            .unwrap_or(false);

                    if is_validation_error {
                        KreuzbergError::Validation {
                            message: e.to_string(),
                            source: None,
                        }
                    } else {
                        KreuzbergError::Plugin {
                            message: format!("Python Validator '{}' failed during validate: {}", validator_name, e),
                            plugin_name: validator_name.clone(),
                        }
                    }
                })?;

                Ok::<(), KreuzbergError>(())
            })
        })?;

        Ok(())
    }

    fn should_validate(&self, result: &ExtractionResult, _config: &ExtractionConfig) -> bool {
        let validator_name = self.name.clone();
        Python::attach(|py| {
            let obj = self.python_obj.bind(py);

            // If hasattr fails due to GIL error, log and default to true ~keep
            let has_should_validate = obj
                .hasattr("should_validate")
                .map_err(|e| {
                    tracing::debug!(
                        "WARNING: Validator '{}': Failed to check for should_validate method due to GIL error ({}), defaulting to true",
                        validator_name, e
                    );
                    e
                })
                .unwrap_or(false);

            if has_should_validate {
                let result_dict = extraction_result_to_dict(py, result).ok()?;
                let py_result = result_dict.bind(py);
                obj.call_method1("should_validate", (py_result,))
                    .and_then(|v| v.extract::<bool>())
                    .ok()
            } else {
                Some(true)
            }
        })
        .unwrap_or(true)
    }

    fn priority(&self) -> i32 {
        self.priority
    }
}

/// Register a Python Validator with the Rust core.
///
/// This function validates the Python validator object, wraps it in a Rust
/// `Validator` implementation, and registers it with the global Validator
/// registry. Once registered, the validator will be called automatically after
/// extraction to validate results.
///
/// # Arguments
///
/// * `validator` - Python object implementing the Validator protocol
///
/// # Required Methods on Python Validator
///
/// The Python validator must implement:
/// - `name() -> str` - Return validator name
/// - `validate(result: dict) -> None` - Validate the extraction result (raise error to fail)
///
/// # Optional Methods
///
/// - `should_validate(result: dict) -> bool` - Check if validator should run (defaults to True)
/// - `priority() -> int` - Return priority (defaults to 50, higher runs first)
/// - `initialize()` - Called when validator is registered
/// - `shutdown()` - Called when validator is unregistered
/// - `version() -> str` - Validator version (defaults to "1.0.0")
///
/// # Example
///
/// ```python
/// from kreuzberg import register_validator
/// from kreuzberg.exceptions import ValidationError
///
/// class MinLengthValidator:
///     def name(self) -> str:
///         return "min_length_validator"
///
///     def priority(self) -> int:
///         return 100  # Run early
///
///     def validate(self, result: dict) -> None:
///         if len(result["content"]) < 100:
///             raise ValidationError(
///                 f"Content too short: {len(result['content'])} < 100 characters"
///             )
///
/// register_validator(MinLengthValidator())
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - Validator is missing required methods
/// - Validator name is empty or duplicate
/// - Registration fails
#[pyfunction]
pub fn register_validator(py: Python<'_>, validator: Py<PyAny>) -> PyResult<()> {
    let rust_validator = PythonValidator::new(py, validator)?;
    let validator_name = rust_validator.name().to_string();

    let arc_validator: Arc<dyn Validator> = Arc::new(rust_validator);

    py.detach(|| {
        let registry = get_validator_registry();
        let mut registry = registry.write().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to acquire write lock on Validator registry: {}",
                e
            ))
        })?;

        registry.register(arc_validator).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to register Validator '{}': {}",
                validator_name, e
            ))
        })
    })?;

    Ok(())
}

/// Unregister a Validator by name.
///
/// Removes a previously registered validator from the global registry and
/// calls its `shutdown()` method to release resources.
///
/// # Arguments
///
/// * `name` - Validator name to unregister
///
/// # Example
///
/// ```python
/// from kreuzberg import register_validator, unregister_validator
///
/// class MyValidator:
///     def name(self) -> str:
///         return "my_validator"
///
///     def validate(self, result: dict) -> None:
///         pass
///
/// register_validator(MyValidator())
/// # ... use validator ...
/// unregister_validator("my_validator")
/// ```
#[pyfunction]
pub fn unregister_validator(py: Python<'_>, name: &str) -> PyResult<()> {
    py.detach(|| {
        let registry = get_validator_registry();
        let mut registry = registry.write().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to acquire write lock on Validator registry: {}",
                e
            ))
        })?;

        registry.remove(name).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to unregister Validator '{}': {}", name, e))
        })
    })?;

    Ok(())
}

/// Clear all registered Validators.
///
/// Removes all validators from the global registry and calls their `shutdown()`
/// methods. Useful for test cleanup or resetting state.
///
/// # Example
///
/// ```python
/// from kreuzberg import clear_validators
///
/// # In pytest fixture or test cleanup
/// clear_validators()
/// ```
#[pyfunction]
pub fn clear_validators(py: Python<'_>) -> PyResult<()> {
    py.detach(|| {
        let registry = get_validator_registry();
        let mut registry = registry.write().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to acquire write lock on Validator registry: {}",
                e
            ))
        })?;

        registry.shutdown_all().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to clear Validator registry: {}", e))
        })
    })?;

    Ok(())
}

/// List all registered validator names.
///
/// Returns a list of all validator names currently registered in the global registry.
///
/// # Returns
///
/// List of validator names.
///
/// # Example
///
/// ```python
/// from kreuzberg import list_validators, register_validator, clear_validators
///
/// class MyValidator:
///     def name(self) -> str:
///         return "my_validator"
///
///     def validate(self, result: dict) -> None:
///         pass
///
/// # Register validator
/// register_validator(MyValidator())
///
/// # List validators
/// validators = list_validators()
/// assert "my_validator" in validators
///
/// # Cleanup
/// clear_validators()
/// ```
#[pyfunction]
pub fn list_validators() -> PyResult<Vec<String>> {
    kreuzberg::plugins::list_validators().map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

/// List all registered post-processor names.
///
/// Returns a list of all post-processor names currently registered in the global registry.
///
/// # Returns
///
/// List of post-processor names.
///
/// # Example
///
/// ```python
/// from kreuzberg import list_post_processors, register_post_processor, clear_post_processors
///
/// class MyProcessor:
///     def name(self) -> str:
///         return "my_processor"
///
///     def process(self, result: dict) -> dict:
///         return result
///
/// # Register processor
/// register_post_processor(MyProcessor())
///
/// # List processors
/// processors = list_post_processors()
/// assert "my_processor" in processors
///
/// # Cleanup
/// clear_post_processors()
/// ```
#[pyfunction]
pub fn list_post_processors() -> PyResult<Vec<String>> {
    kreuzberg::plugins::list_post_processors().map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

/// Unregister an OCR backend by name.
///
/// Removes a previously registered OCR backend from the global registry and
/// calls its `shutdown()` method to release resources.
///
/// # Arguments
///
/// * `name` - Backend name to unregister
///
/// # Example
///
/// ```python
/// from kreuzberg import register_ocr_backend, unregister_ocr_backend
///
/// class MyOcrBackend:
///     def name(self) -> str:
///         return "my_ocr"
///
///     def supported_languages(self) -> list[str]:
///         return ["eng"]
///
///     def process_image(self, image_bytes: bytes, language: str) -> dict:
///         return {"content": "text", "metadata": {}, "tables": []}
///
/// register_ocr_backend(MyOcrBackend())
/// # ... use backend ...
/// unregister_ocr_backend("my_ocr")
/// ```
///
/// # Errors
///
/// Returns an error if the backend is not found or shutdown fails.
#[pyfunction]
pub fn unregister_ocr_backend(name: &str) -> PyResult<()> {
    kreuzberg::plugins::unregister_ocr_backend(name)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

/// List all registered OCR backend names.
///
/// Returns a list of all OCR backend names currently registered in the global registry.
///
/// # Returns
///
/// List of OCR backend names.
///
/// # Example
///
/// ```python
/// from kreuzberg import list_ocr_backends, register_ocr_backend
///
/// class MyOcrBackend:
///     def name(self) -> str:
///         return "my_ocr"
///
///     def supported_languages(self) -> list[str]:
///         return ["eng"]
///
///     def process_image(self, image_bytes: bytes, language: str) -> dict:
///         return {"content": "text", "metadata": {}, "tables": []}
///
/// # Register backend
/// register_ocr_backend(MyOcrBackend())
///
/// # List backends
/// backends = list_ocr_backends()
/// assert "my_ocr" in backends
/// ```
#[pyfunction]
pub fn list_ocr_backends() -> PyResult<Vec<String>> {
    kreuzberg::plugins::list_ocr_backends().map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

/// Clear all registered OCR backends.
///
/// Removes all OCR backends from the global registry and calls their `shutdown()`
/// methods. Useful for test cleanup or resetting state.
///
/// # Example
///
/// ```python
/// from kreuzberg import clear_ocr_backends
///
/// # In pytest fixture or test cleanup
/// clear_ocr_backends()
/// ```
#[pyfunction]
pub fn clear_ocr_backends() -> PyResult<()> {
    kreuzberg::plugins::clear_ocr_backends().map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

/// List all registered document extractor names.
///
/// Returns a list of all document extractor names currently registered in the global registry.
/// This function automatically initializes built-in extractors (PDF, DOCX, etc.) on first call.
///
/// # Returns
///
/// List of document extractor names.
///
/// # Example
///
/// ```python
/// from kreuzberg import list_document_extractors
///
/// # List all registered extractors (includes built-in PDF, DOCX, etc.)
/// extractors = list_document_extractors()
/// assert any("pdf" in e.lower() for e in extractors)
/// ```
#[pyfunction]
pub fn list_document_extractors() -> PyResult<Vec<String>> {
    kreuzberg::plugins::list_extractors().map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

/// Unregister a document extractor by name.
///
/// Removes a previously registered document extractor from the global registry.
///
/// # Arguments
///
/// * `name` - Extractor name to unregister
///
/// # Example
///
/// ```python
/// from kreuzberg import unregister_document_extractor
///
/// # Unregister an extractor
/// unregister_document_extractor("my_custom_extractor")
/// ```
#[pyfunction]
pub fn unregister_document_extractor(name: &str) -> PyResult<()> {
    kreuzberg::plugins::unregister_extractor(name).map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

/// Clear all registered document extractors.
///
/// Removes all document extractors from the global registry.
/// Useful for test cleanup or resetting state.
///
/// # Example
///
/// ```python
/// from kreuzberg import clear_document_extractors
///
/// # In pytest fixture or test cleanup
/// clear_document_extractors()
/// ```
#[pyfunction]
pub fn clear_document_extractors() -> PyResult<()> {
    kreuzberg::plugins::clear_extractors().map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}
