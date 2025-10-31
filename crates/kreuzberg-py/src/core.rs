//! Core extraction functions
//!
//! Provides both synchronous and asynchronous extraction functions for Python.

use crate::config::ExtractionConfig;
use crate::error::to_py_err;
use crate::types::ExtractionResult;
use pyo3::prelude::*;
use pyo3::types::PyList;

/// Extract a path string from Python input (str, pathlib.Path, or bytes).
///
/// Supports:
/// - `str`: Direct string paths
/// - `pathlib.Path`: Extracts via `__fspath__()` protocol
/// - `bytes`: UTF-8 decoded path bytes (Unix paths)
fn extract_path_string(path: &Bound<'_, PyAny>) -> PyResult<String> {
    if let Ok(s) = path.extract::<String>() {
        return Ok(s);
    }

    if let Ok(fspath) = path.call_method0("__fspath__")
        && let Ok(s) = fspath.extract::<String>()
    {
        return Ok(s);
    }

    if let Ok(b) = path.extract::<Vec<u8>>() {
        if let Ok(s) = String::from_utf8(b) {
            return Ok(s);
        }
        return Err(pyo3::exceptions::PyValueError::new_err(
            "Path bytes must be valid UTF-8",
        ));
    }

    Err(pyo3::exceptions::PyTypeError::new_err(
        "Path must be a string, pathlib.Path, or bytes",
    ))
}

/// Extract content from a file (synchronous).
///
/// Args:
///     path: Path to the file to extract (str or pathlib.Path)
///     mime_type: Optional MIME type hint (auto-detected if None)
///     config: Extraction configuration
///
/// Returns:
///     ExtractionResult with content, metadata, and tables
///
/// Raises:
///     ValueError: Invalid configuration or unsupported format
///     IOError: File access errors
///     RuntimeError: Extraction failures
///
/// Example:
///     >>> from kreuzberg import extract_file_sync, ExtractionConfig
///     >>> result = extract_file_sync("document.pdf", None, ExtractionConfig())
///     >>> print(result.content)
///     >>> # Also works with pathlib.Path
///     >>> from pathlib import Path
///     >>> result = extract_file_sync(Path("document.pdf"), None, ExtractionConfig())
#[pyfunction]
#[pyo3(signature = (path, mime_type=None, config=ExtractionConfig::default()))]
pub fn extract_file_sync(
    py: Python,
    path: &Bound<'_, PyAny>,
    mime_type: Option<String>,
    config: ExtractionConfig,
) -> PyResult<ExtractionResult> {
    let path_str = extract_path_string(path)?;
    let rust_config = config.into();
    let result = kreuzberg::extract_file_sync(&path_str, mime_type.as_deref(), &rust_config).map_err(to_py_err)?;
    ExtractionResult::from_rust(result, py)
}

/// Extract content from bytes (synchronous).
///
/// Args:
///     data: Bytes to extract (bytes or bytearray)
///     mime_type: MIME type of the data
///     config: Extraction configuration
///
/// Returns:
///     ExtractionResult with content, metadata, and tables
///
/// Raises:
///     ValueError: Invalid configuration or unsupported format
///     RuntimeError: Extraction failures
///
/// Example:
///     >>> from kreuzberg import extract_bytes_sync, ExtractionConfig
///     >>> with open("document.pdf", "rb") as f:
///     ...     data = f.read()
///     >>> result = extract_bytes_sync(data, "application/pdf", ExtractionConfig())
///     >>> print(result.content)
#[pyfunction]
#[pyo3(signature = (data, mime_type, config=ExtractionConfig::default()))]
pub fn extract_bytes_sync(
    py: Python,
    data: Vec<u8>,
    mime_type: String,
    config: ExtractionConfig,
) -> PyResult<ExtractionResult> {
    let rust_config = config.into();
    let result = kreuzberg::extract_bytes_sync(&data, &mime_type, &rust_config).map_err(to_py_err)?;
    ExtractionResult::from_rust(result, py)
}

/// Batch extract content from multiple files (synchronous).
///
/// MIME types are auto-detected for each file.
///
/// Args:
///     paths: List of file paths to extract (str, pathlib.Path, or bytes)
///     config: Extraction configuration
///
/// Returns:
///     List of ExtractionResult objects (one per file)
///
/// Raises:
///     ValueError: Invalid configuration
///     IOError: File access errors
///     RuntimeError: Extraction failures
///
/// Example:
///     >>> from kreuzberg import batch_extract_files_sync, ExtractionConfig
///     >>> paths = ["doc1.pdf", "doc2.docx"]
///     >>> results = batch_extract_files_sync(paths, ExtractionConfig())
///     >>> for result in results:
///     ...     print(result.content)
///     >>> # Also works with pathlib.Path
///     >>> from pathlib import Path
///     >>> paths = [Path("doc1.pdf"), Path("doc2.docx")]
///     >>> results = batch_extract_files_sync(paths, ExtractionConfig())
#[pyfunction]
#[pyo3(signature = (paths, config=ExtractionConfig::default()))]
pub fn batch_extract_files_sync(
    py: Python,
    paths: &Bound<'_, PyList>,
    config: ExtractionConfig,
) -> PyResult<Py<PyList>> {
    let path_strings: PyResult<Vec<String>> = paths.iter().map(|p| extract_path_string(&p)).collect();
    let path_strings = path_strings?;

    let rust_config = config.into();
    let results = kreuzberg::batch_extract_file_sync(path_strings, &rust_config).map_err(to_py_err)?;

    let list = PyList::empty(py);
    for result in results {
        list.append(ExtractionResult::from_rust(result, py)?)?;
    }
    Ok(list.unbind())
}

/// Batch extract content from multiple byte arrays (synchronous).
///
/// Args:
///     data_list: List of bytes objects to extract
///     mime_types: List of MIME types (one per data object)
///     config: Extraction configuration
///
/// Returns:
///     List of ExtractionResult objects (one per data object)
///
/// Raises:
///     ValueError: Invalid configuration or list length mismatch
///     RuntimeError: Extraction failures
///
/// Example:
///     >>> from kreuzberg import batch_extract_bytes_sync, ExtractionConfig
///     >>> data_list = [open("doc1.pdf", "rb").read(), open("doc2.pdf", "rb").read()]
///     >>> mime_types = ["application/pdf", "application/pdf"]
///     >>> results = batch_extract_bytes_sync(data_list, mime_types, ExtractionConfig())
#[pyfunction]
#[pyo3(signature = (data_list, mime_types, config=ExtractionConfig::default()))]
pub fn batch_extract_bytes_sync(
    py: Python,
    data_list: Vec<Vec<u8>>,
    mime_types: Vec<String>,
    config: ExtractionConfig,
) -> PyResult<Py<PyList>> {
    if data_list.len() != mime_types.len() {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "data_list and mime_types must have the same length (got {} and {})",
            data_list.len(),
            mime_types.len()
        )));
    }

    let rust_config = config.into();

    let contents: Vec<(&[u8], &str)> = data_list
        .iter()
        .zip(mime_types.iter())
        .map(|(data, mime)| (data.as_slice(), mime.as_str()))
        .collect();

    let results = kreuzberg::batch_extract_bytes_sync(contents, &rust_config).map_err(to_py_err)?;

    let list = PyList::empty(py);
    for result in results {
        list.append(ExtractionResult::from_rust(result, py)?)?;
    }
    Ok(list.unbind())
}

/// Extract content from a file (asynchronous).
///
/// Args:
///     path: Path to the file to extract (str or pathlib.Path)
///     mime_type: Optional MIME type hint (auto-detected if None)
///     config: Extraction configuration
///
/// Returns:
///     ExtractionResult with content, metadata, and tables
///
/// Raises:
///     ValueError: Invalid configuration or unsupported format
///     IOError: File access errors
///     RuntimeError: Extraction failures
///
/// Example:
///     >>> import asyncio
///     >>> from kreuzberg import extract_file, ExtractionConfig
///     >>> async def main():
///     ...     result = await extract_file("document.pdf", None, ExtractionConfig())
///     ...     print(result.content)
///     >>> asyncio.run(main())
///     >>> # Also works with pathlib.Path
///     >>> from pathlib import Path
///     >>> async def main():
///     ...     result = await extract_file(Path("document.pdf"))
#[pyfunction]
#[pyo3(signature = (path, mime_type=None, config=ExtractionConfig::default()))]
pub fn extract_file<'py>(
    py: Python<'py>,
    path: &Bound<'py, PyAny>,
    mime_type: Option<String>,
    config: ExtractionConfig,
) -> PyResult<Bound<'py, PyAny>> {
    let path_str = extract_path_string(path)?;
    let rust_config: kreuzberg::ExtractionConfig = config.into();
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        let result = kreuzberg::extract_file(&path_str, mime_type.as_deref(), &rust_config)
            .await
            .map_err(to_py_err)?;
        Python::attach(|py| ExtractionResult::from_rust(result, py))
    })
}

/// Extract content from bytes (asynchronous).
///
/// Args:
///     data: Bytes to extract (bytes or bytearray)
///     mime_type: MIME type of the data
///     config: Extraction configuration
///
/// Returns:
///     ExtractionResult with content, metadata, and tables
///
/// Raises:
///     ValueError: Invalid configuration or unsupported format
///     RuntimeError: Extraction failures
///
/// Example:
///     >>> import asyncio
///     >>> from kreuzberg import extract_bytes, ExtractionConfig
///     >>> async def main():
///     ...     with open("document.pdf", "rb") as f:
///     ...         data = f.read()
///     ...     result = await extract_bytes(data, "application/pdf", ExtractionConfig())
///     ...     print(result.content)
///     >>> asyncio.run(main())
#[pyfunction]
#[pyo3(signature = (data, mime_type, config=ExtractionConfig::default()))]
pub fn extract_bytes<'py>(
    py: Python<'py>,
    data: Vec<u8>,
    mime_type: String,
    config: ExtractionConfig,
) -> PyResult<Bound<'py, PyAny>> {
    let rust_config: kreuzberg::ExtractionConfig = config.into();
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        let result = kreuzberg::extract_bytes(&data, &mime_type, &rust_config)
            .await
            .map_err(to_py_err)?;
        Python::attach(|py| ExtractionResult::from_rust(result, py))
    })
}

/// Batch extract content from multiple files (asynchronous).
///
/// MIME types are auto-detected for each file.
///
/// Args:
///     paths: List of file paths to extract (str, pathlib.Path, or bytes)
///     config: Extraction configuration
///
/// Returns:
///     List of ExtractionResult objects (one per file)
///
/// Raises:
///     ValueError: Invalid configuration
///     IOError: File access errors
///     RuntimeError: Extraction failures
///
/// Example:
///     >>> import asyncio
///     >>> from kreuzberg import batch_extract_files, ExtractionConfig
///     >>> async def main():
///     ...     paths = ["doc1.pdf", "doc2.docx"]
///     ...     results = await batch_extract_files(paths, ExtractionConfig())
///     ...     for result in results:
///     ...         print(result.content)
///     >>> asyncio.run(main())
///     >>> # Also works with pathlib.Path
///     >>> from pathlib import Path
///     >>> async def main():
///     ...     paths = [Path("doc1.pdf"), Path("doc2.docx")]
///     ...     results = await batch_extract_files(paths, ExtractionConfig())
#[pyfunction]
#[pyo3(signature = (paths, config=ExtractionConfig::default()))]
pub fn batch_extract_files<'py>(
    py: Python<'py>,
    paths: &Bound<'py, PyList>,
    config: ExtractionConfig,
) -> PyResult<Bound<'py, PyAny>> {
    let path_strings: PyResult<Vec<String>> = paths.iter().map(|p| extract_path_string(&p)).collect();
    let path_strings = path_strings?;

    let rust_config: kreuzberg::ExtractionConfig = config.into();
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        let results = kreuzberg::batch_extract_file(path_strings, &rust_config)
            .await
            .map_err(to_py_err)?;

        Python::attach(|py| {
            let list = PyList::empty(py);
            for result in results {
                list.append(ExtractionResult::from_rust(result, py)?)?;
            }
            Ok(list.unbind())
        })
    })
}

/// Batch extract content from multiple byte arrays (asynchronous).
///
/// Args:
///     data_list: List of bytes objects to extract
///     mime_types: List of MIME types (one per data object)
///     config: Extraction configuration
///
/// Returns:
///     List of ExtractionResult objects (one per data object)
///
/// Raises:
///     ValueError: Invalid configuration or list length mismatch
///     RuntimeError: Extraction failures
///
/// Example:
///     >>> import asyncio
///     >>> from kreuzberg import batch_extract_bytes, ExtractionConfig
///     >>> async def main():
///     ...     data_list = [open("doc1.pdf", "rb").read(), open("doc2.pdf", "rb").read()]
///     ...     mime_types = ["application/pdf", "application/pdf"]
///     ...     results = await batch_extract_bytes(data_list, mime_types, ExtractionConfig())
///     >>> asyncio.run(main())
#[pyfunction]
#[pyo3(signature = (data_list, mime_types, config=ExtractionConfig::default()))]
pub fn batch_extract_bytes<'py>(
    py: Python<'py>,
    data_list: Vec<Vec<u8>>,
    mime_types: Vec<String>,
    config: ExtractionConfig,
) -> PyResult<Bound<'py, PyAny>> {
    if data_list.len() != mime_types.len() {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "data_list and mime_types must have the same length (got {} and {})",
            data_list.len(),
            mime_types.len()
        )));
    }

    let rust_config: kreuzberg::ExtractionConfig = config.into();
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        let contents: Vec<(&[u8], &str)> = data_list
            .iter()
            .zip(mime_types.iter())
            .map(|(data, mime)| (data.as_slice(), mime.as_str()))
            .collect();

        let results = kreuzberg::batch_extract_bytes(contents, &rust_config)
            .await
            .map_err(to_py_err)?;

        Python::attach(|py| {
            let list = PyList::empty(py);
            for result in results {
                list.append(ExtractionResult::from_rust(result, py)?)?;
            }
            Ok(list.unbind())
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::types::{PyBytes, PyString};
    use std::sync::Once;

    fn prepare_python() {
        static INIT: Once = Once::new();
        INIT.call_once(|| Python::initialize());
    }

    fn with_py<F, R>(f: F) -> R
    where
        F: FnOnce(Python<'_>) -> R,
    {
        prepare_python();
        Python::attach(f)
    }

    #[test]
    fn test_extract_path_string_from_str() {
        with_py(|py| {
            let value = PyString::new(py, "document.txt");
            let result = extract_path_string(&value.into_any()).expect("string path should extract");
            assert_eq!(result, "document.txt");
        });
    }

    #[test]
    fn test_extract_path_string_from_pathlib_path() {
        with_py(|py| -> PyResult<()> {
            let pathlib = py.import("pathlib")?;
            let path_obj = pathlib.getattr("Path")?.call1(("nested/file.md",))?;
            let extracted = extract_path_string(&path_obj)?;
            assert!(
                extracted.ends_with("nested/file.md"),
                "expected path to end with nested/file.md, got {extracted}"
            );
            Ok(())
        })
        .expect("pathlib.Path extraction should succeed");
    }

    #[test]
    fn test_extract_path_string_from_bytes() {
        with_py(|py| {
            let value = PyBytes::new(py, b"ascii.bin");
            let result = extract_path_string(&value.into_any()).expect("bytes path should extract");
            assert_eq!(result, "ascii.bin");
        });
    }

    #[test]
    fn test_extract_path_string_invalid_type() {
        with_py(|py| {
            let value = py
                .eval(pyo3::ffi::c_str!("42"), None, None)
                .expect("should evaluate literal");
            let err = extract_path_string(&value).expect_err("non-path type should fail");
            assert!(err.is_instance_of::<pyo3::exceptions::PyTypeError>(py));
        });
    }

    #[test]
    fn test_extract_bytes_sync_returns_content() {
        with_py(|py| {
            let data = b"hello kreuzberg".to_vec();
            let result = extract_bytes_sync(py, data, "text/plain".to_string(), ExtractionConfig::default())
                .expect("text/plain extraction should succeed");
            assert_eq!(result.mime_type, "text/plain");
            assert!(result.content.contains("hello"));
        });
    }

    #[test]
    fn test_batch_extract_bytes_sync_length_mismatch() {
        with_py(|py| {
            let err = batch_extract_bytes_sync(
                py,
                vec![b"a".to_vec(), b"b".to_vec()],
                vec!["text/plain".to_string()],
                ExtractionConfig::default(),
            )
            .expect_err("length mismatch should error");
            assert!(err.is_instance_of::<pyo3::exceptions::PyValueError>(py));
        });
    }

    #[test]
    fn test_batch_extract_bytes_sync_returns_list() {
        with_py(|py| {
            let data = vec![b"first".to_vec(), b"second".to_vec()];
            let mimes = vec!["text/plain".to_string(), "text/plain".to_string()];
            let list = batch_extract_bytes_sync(py, data, mimes, ExtractionConfig::default())
                .expect("batch extraction should succeed");
            assert_eq!(list.bind(py).len(), 2);
        });
    }
}
