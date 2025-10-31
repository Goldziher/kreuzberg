//! Benchmark configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Configuration for benchmark runs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    /// Maximum file size to process (bytes)
    pub max_file_size: Option<u64>,

    /// File types to include (e.g., ["pdf", "docx"])
    pub file_types: Option<Vec<String>>,

    /// Timeout for each extraction
    pub timeout: Duration,

    /// Maximum number of concurrent extractions
    pub max_concurrent: usize,

    /// Output directory for results
    pub output_dir: PathBuf,

    /// Whether to include quality assessment
    pub measure_quality: bool,

    /// Sample interval for resource monitoring (milliseconds)
    pub sample_interval_ms: u64,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            max_file_size: None,
            file_types: None,
            timeout: Duration::from_secs(1800), // 30 minutes
            max_concurrent: num_cpus::get(),
            output_dir: PathBuf::from("results"),
            measure_quality: false,
            sample_interval_ms: 10,
        }
    }
}

impl BenchmarkConfig {
    /// Validate the configuration
    pub fn validate(&self) -> crate::Result<()> {
        if self.timeout.as_secs() == 0 {
            return Err(crate::Error::Config("Timeout must be > 0".to_string()));
        }

        if self.max_concurrent == 0 {
            return Err(crate::Error::Config("max_concurrent must be > 0".to_string()));
        }

        if self.sample_interval_ms == 0 {
            return Err(crate::Error::Config("sample_interval_ms must be > 0".to_string()));
        }

        Ok(())
    }
}
