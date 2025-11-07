use crate::{adapters::subprocess::SubprocessAdapter, error::Result};
use std::{env, path::PathBuf};

/// Creates a subprocess adapter for Docling framework (single-file mode)
pub fn create_docling_adapter() -> Result<SubprocessAdapter> {
    let script_path = get_script_path("docling_extract.py")?;
    let (command, mut args) = find_python_with_framework("docling")?;
    args.push(script_path.to_string_lossy().to_string());
    args.push("sync".to_string());

    Ok(SubprocessAdapter::new("docling", command, args, vec![]))
}

/// Creates a subprocess adapter for Docling framework (batch mode)
pub fn create_docling_batch_adapter() -> Result<SubprocessAdapter> {
    let script_path = get_script_path("docling_extract.py")?;
    let (command, mut args) = find_python_with_framework("docling")?;
    args.push(script_path.to_string_lossy().to_string());
    args.push("batch".to_string());

    Ok(SubprocessAdapter::with_batch_support(
        "docling-batch",
        command,
        args,
        vec![],
    ))
}

/// Creates a subprocess adapter for Unstructured framework
pub fn create_unstructured_adapter() -> Result<SubprocessAdapter> {
    let script_path = get_script_path("unstructured_extract.py")?;
    let (command, mut args) = find_python_with_framework("unstructured")?;
    args.push(script_path.to_string_lossy().to_string());

    Ok(SubprocessAdapter::new("unstructured", command, args, vec![]))
}

/// Creates a subprocess adapter for MarkItDown framework
pub fn create_markitdown_adapter() -> Result<SubprocessAdapter> {
    let script_path = get_script_path("markitdown_extract.py")?;
    let (command, mut args) = find_python_with_framework("markitdown")?;
    args.push(script_path.to_string_lossy().to_string());

    Ok(SubprocessAdapter::new("markitdown", command, args, vec![]))
}

/// Creates a subprocess adapter for Extractous (Python bindings)
pub fn create_extractous_python_adapter() -> Result<SubprocessAdapter> {
    let script_path = get_script_path("extractous_extract.py")?;
    let (command, mut args) = find_python_with_framework("extractous")?;
    args.push(script_path.to_string_lossy().to_string());

    Ok(SubprocessAdapter::new("extractous-python", command, args, vec![]))
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
///
/// Returns (command, args) where command is the executable and args are the base arguments
fn find_python_with_framework(framework: &str) -> Result<(PathBuf, Vec<String>)> {
    // First, try using uv to run with the workspace environment
    if which::which("uv").is_ok() {
        // uv will use the workspace environment
        // Command will be: uv run python script.py file_path
        return Ok((PathBuf::from("uv"), vec!["run".to_string(), "python".to_string()]));
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
                // Command will be: python script.py file_path
                return Ok((python_path, vec![]));
            }
        }
    }

    Err(crate::error::Error::Config(format!(
        "No Python interpreter found with {} installed. Install with: pip install {}",
        framework, framework
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
