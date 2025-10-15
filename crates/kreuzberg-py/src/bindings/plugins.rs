//! FFI bridge for Python plugins.
//!
//! This module provides wrappers that allow Python classes to implement
//! Kreuzberg plugin traits (Plugin, OcrBackend, DocumentExtractor, etc.).

use crate::error::to_py_err;
use async_trait::async_trait;
use kreuzberg::core::config::{ExtractionConfig, OcrConfig};
use kreuzberg::plugins::{
    DocumentExtractor, OcrBackend, OcrBackendType, Plugin, PostProcessor, ProcessingStage, Validator,
};
use kreuzberg::types::ExtractionResult;
use kreuzberg::{KreuzbergError, Result};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyString};
use std::sync::Arc;

/// Wrapper for Python plugin instances.
///
/// This struct wraps a Python object that implements the Plugin protocol
/// and provides the Rust Plugin trait implementation.
pub struct PyPlugin {
    /// Reference to the Python plugin instance
    py_instance: Py<PyAny>,
}

impl PyPlugin {
    /// Create a new PyPlugin wrapper.
    pub fn new(py_instance: Py<PyAny>) -> Self {
        Self { py_instance }
    }
}

impl Plugin for PyPlugin {
    fn name(&self) -> &str {
        Python::with_gil(|py| {
            self.py_instance
                .call_method0(py, "name")
                .and_then(|result| result.extract::<String>(py))
                .unwrap_or_else(|_| "unknown".to_string())
        })
        .leak()
    }

    fn version(&self) -> &str {
        Python::with_gil(|py| {
            self.py_instance
                .call_method0(py, "version")
                .and_then(|result| result.extract::<String>(py))
                .unwrap_or_else(|_| "0.0.0".to_string())
        })
        .leak()
    }

    fn initialize(&self) -> Result<()> {
        Python::with_gil(|py| {
            self.py_instance
                .call_method0(py, "initialize")
                .map_err(|e| KreuzbergError::Plugin {
                    message: format!("Python plugin initialization failed: {}", e),
                    plugin_name: self.name().to_string(),
                })?;
            Ok(())
        })
    }

    fn shutdown(&self) -> Result<()> {
        Python::with_gil(|py| {
            self.py_instance
                .call_method0(py, "shutdown")
                .map_err(|e| KreuzbergError::Plugin {
                    message: format!("Python plugin shutdown failed: {}", e),
                    plugin_name: self.name().to_string(),
                })?;
            Ok(())
        })
    }

    fn description(&self) -> &str {
        Python::with_gil(|py| {
            self.py_instance
                .call_method0(py, "description")
                .and_then(|result| result.extract::<String>(py))
                .ok()
        })
        .unwrap_or_default()
        .leak()
    }

    fn author(&self) -> &str {
        Python::with_gil(|py| {
            self.py_instance
                .call_method0(py, "author")
                .and_then(|result| result.extract::<String>(py))
                .ok()
        })
        .unwrap_or_default()
        .leak()
    }
}

/// Wrapper for Python OCR backend instances.
pub struct PyOcrBackendWrapper {
    base: PyPlugin,
}

impl PyOcrBackendWrapper {
    /// Create a new PyOcrBackendWrapper.
    pub fn new(py_instance: Py<PyAny>) -> Self {
        Self {
            base: PyPlugin::new(py_instance),
        }
    }
}

impl Plugin for PyOcrBackendWrapper {
    fn name(&self) -> &str {
        self.base.name()
    }

    fn version(&self) -> &str {
        self.base.version()
    }

    fn initialize(&self) -> Result<()> {
        self.base.initialize()
    }

    fn shutdown(&self) -> Result<()> {
        self.base.shutdown()
    }

    fn description(&self) -> &str {
        self.base.description()
    }

    fn author(&self) -> &str {
        self.base.author()
    }
}

#[async_trait]
impl OcrBackend for PyOcrBackendWrapper {
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractionResult> {
        // Call Python method in blocking context (Python GIL is not async-safe)
        let py_instance = self.base.py_instance.clone();
        let image_bytes = image_bytes.to_vec();
        let config = config.clone();

        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                let py_bytes = PyBytes::new(py, &image_bytes);
                let config_dict = config_to_py_dict(py, &config)?;

                let result = py_instance
                    .call_method1(py, "process_image", (py_bytes, config_dict))
                    .map_err(to_py_err)?;

                py_dict_to_extraction_result(py, result.bind(py))
            })
        })
        .await
        .map_err(|e| KreuzbergError::Other(format!("Tokio join error: {}", e)))?
    }

    fn supports_language(&self, lang: &str) -> bool {
        Python::with_gil(|py| {
            self.base
                .py_instance
                .call_method1(py, "supports_language", (lang,))
                .and_then(|result| result.extract::<bool>(py))
                .unwrap_or(false)
        })
    }

    fn backend_type(&self) -> OcrBackendType {
        Python::with_gil(|py| {
            self.base
                .py_instance
                .call_method0(py, "backend_type")
                .and_then(|result| result.extract::<String>(py))
                .ok()
                .and_then(|s| match s.as_str() {
                    "tesseract" => Some(OcrBackendType::Tesseract),
                    "easyocr" => Some(OcrBackendType::EasyOCR),
                    "paddleocr" => Some(OcrBackendType::PaddleOCR),
                    _ => Some(OcrBackendType::Custom),
                })
                .unwrap_or(OcrBackendType::Custom)
        })
    }
}

/// Wrapper for Python document extractor instances.
pub struct PyDocumentExtractorWrapper {
    base: PyPlugin,
    mime_types: Vec<&'static str>,
    priority: i32,
}

impl PyDocumentExtractorWrapper {
    /// Create a new PyDocumentExtractorWrapper.
    pub fn new(py_instance: Py<PyAny>) -> Result<Self> {
        let (mime_types, priority) = Python::with_gil(|py| {
            let mime_types = py_instance
                .call_method0(py, "supported_mime_types")
                .and_then(|result| result.extract::<Vec<String>>(py))
                .map_err(to_py_err)?
                .into_iter()
                .map(|s| Box::leak(s.into_boxed_str()) as &'static str)
                .collect();

            let priority = py_instance
                .call_method0(py, "priority")
                .and_then(|result| result.extract::<i32>(py))
                .unwrap_or(50);

            Ok::<_, KreuzbergError>((mime_types, priority))
        })?;

        Ok(Self {
            base: PyPlugin::new(py_instance),
            mime_types,
            priority,
        })
    }
}

impl Plugin for PyDocumentExtractorWrapper {
    fn name(&self) -> &str {
        self.base.name()
    }

    fn version(&self) -> &str {
        self.base.version()
    }

    fn initialize(&self) -> Result<()> {
        self.base.initialize()
    }

    fn shutdown(&self) -> Result<()> {
        self.base.shutdown()
    }

    fn description(&self) -> &str {
        self.base.description()
    }

    fn author(&self) -> &str {
        self.base.author()
    }
}

#[async_trait]
impl DocumentExtractor for PyDocumentExtractorWrapper {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let py_instance = self.base.py_instance.clone();
        let content = content.to_vec();
        let mime_type = mime_type.to_string();
        let config = config.clone();

        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                let py_bytes = PyBytes::new(py, &content);
                let py_mime_type = PyString::new(py, &mime_type);
                let config_dict = extraction_config_to_py_dict(py, &config)?;

                let result = py_instance
                    .call_method1(py, "extract_bytes", (py_bytes, py_mime_type, config_dict))
                    .map_err(to_py_err)?;

                py_dict_to_extraction_result(py, result.bind(py))
            })
        })
        .await
        .map_err(|e| KreuzbergError::Other(format!("Tokio join error: {}", e)))?
    }

    fn supported_mime_types(&self) -> &[&str] {
        &self.mime_types
    }

    fn priority(&self) -> i32 {
        self.priority
    }
}

/// Wrapper for Python post-processor instances.
pub struct PyPostProcessorWrapper {
    base: PyPlugin,
    stage: ProcessingStage,
}

impl PyPostProcessorWrapper {
    /// Create a new PyPostProcessorWrapper.
    pub fn new(py_instance: Py<PyAny>) -> Result<Self> {
        let stage = Python::with_gil(|py| {
            py_instance
                .call_method0(py, "processing_stage")
                .and_then(|result| result.extract::<String>(py))
                .map_err(to_py_err)
                .and_then(|s| match s.as_str() {
                    "early" => Ok(ProcessingStage::Early),
                    "middle" => Ok(ProcessingStage::Middle),
                    "late" => Ok(ProcessingStage::Late),
                    _ => Err(KreuzbergError::Validation(format!("Invalid processing stage: {}", s))),
                })
        })?;

        Ok(Self {
            base: PyPlugin::new(py_instance),
            stage,
        })
    }
}

impl Plugin for PyPostProcessorWrapper {
    fn name(&self) -> &str {
        self.base.name()
    }

    fn version(&self) -> &str {
        self.base.version()
    }

    fn initialize(&self) -> Result<()> {
        self.base.initialize()
    }

    fn shutdown(&self) -> Result<()> {
        self.base.shutdown()
    }

    fn description(&self) -> &str {
        self.base.description()
    }

    fn author(&self) -> &str {
        self.base.author()
    }
}

#[async_trait]
impl PostProcessor for PyPostProcessorWrapper {
    async fn process(&self, result: ExtractionResult, config: &ExtractionConfig) -> Result<ExtractionResult> {
        let py_instance = self.base.py_instance.clone();
        let config = config.clone();

        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                let result_dict = extraction_result_to_py_dict(py, &result)?;
                let config_dict = extraction_config_to_py_dict(py, &config)?;

                let py_result = py_instance
                    .call_method1(py, "process", (result_dict, config_dict))
                    .map_err(to_py_err)?;

                py_dict_to_extraction_result(py, py_result.bind(py))
            })
        })
        .await
        .map_err(|e| KreuzbergError::Other(format!("Tokio join error: {}", e)))?
    }

    fn processing_stage(&self) -> ProcessingStage {
        self.stage
    }
}

/// Wrapper for Python validator instances.
pub struct PyValidatorWrapper {
    base: PyPlugin,
    priority: i32,
}

impl PyValidatorWrapper {
    /// Create a new PyValidatorWrapper.
    pub fn new(py_instance: Py<PyAny>) -> Result<Self> {
        let priority = Python::with_gil(|py| {
            py_instance
                .call_method0(py, "priority")
                .and_then(|result| result.extract::<i32>(py))
                .unwrap_or(50)
        });

        Ok(Self {
            base: PyPlugin::new(py_instance),
            priority,
        })
    }
}

impl Plugin for PyValidatorWrapper {
    fn name(&self) -> &str {
        self.base.name()
    }

    fn version(&self) -> &str {
        self.base.version()
    }

    fn initialize(&self) -> Result<()> {
        self.base.initialize()
    }

    fn shutdown(&self) -> Result<()> {
        self.base.shutdown()
    }

    fn description(&self) -> &str {
        self.base.description()
    }

    fn author(&self) -> &str {
        self.base.author()
    }
}

#[async_trait]
impl Validator for PyValidatorWrapper {
    async fn validate(&self, result: &ExtractionResult, config: &ExtractionConfig) -> Result<()> {
        let py_instance = self.base.py_instance.clone();
        let result = result.clone();
        let config = config.clone();

        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                let result_dict = extraction_result_to_py_dict(py, &result)?;
                let config_dict = extraction_config_to_py_dict(py, &config)?;

                py_instance
                    .call_method1(py, "validate", (result_dict, config_dict))
                    .map_err(to_py_err)?;

                Ok(())
            })
        })
        .await
        .map_err(|e| KreuzbergError::Other(format!("Tokio join error: {}", e)))?
    }

    fn priority(&self) -> i32 {
        self.priority
    }
}

// Helper functions for type conversion

fn config_to_py_dict(py: Python, _config: &OcrConfig) -> Result<Py<PyDict>> {
    let dict = PyDict::new(py);
    // TODO: Add config fields
    Ok(dict.into())
}

fn extraction_config_to_py_dict(py: Python, _config: &ExtractionConfig) -> Result<Py<PyDict>> {
    let dict = PyDict::new(py);
    // TODO: Add config fields
    Ok(dict.into())
}

fn extraction_result_to_py_dict(py: Python, result: &ExtractionResult) -> Result<Py<PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("content", &result.content)
        .map_err(|e| KreuzbergError::Other(e.to_string()))?;
    dict.set_item("mime_type", &result.mime_type)
        .map_err(|e| KreuzbergError::Other(e.to_string()))?;
    // TODO: Add metadata and tables
    Ok(dict.into())
}

fn py_dict_to_extraction_result(py: Python, obj: &Bound<'_, PyAny>) -> Result<ExtractionResult> {
    let content = obj
        .get_item("content")
        .map_err(|e| KreuzbergError::Other(e.to_string()))?
        .ok_or_else(|| KreuzbergError::Parsing("Missing 'content' field".to_string()))?
        .extract::<String>()
        .map_err(|e| KreuzbergError::Other(e.to_string()))?;

    let mime_type = obj
        .get_item("mime_type")
        .map_err(|e| KreuzbergError::Other(e.to_string()))?
        .ok_or_else(|| KreuzbergError::Parsing("Missing 'mime_type' field".to_string()))?
        .extract::<String>()
        .map_err(|e| KreuzbergError::Other(e.to_string()))?;

    Ok(ExtractionResult {
        content,
        mime_type,
        metadata: std::collections::HashMap::new(),
        tables: vec![],
    })
}

/// Register a Python OCR backend.
#[pyfunction]
pub fn register_ocr_backend(py_instance: Py<PyAny>) -> PyResult<()> {
    let wrapper = Arc::new(PyOcrBackendWrapper::new(py_instance));
    let registry = kreuzberg::plugins::registry::get_ocr_backend_registry();
    registry.write().unwrap().register(wrapper).map_err(to_py_err)?;
    Ok(())
}

/// Register a Python document extractor.
#[pyfunction]
pub fn register_document_extractor(py_instance: Py<PyAny>) -> PyResult<()> {
    let wrapper = Arc::new(PyDocumentExtractorWrapper::new(py_instance).map_err(to_py_err)?);
    let registry = kreuzberg::plugins::registry::get_document_extractor_registry();
    registry.write().unwrap().register(wrapper).map_err(to_py_err)?;
    Ok(())
}

/// Register a Python post-processor.
#[pyfunction]
pub fn register_post_processor(py_instance: Py<PyAny>, priority: i32) -> PyResult<()> {
    let wrapper = Arc::new(PyPostProcessorWrapper::new(py_instance).map_err(to_py_err)?);
    let registry = kreuzberg::plugins::registry::get_post_processor_registry();
    registry
        .write()
        .unwrap()
        .register(wrapper, priority)
        .map_err(to_py_err)?;
    Ok(())
}

/// Register a Python validator.
#[pyfunction]
pub fn register_validator(py_instance: Py<PyAny>) -> PyResult<()> {
    let wrapper = Arc::new(PyValidatorWrapper::new(py_instance).map_err(to_py_err)?);
    let registry = kreuzberg::plugins::registry::get_validator_registry();
    registry.write().unwrap().register(wrapper).map_err(to_py_err)?;
    Ok(())
}

/// Register plugin functions with the Python module.
pub fn register_plugin_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(register_ocr_backend, m)?)?;
    m.add_function(wrap_pyfunction!(register_document_extractor, m)?)?;
    m.add_function(wrap_pyfunction!(register_post_processor, m)?)?;
    m.add_function(wrap_pyfunction!(register_validator, m)?)?;
    Ok(())
}
