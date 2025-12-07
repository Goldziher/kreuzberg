use crate::{adapters::subprocess::SubprocessAdapter, error::Result};
use std::{env, path::PathBuf};

/// Creates a subprocess adapter for Docling (open source extraction framework, single-file mode)
pub fn create_docling_adapter() -> Result<SubprocessAdapter> {
    let script_path = get_script_path("docling_extract.py")?;
    let (command, mut args) = find_python_with_framework("docling")?;
    args.push(script_path.to_string_lossy().to_string());
    args.push("sync".to_string());

    Ok(SubprocessAdapter::new("docling", command, args, vec![]))
}

/// Creates a subprocess adapter for Docling (open source extraction framework, batch mode)
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

/// Creates a subprocess adapter for Unstructured (open source extraction framework)
pub fn create_unstructured_adapter() -> Result<SubprocessAdapter> {
    let script_path = get_script_path("unstructured_extract.py")?;
    let (command, mut args) = find_python_with_framework("unstructured")?;
    args.push(script_path.to_string_lossy().to_string());

    Ok(SubprocessAdapter::new("unstructured", command, args, vec![]))
}

/// Creates a subprocess adapter for MarkItDown (open source extraction framework)
pub fn create_markitdown_adapter() -> Result<SubprocessAdapter> {
    let script_path = get_script_path("markitdown_extract.py")?;
    let (command, mut args) = find_python_with_framework("markitdown")?;
    args.push(script_path.to_string_lossy().to_string());

    Ok(SubprocessAdapter::new("markitdown", command, args, vec![]))
}

/// Helper function to get the path to a wrapper script
fn get_script_path(script_name: &str) -> Result<PathBuf> {
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let script_path = PathBuf::from(manifest_dir).join("scripts").join(script_name);
        if script_path.exists() {
            return Ok(script_path);
        }
    }

    let script_path = PathBuf::from("tools/benchmark-harness/scripts").join(script_name);
    if script_path.exists() {
        return Ok(script_path);
    }

    Err(crate::error::Error::Config(format!(
        "Script not found: {}",
        script_name
    )))
}

/// Helper function to find Python interpreter with a specific open source extraction framework installed
///
/// Returns (command, args) where command is the executable and args are the base arguments
fn find_python_with_framework(framework: &str) -> Result<(PathBuf, Vec<String>)> {
    if which::which("uv").is_ok() {
        return Ok((PathBuf::from("uv"), vec!["run".to_string(), "python".to_string()]));
    }

    let python_candidates = vec!["python3", "python"];

    for candidate in python_candidates {
        if let Ok(python_path) = which::which(candidate) {
            let check = std::process::Command::new(&python_path)
                .arg("-c")
                .arg(format!("import {}", framework))
                .output();

            if let Ok(output) = check
                && output.status.success()
            {
                return Ok((python_path, vec![]));
            }
        }
    }

    Err(crate::error::Error::Config(format!(
        "No Python interpreter found with {} installed. Install with: pip install {}",
        framework, framework
    )))
}

/// Helper to find Java runtime
fn find_java() -> Result<PathBuf> {
    which::which("java").map_err(|_| crate::Error::Config("Java runtime not found".to_string()))
}

/// Helper to locate Tika JAR (auto-detect from libs/ or env var)
fn get_tika_jar_path() -> Result<PathBuf> {
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let lib_dir = PathBuf::from(manifest_dir).join("libs");
        if let Ok(entries) = std::fs::read_dir(&lib_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str())
                    && name.starts_with("tika-app-")
                    && name.ends_with(".jar")
                {
                    return Ok(path);
                }
            }
        }
    }

    let fallback_lib_dir = PathBuf::from("tools/benchmark-harness/libs");
    if let Ok(entries) = std::fs::read_dir(&fallback_lib_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str())
                && name.starts_with("tika-app-")
                && name.ends_with(".jar")
            {
                return Ok(path);
            }
        }
    }

    if let Ok(jar_path) = env::var("TIKA_JAR") {
        let path = PathBuf::from(jar_path);
        if path.exists() {
            return Ok(path);
        }
    }

    Err(crate::Error::Config(
        "Tika JAR not found. Download: curl -LO https://repo1.maven.org/maven2/org/apache/tika/tika-app/2.9.2/tika-app-2.9.2.jar && mv tika-app-2.9.2.jar tools/benchmark-harness/libs/".to_string()
    ))
}

/// Creates a subprocess adapter for Apache Tika (single-file mode)
pub fn create_tika_sync_adapter() -> Result<SubprocessAdapter> {
    let jar_path = get_tika_jar_path()?;
    let script_path = get_script_path("TikaExtract.java")?;
    let command = find_java()?;

    let args = vec![
        "-cp".to_string(),
        jar_path.to_string_lossy().to_string(),
        script_path.to_string_lossy().to_string(),
        "sync".to_string(),
    ];

    Ok(SubprocessAdapter::new("tika-sync", command, args, vec![]))
}

/// Creates a subprocess adapter for Apache Tika (batch mode)
pub fn create_tika_batch_adapter() -> Result<SubprocessAdapter> {
    let jar_path = get_tika_jar_path()?;
    let script_path = get_script_path("TikaExtract.java")?;
    let command = find_java()?;

    let args = vec![
        "-cp".to_string(),
        jar_path.to_string_lossy().to_string(),
        script_path.to_string_lossy().to_string(),
        "batch".to_string(),
    ];

    Ok(SubprocessAdapter::with_batch_support(
        "tika-batch",
        command,
        args,
        vec![],
    ))
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
        let _ = create_docling_adapter();
        let _ = create_unstructured_adapter();
        let _ = create_markitdown_adapter();
        let _ = create_tika_sync_adapter();
        let _ = create_tika_batch_adapter();
    }
}
