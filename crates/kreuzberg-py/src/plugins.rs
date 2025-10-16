//! Plugin registration functions
//!
//! Allows Python-based OCR backends to register with the Rust core.
//!
//! **Note**: This is a stub implementation. Full Python OCR backend registration
//! will be implemented in Phase 4B after core bindings are complete.

use pyo3::prelude::*;

// Placeholder for future implementation
// TODO(v4.1): Implement Python OCR backend registration
//
// Design:
// - Accept Python callable that implements OCR interface
// - Wrap it in a Rust OcrBackend trait object
// - Register with the Rust core's OcrBackendRegistry
//
// Example future API:
// ```python
// from kreuzberg import register_ocr_backend
//
// class MyOcrBackend:
//     def extract_text(self, image_bytes: bytes, language: str) -> str:
//         # Implementation
//         pass
//
// register_ocr_backend("my_backend", MyOcrBackend())
// ```

/// Register a Python OCR backend (stub).
///
/// **Status**: Not yet implemented. Will be added in Phase 4B.
///
/// Args:
///     name: Backend name
///     backend: Python object implementing OCR interface
///
/// Raises:
///     RuntimeError: Always (not yet implemented)
///
/// Example:
///     >>> from kreuzberg import register_ocr_backend
///     >>> # register_ocr_backend("easyocr", MyEasyOCR())  # Not yet implemented
#[pyfunction]
#[allow(unused_variables)]
pub fn register_ocr_backend(name: String, backend: Py<PyAny>) -> PyResult<()> {
    Err(pyo3::exceptions::PyRuntimeError::new_err(
        "Python OCR backend registration not yet implemented. \
         Use native Tesseract backend for now. \
         This will be available in v4.1.",
    ))
}

/// List registered OCR backends.
///
/// Returns:
///     List of backend names
///
/// Example:
///     >>> from kreuzberg import list_ocr_backends
///     >>> backends = list_ocr_backends()
///     >>> print(backends)
///     ['tesseract']
#[pyfunction]
pub fn list_ocr_backends(py: Python) -> PyResult<Py<pyo3::types::PyList>> {
    // For now, just return the built-in Tesseract backend
    let list = pyo3::types::PyList::empty(py);
    list.append("tesseract")?;
    Ok(list.unbind())
}

/// Unregister an OCR backend (stub).
///
/// **Status**: Not yet implemented. Will be added in Phase 4B.
///
/// Args:
///     name: Backend name to unregister
///
/// Raises:
///     RuntimeError: Always (not yet implemented)
#[pyfunction]
#[allow(unused_variables)]
pub fn unregister_ocr_backend(name: String) -> PyResult<()> {
    Err(pyo3::exceptions::PyRuntimeError::new_err(
        "Python OCR backend unregistration not yet implemented. \
         This will be available in v4.1.",
    ))
}
