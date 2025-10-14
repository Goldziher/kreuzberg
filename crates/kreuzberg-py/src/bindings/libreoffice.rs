use crate::error::to_py_err;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

/// Check if LibreOffice is available
#[pyfunction]
pub fn check_libreoffice_available(py: Python<'_>) -> PyResult<()> {
    py.detach(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async { kreuzberg::extraction::libreoffice::check_libreoffice_available().await })
    })
    .map_err(to_py_err)
}

/// Convert .doc to .docx using LibreOffice
/// Returns MessagePack-encoded LibreOfficeConversionResult
#[pyfunction]
pub fn convert_doc_to_docx_msgpack<'py>(py: Python<'py>, doc_bytes: &[u8]) -> PyResult<Bound<'py, PyBytes>> {
    let bytes = doc_bytes.to_vec();

    let result = py.detach(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async { kreuzberg::extraction::libreoffice::convert_doc_to_docx(&bytes).await })
    });

    let conversion_result = result.map_err(to_py_err)?;

    // Serialize to MessagePack
    let msgpack_bytes = rmp_serde::to_vec_named(&conversion_result).map_err(|e| to_py_err(e.into()))?;

    Ok(PyBytes::new(py, &msgpack_bytes))
}

/// Convert .ppt to .pptx using LibreOffice
/// Returns MessagePack-encoded LibreOfficeConversionResult
#[pyfunction]
pub fn convert_ppt_to_pptx_msgpack<'py>(py: Python<'py>, ppt_bytes: &[u8]) -> PyResult<Bound<'py, PyBytes>> {
    let bytes = ppt_bytes.to_vec();

    let result = py.detach(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async { kreuzberg::extraction::libreoffice::convert_ppt_to_pptx(&bytes).await })
    });

    let conversion_result = result.map_err(to_py_err)?;

    // Serialize to MessagePack
    let msgpack_bytes = rmp_serde::to_vec_named(&conversion_result).map_err(|e| to_py_err(e.into()))?;

    Ok(PyBytes::new(py, &msgpack_bytes))
}
