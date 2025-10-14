use crate::error::to_py_err;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

/// Extract content and metadata from a file using Pandoc
/// Returns MessagePack-encoded PandocExtractionResult
#[pyfunction]
pub fn extract_with_pandoc_msgpack<'py>(
    py: Python<'py>,
    file_path: String,
    from_format: String,
) -> PyResult<Bound<'py, PyBytes>> {
    let result = py.detach(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let path = std::path::Path::new(&file_path);
                kreuzberg::extraction::pandoc::extract_file(path, &from_format).await
            })
    });

    let extraction_result = result.map_err(to_py_err)?;

    // Serialize to MessagePack
    let msgpack_bytes = rmp_serde::to_vec_named(&extraction_result).map_err(|e| to_py_err(e.into()))?;

    Ok(PyBytes::new(py, &msgpack_bytes))
}

/// Extract content and metadata from bytes using Pandoc
/// Returns MessagePack-encoded PandocExtractionResult
#[pyfunction]
pub fn extract_with_pandoc_from_bytes_msgpack<'py>(
    py: Python<'py>,
    file_bytes: &[u8],
    from_format: String,
    extension: String,
) -> PyResult<Bound<'py, PyBytes>> {
    let bytes = file_bytes.to_vec();

    let result = py.detach(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async { kreuzberg::extraction::pandoc::extract_bytes(&bytes, &from_format, &extension).await })
    });

    let extraction_result = result.map_err(to_py_err)?;

    // Serialize to MessagePack
    let msgpack_bytes = rmp_serde::to_vec_named(&extraction_result).map_err(|e| to_py_err(e.into()))?;

    Ok(PyBytes::new(py, &msgpack_bytes))
}

/// Validate that Pandoc is installed and has the correct version
#[pyfunction]
pub fn validate_pandoc_version(py: Python<'_>) -> PyResult<()> {
    py.detach(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async { kreuzberg::extraction::pandoc::validate_pandoc_version().await })
    })
    .map_err(to_py_err)
}
