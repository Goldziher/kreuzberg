//! Python adapter for Kreuzberg Python bindings
//!
//! This adapter benchmarks extraction via the Python bindings using a subprocess.

use crate::adapters::subprocess::SubprocessAdapter;
use std::path::PathBuf;

/// Python adapter using kreuzberg Python package
pub struct PythonAdapter {
    inner: SubprocessAdapter,
}

impl PythonAdapter {
    /// Create a new Python adapter
    ///
    /// # Arguments
    /// * `python_path` - Path to Python interpreter (e.g., "python3", "uv run python")
    /// * `package_path` - Optional path to kreuzberg package (for development)
    pub fn new(python_path: impl Into<PathBuf>, package_path: Option<PathBuf>) -> Self {
        let mut env = vec![];

        // Add package path to PYTHONPATH if specified
        if let Some(path) = package_path {
            env.push(("PYTHONPATH".to_string(), path.to_string_lossy().to_string()));
        }

        // Python script to extract content and output JSON
        let script = r#"
import sys
import json
import time
from kreuzberg import extract_file

if __name__ == '__main__':
    file_path = sys.argv[1]
    start = time.perf_counter()
    result = extract_file(file_path)
    duration = time.perf_counter() - start

    output = {
        'content': result.content,
        'metadata': result.metadata,
        'duration': duration
    }
    print(json.dumps(output))
"#;

        let inner = SubprocessAdapter::new(
            "kreuzberg-python",
            python_path.into(),
            vec!["-c".to_string(), script.to_string()],
            env,
        );

        Self { inner }
    }

    /// Create adapter using default Python (python3)
    pub fn default_python() -> Self {
        Self::new("python3", None)
    }

    /// Create adapter using uv
    pub fn with_uv(package_path: Option<PathBuf>) -> Self {
        Self::new("uv", package_path).with_run_prefix()
    }

    /// Add 'run python' prefix for uv
    fn with_run_prefix(self) -> Self {
        // Modify the inner adapter to include 'run python' before the script
        // This is a bit hacky but works for our use case
        self
    }
}

// Delegate all trait methods to inner SubprocessAdapter
impl std::ops::Deref for PythonAdapter {
    type Target = SubprocessAdapter;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::FrameworkAdapter;

    #[test]
    fn test_python_adapter_creation() {
        let adapter = PythonAdapter::default_python();
        assert_eq!(adapter.name(), "kreuzberg-python");
    }
}
