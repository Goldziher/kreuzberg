//! Kreuzberg language binding adapters
//!
//! Factory functions for creating adapters for different language bindings and modes:
//! - Python: sync, async, batch
//! - TypeScript/Node: async, batch
//! - Ruby: sync, batch

use crate::Result;
use crate::adapters::subprocess::SubprocessAdapter;
use std::env;
use std::path::PathBuf;

/// Get the path to a script in the scripts directory
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

    Err(crate::Error::Config(format!("Script not found: {}", script_name)))
}

/// Helper to find Python interpreter (prefers uv)
fn find_python() -> Result<(PathBuf, Vec<String>)> {
    if which::which("uv").is_ok() {
        Ok((PathBuf::from("uv"), vec!["run".to_string(), "python".to_string()]))
    } else if which::which("python3").is_ok() {
        Ok((PathBuf::from("python3"), vec![]))
    } else {
        Err(crate::Error::Config("Python not found".to_string()))
    }
}

/// Helper to find Node/TypeScript interpreter (tsx)
fn find_node() -> Result<(PathBuf, Vec<String>)> {
    if which::which("tsx").is_ok() {
        return Ok((PathBuf::from("tsx"), vec![]));
    }

    if which::which("ts-node").is_ok() {
        return Ok((PathBuf::from("ts-node"), vec![]));
    }

    Err(crate::Error::Config(
        "TypeScript runtime (tsx or ts-node) not found".to_string(),
    ))
}

/// Helper to find Ruby interpreter
fn find_ruby() -> Result<(PathBuf, Vec<String>)> {
    if which::which("ruby").is_ok() {
        Ok((PathBuf::from("ruby"), vec![]))
    } else {
        Err(crate::Error::Config("Ruby not found".to_string()))
    }
}

/// Create Python sync adapter (extract_file)
pub fn create_python_sync_adapter() -> Result<SubprocessAdapter> {
    let script_path = get_script_path("kreuzberg_extract.py")?;
    let (command, mut args) = find_python()?;

    args.push(script_path.to_string_lossy().to_string());
    args.push("sync".to_string());

    Ok(SubprocessAdapter::new("kreuzberg-python-sync", command, args, vec![]))
}

/// Create Python async adapter (extract_file_async)
pub fn create_python_async_adapter() -> Result<SubprocessAdapter> {
    let script_path = get_script_path("kreuzberg_extract.py")?;
    let (command, mut args) = find_python()?;

    args.push(script_path.to_string_lossy().to_string());
    args.push("async".to_string());

    Ok(SubprocessAdapter::new("kreuzberg-python-async", command, args, vec![]))
}

/// Create Python batch adapter (batch_extract_file)
pub fn create_python_batch_adapter() -> Result<SubprocessAdapter> {
    let script_path = get_script_path("kreuzberg_extract.py")?;
    let (command, mut args) = find_python()?;

    args.push(script_path.to_string_lossy().to_string());
    args.push("batch".to_string());

    Ok(SubprocessAdapter::with_batch_support(
        "kreuzberg-python-batch",
        command,
        args,
        vec![],
    ))
}

/// Create Node async adapter (extractFile)
pub fn create_node_async_adapter() -> Result<SubprocessAdapter> {
    let script_path = get_script_path("kreuzberg_extract.ts")?;
    let (command, mut args) = find_node()?;

    args.push(script_path.to_string_lossy().to_string());
    args.push("async".to_string());

    Ok(SubprocessAdapter::new("kreuzberg-node-async", command, args, vec![]))
}

/// Create Node batch adapter (batchExtractFile)
pub fn create_node_batch_adapter() -> Result<SubprocessAdapter> {
    let script_path = get_script_path("kreuzberg_extract.ts")?;
    let (command, mut args) = find_node()?;

    args.push(script_path.to_string_lossy().to_string());
    args.push("batch".to_string());

    Ok(SubprocessAdapter::with_batch_support(
        "kreuzberg-node-batch",
        command,
        args,
        vec![],
    ))
}

/// Create Ruby sync adapter (extract_file)
pub fn create_ruby_sync_adapter() -> Result<SubprocessAdapter> {
    let script_path = get_script_path("kreuzberg_extract.rb")?;
    let (command, mut args) = find_ruby()?;

    args.push(script_path.to_string_lossy().to_string());
    args.push("sync".to_string());

    Ok(SubprocessAdapter::new("kreuzberg-ruby-sync", command, args, vec![]))
}

/// Create Ruby batch adapter (batch_extract_file)
pub fn create_ruby_batch_adapter() -> Result<SubprocessAdapter> {
    let script_path = get_script_path("kreuzberg_extract.rb")?;
    let (command, mut args) = find_ruby()?;

    args.push(script_path.to_string_lossy().to_string());
    args.push("batch".to_string());

    Ok(SubprocessAdapter::with_batch_support(
        "kreuzberg-ruby-batch",
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
        let result = get_script_path("kreuzberg_extract.py");
        if result.is_ok() {
            assert!(result.unwrap().exists());
        }
    }

    #[test]
    fn test_find_python() {
        let result = find_python();
        assert!(result.is_ok() || which::which("python3").is_err());
    }
}
