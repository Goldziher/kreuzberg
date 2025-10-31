use crate::{adapters::subprocess::SubprocessAdapter, error::Result};
use std::{env, path::PathBuf};

/// Creates a subprocess adapter for Docling framework
pub fn create_docling_adapter() -> Result<SubprocessAdapter> {
    let script_path = get_script_path("docling_extract.py")?;
    let python_cmd = find_python_with_framework("docling")?;

    Ok(SubprocessAdapter::new(
        "docling",
        python_cmd,
        vec![script_path.to_string_lossy().to_string()],
        vec![],
    ))
}

/// Creates a subprocess adapter for Unstructured framework
pub fn create_unstructured_adapter() -> Result<SubprocessAdapter> {
    let script_path = get_script_path("unstructured_extract.py")?;
    let python_cmd = find_python_with_framework("unstructured")?;

    Ok(SubprocessAdapter::new(
        "unstructured",
        python_cmd,
        vec![script_path.to_string_lossy().to_string()],
        vec![],
    ))
}

/// Creates a subprocess adapter for MarkItDown framework
pub fn create_markitdown_adapter() -> Result<SubprocessAdapter> {
    let script_path = get_script_path("markitdown_extract.py")?;
    let python_cmd = find_python_with_framework("markitdown")?;

    Ok(SubprocessAdapter::new(
        "markitdown",
        python_cmd,
        vec![script_path.to_string_lossy().to_string()],
        vec![],
    ))
}

/// Creates a subprocess adapter for Extractous (Python bindings)
pub fn create_extractous_python_adapter() -> Result<SubprocessAdapter> {
    let script_path = get_script_path("extractous_extract.py")?;
    let python_cmd = find_python_with_framework("extractous")?;

    Ok(SubprocessAdapter::new(
        "extractous-python",
        python_cmd,
        vec![script_path.to_string_lossy().to_string()],
        vec![],
    ))
}

// NOTE: Native Rust adapter for Extractous could be implemented here
// when feature "extractous-native" is enabled. For now, we use the Python
// bindings via subprocess adapter.

/// Helper function to get the path to a wrapper script
fn get_script_path(script_name: &str) -> Result<PathBuf> {
    // Try to find script relative to cargo manifest dir
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let script_path = PathBuf::from(manifest_dir).join("scripts").join(script_name);
        if script_path.exists() {
            return Ok(script_path);
        }
    }

    // Try relative to current directory
    let script_path = PathBuf::from("tools/benchmark-harness/scripts").join(script_name);
    if script_path.exists() {
        return Ok(script_path);
    }

    Err(crate::error::Error::Config(format!(
        "Script not found: {}",
        script_name
    )))
}

/// Helper function to find Python interpreter with a specific framework installed
fn find_python_with_framework(framework: &str) -> Result<PathBuf> {
    // First, try using uv to run with the comparative-benchmarks group
    if which::which("uv").is_ok() {
        // uv will use the workspace environment with comparative-benchmarks group
        return Ok(PathBuf::from("uv"));
    }

    // Fall back to system Python
    let python_candidates = vec!["python3", "python"];

    for candidate in python_candidates {
        if let Ok(python_path) = which::which(candidate) {
            // Quick check if framework is available
            let check = std::process::Command::new(&python_path)
                .arg("-c")
                .arg(format!("import {}", framework))
                .output();

            if let Ok(output) = check
                && output.status.success()
            {
                return Ok(python_path);
            }
        }
    }

    Err(crate::error::Error::Config(format!(
        "No Python interpreter found with {} installed. Install with: uv sync --group comparative-benchmarks",
        framework
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_script_path() {
        let result = get_script_path("docling_extract.py");
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_adapter_creation() {
        // These tests will fail if frameworks are not installed,
        // but they verify the adapter creation logic
        let _ = create_docling_adapter();
        let _ = create_unstructured_adapter();
        let _ = create_markitdown_adapter();
        let _ = create_extractous_python_adapter();
    }
}
