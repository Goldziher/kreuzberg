//! Node.js adapter for Kreuzberg TypeScript/Node bindings
//!
//! This adapter benchmarks extraction via the Node.js bindings using a subprocess.

use crate::adapters::subprocess::SubprocessAdapter;
use std::path::PathBuf;

/// Node.js adapter using @goldziher/kreuzberg package
pub struct NodeAdapter {
    inner: SubprocessAdapter,
}

impl NodeAdapter {
    /// Create a new Node.js adapter
    ///
    /// # Arguments
    /// * `node_path` - Path to Node executable (e.g., "node")
    /// * `package_path` - Optional path to kreuzberg package (for development)
    pub fn new(node_path: impl Into<PathBuf>, package_path: Option<PathBuf>) -> Self {
        let mut env = vec![];

        // Add package path to NODE_PATH if specified
        if let Some(path) = package_path {
            env.push(("NODE_PATH".to_string(), path.to_string_lossy().to_string()));
        }

        // JavaScript code to extract content and output JSON
        let script = r#"
const { extractFile } = require('@goldziher/kreuzberg');

const filePath = process.argv[2];
const start = performance.now();

extractFile(filePath)
    .then(result => {
        const duration = (performance.now() - start) / 1000;
        const output = {
            content: result.content,
            metadata: result.metadata,
            duration: duration
        };
        console.log(JSON.stringify(output));
    })
    .catch(err => {
        console.error(err);
        process.exit(1);
    });
"#;

        let inner = SubprocessAdapter::new(
            "kreuzberg-node",
            node_path.into(),
            vec!["-e".to_string(), script.to_string()],
            env,
        );

        Self { inner }
    }

    /// Create adapter using default Node.js (node)
    pub fn default_node() -> Self {
        Self::new("node", None)
    }
}

// Delegate all trait methods to inner SubprocessAdapter
impl std::ops::Deref for NodeAdapter {
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
    fn test_node_adapter_creation() {
        let adapter = NodeAdapter::default_node();
        assert_eq!(adapter.name(), "kreuzberg-node");
    }
}
