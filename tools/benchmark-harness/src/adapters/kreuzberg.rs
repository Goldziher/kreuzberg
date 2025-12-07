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

/// Helper to find Go toolchain
fn find_go() -> Result<PathBuf> {
    which::which("go").map_err(|_| crate::Error::Config("Go toolchain not found".to_string()))
}

/// Helper to find Java runtime
fn find_java() -> Result<PathBuf> {
    which::which("java").map_err(|_| crate::Error::Config("Java runtime not found".to_string()))
}

fn find_dotnet() -> Result<PathBuf> {
    which::which("dotnet").map_err(|_| crate::Error::Config("dotnet CLI not found".to_string()))
}

fn workspace_root() -> Result<PathBuf> {
    let manifest_dir =
        env::var("CARGO_MANIFEST_DIR").map_err(|_| crate::Error::Config("CARGO_MANIFEST_DIR not set".to_string()))?;
    let path = PathBuf::from(manifest_dir);
    let root = path
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| crate::Error::Config("Unable to resolve workspace root".to_string()))?;
    Ok(root.to_path_buf())
}

fn native_library_dir() -> Result<PathBuf> {
    let root = workspace_root()?;
    let release = root.join("target/release");
    if release.exists() {
        return Ok(release);
    }
    let debug = root.join("target/debug");
    if debug.exists() {
        return Ok(debug);
    }
    Err(crate::Error::Config(
        "Native library directory not found in target/".to_string(),
    ))
}

fn prepend_env(var: &str, value: &str, separator: &str) -> String {
    match env::var(var) {
        Ok(existing) if !existing.is_empty() => format!("{value}{separator}{existing}"),
        _ => value.to_string(),
    }
}

fn build_library_env() -> Result<Vec<(String, String)>> {
    let lib_dir = native_library_dir()?;
    let lib_str = lib_dir.to_string_lossy().to_string();
    let mut envs = vec![
        (
            "LD_LIBRARY_PATH".to_string(),
            prepend_env("LD_LIBRARY_PATH", &lib_str, ":"),
        ),
        (
            "DYLD_LIBRARY_PATH".to_string(),
            prepend_env("DYLD_LIBRARY_PATH", &lib_str, ":"),
        ),
    ];
    if cfg!(target_os = "windows") {
        envs.push(("PATH".to_string(), prepend_env("PATH", &lib_str, ";")));
    }
    Ok(envs)
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

    let env = build_library_env()?;
    Ok(SubprocessAdapter::new("kreuzberg-ruby-sync", command, args, env))
}

/// Create Ruby batch adapter (batch_extract_file)
pub fn create_ruby_batch_adapter() -> Result<SubprocessAdapter> {
    let script_path = get_script_path("kreuzberg_extract.rb")?;
    let (command, mut args) = find_ruby()?;

    args.push(script_path.to_string_lossy().to_string());
    args.push("batch".to_string());

    let env = build_library_env()?;
    Ok(SubprocessAdapter::with_batch_support(
        "kreuzberg-ruby-batch",
        command,
        args,
        env,
    ))
}

/// Create Go sync adapter
pub fn create_go_sync_adapter() -> Result<SubprocessAdapter> {
    let script_path = get_script_path("kreuzberg_extract_go.go")?;
    let scripts_dir = script_path
        .parent()
        .ok_or_else(|| crate::Error::Config("Unable to determine scripts directory".to_string()))?
        .to_path_buf();
    let command = find_go()?;
    let args = vec![
        "run".to_string(),
        "kreuzberg_extract_go.go".to_string(),
        "sync".to_string(),
    ];
    let mut env = build_library_env()?;
    if env::var("KREUZBERG_BENCHMARK_DEBUG").is_ok() {
        env.push(("KREUZBERG_BENCHMARK_DEBUG".to_string(), "true".to_string()));
    }
    let mut adapter = SubprocessAdapter::new("kreuzberg-go-sync", command, args, env);
    adapter.set_working_dir(scripts_dir);
    Ok(adapter)
}

/// Create Go batch adapter
pub fn create_go_batch_adapter() -> Result<SubprocessAdapter> {
    let script_path = get_script_path("kreuzberg_extract_go.go")?;
    let scripts_dir = script_path
        .parent()
        .ok_or_else(|| crate::Error::Config("Unable to determine scripts directory".to_string()))?
        .to_path_buf();
    let command = find_go()?;
    let args = vec![
        "run".to_string(),
        "kreuzberg_extract_go.go".to_string(),
        "batch".to_string(),
    ];
    let mut env = build_library_env()?;
    if env::var("KREUZBERG_BENCHMARK_DEBUG").is_ok() {
        env.push(("KREUZBERG_BENCHMARK_DEBUG".to_string(), "true".to_string()));
    }
    let mut adapter = SubprocessAdapter::with_batch_support("kreuzberg-go-batch", command, args, env);
    adapter.set_working_dir(scripts_dir);
    Ok(adapter)
}

/// Create Java sync adapter
pub fn create_java_sync_adapter() -> Result<SubprocessAdapter> {
    let _script_path = get_script_path("KreuzbergExtractJava.java")?;
    let command = find_java()?;
    let classpath = workspace_root()?.join("packages/java/target/classes");
    if !classpath.exists() {
        return Err(crate::Error::Config(format!(
            "Java classes not found at {} – run `mvn package` inside packages/java first",
            classpath.display()
        )));
    }
    let lib_dir = native_library_dir()?;
    let lib_dir_str = lib_dir.to_string_lossy().to_string();
    let mut env = build_library_env()?;
    env.push(("KREUZBERG_FFI_DIR".to_string(), lib_dir_str.clone()));
    let args = vec![
        "--enable-native-access=ALL-UNNAMED".to_string(),
        format!("-Djava.library.path={}", lib_dir.display()),
        "--class-path".to_string(),
        classpath.to_string_lossy().to_string(),
        "KreuzbergExtractJava".to_string(),
        "sync".to_string(),
    ];
    Ok(SubprocessAdapter::new("kreuzberg-java-sync", command, args, env))
}

/// Create C# sync adapter
pub fn create_csharp_sync_adapter() -> Result<SubprocessAdapter> {
    let command = find_dotnet()?;
    let project = workspace_root()?.join("packages/csharp/Benchmark/Benchmark.csproj");
    if !project.exists() {
        return Err(crate::Error::Config(format!(
            "C# benchmark project missing at {}",
            project.display()
        )));
    }
    let args = vec![
        "run".to_string(),
        "--project".to_string(),
        project.to_string_lossy().to_string(),
        "--".to_string(),
        "--file".to_string(),
    ];
    let lib_dir = native_library_dir()?;
    let mut env = build_library_env()?;
    env.push(("KREUZBERG_FFI_DIR".to_string(), lib_dir.to_string_lossy().to_string()));
    Ok(SubprocessAdapter::new("kreuzberg-csharp-sync", command, args, env))
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
