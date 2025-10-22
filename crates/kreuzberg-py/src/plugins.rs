//! Plugin registration functions
//!
//! Allows Python-based plugins (OCR backends, PostProcessors) to register
//! with the Rust core.
//!
//! This module provides the FFI bridge that enables:
//! - Python OCR backends (EasyOCR, PaddleOCR, etc.) to be used by Rust extraction
//! - Python PostProcessors (entity extraction, keyword extraction, etc.) to enrich results

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

    async fn process_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractionResult> {
        let has_process_file = Python::attach(|py| self.python_obj.bind(py).hasattr("process_file").unwrap_or(false));

        if has_process_file {
            let path_str = path.to_string_lossy().to_string();
            let language = config.language.clone();
            let backend_name = self.name.clone();

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
        Ok(t) if !t.is_none() => extract_tables(&t).unwrap_or_default(),
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

        let json_value = python_to_json(&value)?;
        metadata.insert(key_str, json_value);
    }

    Ok(metadata)
}

/// Extract tables from Python object.
fn extract_tables(_obj: &Bound<'_, PyAny>) -> Result<Vec<Table>> {
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

            let processed_dict = processed.downcast::<PyDict>().map_err(|e| KreuzbergError::Plugin {
                message: format!("PostProcessor did not return a dict: {}", e),
                plugin_name: processor_name.clone(),
            })?;

            merge_dict_to_extraction_result(py, processed_dict, result)?;

            Ok(())
        })
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
        && let Ok(meta_dict) = m.downcast::<PyDict>()
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

        if !obj.hasattr("name")? {
            return Err(pyo3::exceptions::PyAttributeError::new_err(
                "Validator must have a 'name()' method",
            ));
        }
        if !obj.hasattr("validate")? {
            return Err(pyo3::exceptions::PyAttributeError::new_err(
                "Validator must have a 'validate(result: dict) -> None' method",
            ));
        }

        let name: String = obj.call_method0("name")?.extract()?;
        if name.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Validator name cannot be empty",
            ));
        }

        let priority = if obj.hasattr("priority")? {
            obj.call_method0("priority")?.extract()?
        } else {
            50 // Default priority
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

        Python::attach(|py| {
            let obj = self.python_obj.bind(py);

            let result_dict = extraction_result_to_dict(py, result).map_err(|e| KreuzbergError::Plugin {
                message: format!("Failed to convert ExtractionResult to Python dict: {}", e),
                plugin_name: validator_name.clone(),
            })?;

            let py_result = result_dict.bind(py);
            obj
                .call_method1("validate", (py_result,))
                .map_err(|e| {
                    // Check if it's a ValidationError from Python
                    let is_validation_error = e.is_instance_of::<pyo3::exceptions::PyValueError>(py)
                        || e.get_type(py).name()
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

            Ok(())
        })
    }

    fn should_validate(&self, result: &ExtractionResult, _config: &ExtractionConfig) -> bool {
        Python::attach(|py| {
            let obj = self.python_obj.bind(py);

            if obj.hasattr("should_validate").unwrap_or(false) {
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
