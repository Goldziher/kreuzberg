//! Server bindings for Python (MCP and API).
//!
//! Exposes the Rust MCP and API servers to Python.

use pyo3::prelude::*;

/// Start the Kreuzberg MCP server.
///
/// This function starts the MCP server using stdio transport.
/// It will block until the server is shut down.
///
/// # Errors
///
/// Returns an error if the server fails to start.
#[pyfunction]
pub fn start_mcp_server(py: Python<'_>) -> PyResult<()> {
    py.detach(|| {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create Tokio runtime: {}", e)))?;

        rt.block_on(async {
            kreuzberg::mcp::start_mcp_server()
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("MCP server error: {}", e)))
        })
    })
}

/// Start the Kreuzberg API server.
///
/// This function starts the HTTP API server.
/// It will block until the server is shut down.
///
/// # Arguments
///
/// * `host` - IP address to bind to (e.g., "127.0.0.1" or "0.0.0.0")
/// * `port` - Port number to bind to (e.g., 8000)
///
/// # Errors
///
/// Returns an error if the server fails to start.
#[pyfunction]
pub fn start_api_server(py: Python<'_>, host: String, port: u16) -> PyResult<()> {
    py.detach(|| {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create Tokio runtime: {}", e)))?;

        rt.block_on(async {
            kreuzberg::api::serve(&host, port)
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("API server error: {}", e)))
        })
    })
}
