//! Plugin registration functions
//!
//! Allows Python-based plugins (OCR backends, PostProcessors) to register
//! with the Rust core.
//!
//! This module provides the FFI bridge that enables:
//! - Python OCR backends (EasyOCR, PaddleOCR, etc.) to be used by Rust extraction
//! - Python PostProcessors (entity extraction, keyword extraction, etc.) to enrich results

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PyString};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use kreuzberg::core::config::{ExtractionConfig, OcrConfig};
use kreuzberg::plugins::registry::{get_ocr_backend_registry, get_post_processor_registry};
use kreuzberg::plugins::{OcrBackend, OcrBackendType, Plugin, PostProcessor, ProcessingStage};
use kreuzberg::types::{ExtractionResult, Table};
use kreuzberg::{KreuzbergError, Result};

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

        // Validate required methods
        if !obj.hasattr("name")? {
            return Err(pyo3::exceptions::PyAttributeError::new_err(
                "OCR backend must have a 'name()' method",
            ));
        }
        if !obj.hasattr("supported_languages")? {
            return Err(pyo3::exceptions::PyAttributeError::new_err(
                "OCR backend must have a 'supported_languages()' method",
            ));
        }
        if !obj.hasattr("process_image")? {
            return Err(pyo3::exceptions::PyAttributeError::new_err(
                "OCR backend must have a 'process_image(image_bytes, language)' method",
            ));
        }

        // Get backend name
        let name: String = obj.call_method0("name")?.extract()?;
        if name.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "OCR backend name cannot be empty",
            ));
        }

        // Get supported languages
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
        // Try to get version from Python, fallback to default
        // SAFETY: Using attach for safe GIL acquisition (PyO3 0.26+ recommended approach)
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
        // SAFETY: Using attach for safe GIL acquisition (PyO3 0.26+ recommended approach)
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
        // SAFETY: Using attach for safe GIL acquisition (PyO3 0.26+ recommended approach)
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

        // Clone the Python object reference with GIL
        // SAFETY: Using attach for safe GIL acquisition (PyO3 0.26+ recommended approach) before passing to spawn_blocking
        let python_obj = Python::attach(|py| self.python_obj.clone_ref(py));

        // Use spawn_blocking to avoid blocking the async runtime with Python/GIL
        tokio::task::spawn_blocking(move || {
            // SAFETY: Using attach for safe GIL acquisition (PyO3 0.26+ recommended approach) in blocking task
            Python::attach(|py| {
                let obj = python_obj.bind(py);

                // Convert Rust types → Python
                let py_bytes = PyBytes::new(py, &image_bytes);

                // Call Python method: process_image(image_bytes, language)
                let result = obj
                    .call_method1("process_image", (py_bytes, language.as_str()))
                    .map_err(|e| KreuzbergError::Ocr {
                        message: format!(
                            "Python OCR backend '{}' failed during process_image: {}",
                            backend_name, e
                        ),
                        source: Some(Box::new(e)),
                    })?;

                // Convert Python dict → Rust ExtractionResult
                dict_to_extraction_result(py, &result)
            })
        })
        .await
        .map_err(|e| KreuzbergError::Ocr {
            message: format!("Failed to spawn blocking task for Python OCR backend: {}", e),
            source: Some(Box::new(e)),
        })?
    }

    async fn process_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractionResult> {
        // Check if Python backend has custom process_file implementation
        // SAFETY: Using attach for safe GIL acquisition (PyO3 0.26+ recommended approach)
        let has_process_file = Python::attach(|py| self.python_obj.bind(py).hasattr("process_file").unwrap_or(false));

        if has_process_file {
            let path_str = path.to_string_lossy().to_string();
            let language = config.language.clone();
            let backend_name = self.name.clone();

            // Clone the Python object reference with GIL
            // SAFETY: Using attach for safe GIL acquisition (PyO3 0.26+ recommended approach) before passing to spawn_blocking
            let python_obj = Python::attach(|py| self.python_obj.clone_ref(py));

            tokio::task::spawn_blocking(move || {
                // SAFETY: Using attach for safe GIL acquisition (PyO3 0.26+ recommended approach) in blocking task
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
        } else {
            // Default implementation: read file and call process_image
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

// ============================================================================
// Type Conversion Helpers
// ============================================================================

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
    // Extract content (required)
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

    // Extract metadata (optional, default to empty)
    let metadata = match dict.get_item("metadata") {
        Ok(m) if !m.is_none() => extract_metadata(&m).unwrap_or_default(),
        _ => HashMap::new(),
    };

    // Extract tables (optional, default to empty)
    let tables = match dict.get_item("tables") {
        Ok(t) if !t.is_none() => extract_tables(&t).unwrap_or_default(),
        _ => vec![],
    };

    Ok(ExtractionResult {
        content,
        mime_type: "text/plain".to_string(),
        metadata,
        tables,
        detected_languages: None,
    })
}

/// Extract metadata dict from Python object.
fn extract_metadata(obj: &Bound<'_, PyAny>) -> Result<HashMap<String, serde_json::Value>> {
    let dict = obj.downcast::<PyDict>().map_err(|_| KreuzbergError::Validation {
        message: "Metadata must be a dict".to_string(),
        source: None,
    })?;

    let mut metadata = HashMap::new();
    for (key, value) in dict.iter() {
        let key_str: String = key.extract().map_err(|_| KreuzbergError::Validation {
            message: "Metadata keys must be strings".to_string(),
            source: None,
        })?;

        // Convert Python value to serde_json::Value
        let json_value = python_to_json(&value)?;
        metadata.insert(key_str, json_value);
    }

    Ok(metadata)
}

/// Extract tables from Python object.
fn extract_tables(_obj: &Bound<'_, PyAny>) -> Result<Vec<Table>> {
    // For now, tables support is optional - return empty vec
    // TODO: Implement full table extraction from Python
    Ok(vec![])
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
    } else if let Ok(list) = obj.downcast::<PyList>() {
        let mut vec = Vec::new();
        for item in list.iter() {
            vec.push(python_to_json(&item)?);
        }
        Ok(serde_json::Value::Array(vec))
    } else if let Ok(dict) = obj.downcast::<PyDict>() {
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
        // Fallback: convert to string
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

// ============================================================================
// PyO3 Functions (exposed to Python)
// ============================================================================

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
    // Create wrapper
    let rust_backend = PythonOcrBackend::new(py, backend)?;
    let backend_name = rust_backend.name().to_string();

    // Wrap in Arc for thread-safe sharing
    let arc_backend: Arc<dyn OcrBackend> = Arc::new(rust_backend);

    // Register with global registry (release GIL before locking registry)
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

/// List all registered OCR backends.
///
/// Returns a list of backend names currently registered in the global
/// OCR backend registry. This includes both native Rust backends
/// (like Tesseract) and Python backends registered via `register_ocr_backend()`.
///
/// # Returns
///
/// List of backend names as strings.
///
/// # Example
///
/// ```python
/// from kreuzberg import list_ocr_backends
///
/// backends = list_ocr_backends()
/// print(backends)  # ['tesseract', 'easyocr', 'paddleocr']
/// ```
#[pyfunction]
pub fn list_ocr_backends(py: Python) -> PyResult<Py<PyList>> {
    let backend_names = py.detach(|| {
        let registry = get_ocr_backend_registry();
        let registry = registry.read().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire read lock on OCR registry: {}", e))
        })?;

        Ok::<Vec<String>, PyErr>(registry.list())
    })?;

    let list = PyList::empty(py);
    for name in backend_names {
        list.append(name)?;
    }
    Ok(list.unbind())
}

/// Unregister an OCR backend.
///
/// Removes a backend from the global OCR backend registry and calls
/// its `shutdown()` method if available. After unregistering, the
/// backend can no longer be used for OCR processing.
///
/// # Arguments
///
/// * `name` - Name of the backend to unregister
///
/// # Example
///
/// ```python
/// from kreuzberg import unregister_ocr_backend
///
/// unregister_ocr_backend("easyocr")
/// ```
///
/// # Errors
///
/// Returns an error if the backend is not found or shutdown fails.
#[pyfunction]
pub fn unregister_ocr_backend(py: Python<'_>, name: String) -> PyResult<()> {
    py.detach(|| {
        let registry = get_ocr_backend_registry();
        let mut registry = registry.write().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire write lock on OCR registry: {}", e))
        })?;

        registry.remove(&name).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to unregister OCR backend '{}': {}", name, e))
        })
    })?;

    Ok(())
}

// ============================================================================
// Python PostProcessor FFI Bridge
// ============================================================================

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

        // Validate required methods
        if !obj.hasattr("name")? {
            return Err(pyo3::exceptions::PyAttributeError::new_err(
                "PostProcessor must have a 'name()' method",
            ));
        }
        if !obj.hasattr("process")? {
            return Err(pyo3::exceptions::PyAttributeError::new_err(
                "PostProcessor must have a 'process(result: dict) -> dict' method",
            ));
        }

        // Get processor name
        let name: String = obj.call_method0("name")?.extract()?;
        if name.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "PostProcessor name cannot be empty",
            ));
        }

        // Get processing stage (optional, default to Middle)
        let stage = if obj.hasattr("processing_stage")? {
            let stage_str: String = obj.call_method0("processing_stage")?.extract()?;
            match stage_str.to_lowercase().as_str() {
                "early" => ProcessingStage::Early,
                "middle" => ProcessingStage::Middle,
                "late" => ProcessingStage::Late,
                _ => ProcessingStage::Middle, // Default
            }
        } else {
            ProcessingStage::Middle // Default
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
        // Try to get version from Python, fallback to default
        // SAFETY: Using attach for safe GIL acquisition (PyO3 0.26+ recommended approach)
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
        // SAFETY: Using attach for safe GIL acquisition (PyO3 0.26+ recommended approach)
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
        // SAFETY: Using attach for safe GIL acquisition (PyO3 0.26+ recommended approach)
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

        // Clone Python object reference with GIL
        // SAFETY: Using attach for safe GIL acquisition (PyO3 0.26+ recommended approach) before passing to spawn_blocking
        let python_obj = Python::attach(|py| self.python_obj.clone_ref(py));

        // Convert ExtractionResult to Python dict (need to clone for thread safety)
        // SAFETY: Using attach for safe GIL acquisition (PyO3 0.26+ recommended approach) before passing to spawn_blocking
        let result_dict =
            Python::attach(|py| extraction_result_to_dict(py, result)).map_err(|e| KreuzbergError::Plugin {
                message: format!("Failed to convert ExtractionResult to Python dict: {}", e),
                plugin_name: self.name.clone(),
            })?;

        // Use spawn_blocking to avoid blocking async runtime
        let processed_dict = tokio::task::spawn_blocking(move || {
            // SAFETY: Using attach for safe GIL acquisition (PyO3 0.26+ recommended approach) in blocking task
            Python::attach(|py| {
                let obj = python_obj.bind(py);

                // Convert Rust dict → Python
                let py_result = result_dict.bind(py);

                // Call Python method: process(result: dict) -> dict
                let processed = obj
                    .call_method1("process", (py_result,))
                    .map_err(|e| KreuzbergError::Plugin {
                        message: format!("Python PostProcessor '{}' failed during process: {}", processor_name, e),
                        plugin_name: processor_name.clone(),
                    })?;

                // Return the processed dict as Py<PyDict>
                processed.extract::<Py<PyDict>>().map_err(|e| KreuzbergError::Plugin {
                    message: format!("Failed to extract Python dict from process result: {}", e),
                    plugin_name: processor_name.clone(),
                })
            })
        })
        .await
        .map_err(|e| KreuzbergError::Plugin {
            message: format!("Failed to spawn blocking task for Python PostProcessor: {}", e),
            plugin_name: self.name.clone(),
        })??;

        // Merge the processed result back into the original result
        // SAFETY: Using attach for safe GIL acquisition (PyO3 0.26+ recommended approach) after spawn_blocking completes
        Python::attach(|py| {
            let dict = processed_dict.bind(py);
            merge_dict_to_extraction_result(py, dict, result)
        })?;

        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        self.stage
    }
}

// ============================================================================
// ExtractionResult ↔ Python Dict Conversions
// ============================================================================

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

    // Add content
    dict.set_item("content", &result.content)?;

    // Add mime_type
    dict.set_item("mime_type", &result.mime_type)?;

    // Add metadata
    let metadata_dict = PyDict::new(py);
    for (key, value) in &result.metadata {
        // Convert serde_json::Value to Python
        let py_value = json_to_python(py, value)?;
        metadata_dict.set_item(key, py_value)?;
    }
    dict.set_item("metadata", metadata_dict)?;

    // Add tables (simplified - just empty list for now)
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
    // Update content if present
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

    // Merge metadata (don't overwrite existing keys)
    if let Some(m) = dict.get_item("metadata").map_err(|e| KreuzbergError::Plugin {
        message: format!("Failed to get 'metadata' from result dict: {}", e),
        plugin_name: "python".to_string(),
    })? && !m.is_none()
        && let Ok(meta_dict) = m.downcast::<PyDict>()
    {
        for (key, value) in meta_dict.iter() {
            let key_str: String = key.extract().map_err(|_| KreuzbergError::Plugin {
                message: "Metadata keys must be strings".to_string(),
                plugin_name: "python".to_string(),
            })?;

            // Only add if key doesn't exist (don't overwrite)
            use std::collections::hash_map::Entry;
            if let Entry::Vacant(e) = result.metadata.entry(key_str) {
                let json_value = python_to_json(&value)?;
                e.insert(json_value);
            }
        }
    }

    Ok(())
}

/// Convert serde_json::Value to Python object.
fn json_to_python<'a>(py: Python<'a>, value: &'a serde_json::Value) -> PyResult<Bound<'a, PyAny>> {
    match value {
        serde_json::Value::Null => Ok(py.None().into_bound(py)),
        serde_json::Value::Bool(b) => {
            use pyo3::types::PyBool;
            Ok(PyBool::new(py, *b).as_any().clone())
        }
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                use pyo3::types::PyInt;
                Ok(PyInt::new(py, i).as_any().clone())
            } else if let Some(f) = n.as_f64() {
                use pyo3::types::PyFloat;
                Ok(PyFloat::new(py, f).as_any().clone())
            } else {
                Ok(py.None().into_bound(py))
            }
        }
        serde_json::Value::String(s) => {
            use pyo3::types::PyString;
            Ok(PyString::new(py, s).as_any().clone())
        }
        serde_json::Value::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(json_to_python(py, item)?)?;
            }
            Ok(list.into_any())
        }
        serde_json::Value::Object(obj) => {
            let dict = PyDict::new(py);
            for (key, val) in obj {
                dict.set_item(key, json_to_python(py, val)?)?;
            }
            Ok(dict.into_any())
        }
    }
}

// ============================================================================
// PostProcessor Registration Functions (PyO3 exposed)
// ============================================================================

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
    // Create wrapper
    let rust_processor = PythonPostProcessor::new(py, processor)?;
    let processor_name = rust_processor.name().to_string();

    // Wrap in Arc for thread-safe sharing
    let arc_processor: Arc<dyn PostProcessor> = Arc::new(rust_processor);

    // Register with global registry (release GIL before locking registry)
    py.detach(|| {
        let registry = get_post_processor_registry();
        let mut registry = registry.write().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to acquire write lock on PostProcessor registry: {}",
                e
            ))
        })?;

        // Register with default priority of 0
        registry.register(arc_processor, 0).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to register PostProcessor '{}': {}",
                processor_name, e
            ))
        })
    })?;

    Ok(())
}

/// List all registered PostProcessors.
///
/// Returns a list of processor names currently registered in the global
/// PostProcessor registry. This includes both native Rust processors
/// and Python processors registered via `register_post_processor()`.
///
/// # Returns
///
/// List of processor names as strings.
///
/// # Example
///
/// ```python
/// from kreuzberg import list_post_processors
///
/// processors = list_post_processors()
/// print(processors)  # ['entity_extraction', 'keyword_extraction', 'category_extraction']
/// ```
#[pyfunction]
pub fn list_post_processors(py: Python) -> PyResult<Py<PyList>> {
    let processor_names = py.detach(|| {
        let registry = get_post_processor_registry();
        let registry = registry.read().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to acquire read lock on PostProcessor registry: {}",
                e
            ))
        })?;

        Ok::<Vec<String>, PyErr>(registry.list())
    })?;

    let list = PyList::empty(py);
    for name in processor_names {
        list.append(name)?;
    }
    Ok(list.unbind())
}

/// Unregister a PostProcessor.
///
/// Removes a processor from the global PostProcessor registry and calls
/// its `shutdown()` method if available. After unregistering, the
/// processor will no longer be called during extraction.
///
/// # Arguments
///
/// * `name` - Name of the processor to unregister
///
/// # Example
///
/// ```python
/// from kreuzberg import unregister_post_processor
///
/// unregister_post_processor("entity_extraction")
/// ```
///
/// # Errors
///
/// Returns an error if the processor is not found or shutdown fails.
#[pyfunction]
pub fn unregister_post_processor(py: Python<'_>, name: String) -> PyResult<()> {
    py.detach(|| {
        let registry = get_post_processor_registry();
        let mut registry = registry.write().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to acquire write lock on PostProcessor registry: {}",
                e
            ))
        })?;

        registry.remove(&name).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to unregister PostProcessor '{}': {}", name, e))
        })
    })?;

    Ok(())
}
