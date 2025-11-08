//! Ruby adapter for Kreuzberg Ruby bindings
//!
//! This adapter benchmarks extraction via the Ruby bindings using a subprocess.

use crate::adapters::subprocess::SubprocessAdapter;
use std::path::PathBuf;

/// Ruby adapter using kreuzberg gem
pub struct RubyAdapter {
    inner: SubprocessAdapter,
}

impl RubyAdapter {
    /// Create a new Ruby adapter
    ///
    /// # Arguments
    /// * `ruby_path` - Path to Ruby interpreter (e.g., "ruby")
    /// * `gem_path` - Optional path to kreuzberg gem (for development)
    pub fn new(ruby_path: impl Into<PathBuf>, gem_path: Option<PathBuf>) -> Self {
        let mut env = vec![];

        if let Some(path) = gem_path {
            let lib_path = path.join("lib");
            env.push(("RUBYLIB".to_string(), lib_path.to_string_lossy().to_string()));
        }

        let script = r#"
require 'kreuzberg'
require 'json'

file_path = ARGV[0]
start = Process.clock_gettime(Process::CLOCK_MONOTONIC)

result = Kreuzberg.extract_file(file_path)
duration = Process.clock_gettime(Process::CLOCK_MONOTONIC) - start

output = {
  content: result.content,
  metadata: result.metadata,
  duration: duration
}

puts JSON.generate(output)
"#;

        let inner = SubprocessAdapter::new(
            "kreuzberg-ruby",
            ruby_path.into(),
            vec!["-e".to_string(), script.to_string()],
            env,
        );

        Self { inner }
    }

    /// Create adapter using default Ruby (ruby)
    pub fn default_ruby() -> Self {
        Self::new("ruby", None)
    }

    /// Create adapter using bundle exec
    pub fn with_bundle(gem_path: Option<PathBuf>) -> Self {
        Self::new("bundle", gem_path)
    }
}

impl std::ops::Deref for RubyAdapter {
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
    fn test_ruby_adapter_creation() {
        let adapter = RubyAdapter::default_ruby();
        assert_eq!(adapter.name(), "kreuzberg-ruby");
    }
}
