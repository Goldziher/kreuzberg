//! Benchmark configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Benchmark execution mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BenchmarkMode {
    /// Single-file mode: Sequential execution (max_concurrent=1) for fair latency comparison
    SingleFile,
    /// Batch mode: Concurrent execution to measure throughput
    Batch,
}

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

    /// Benchmark execution mode (single-file or batch)
    pub benchmark_mode: BenchmarkMode,

    /// Number of warmup iterations (discarded from statistics)
    pub warmup_iterations: usize,

    /// Number of benchmark iterations for statistical analysis
    pub benchmark_iterations: usize,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            max_file_size: None,
            file_types: None,
            timeout: Duration::from_secs(1800),
            max_concurrent: num_cpus::get(),
            output_dir: PathBuf::from("results"),
            measure_quality: false,
            sample_interval_ms: 10,
            benchmark_mode: BenchmarkMode::Batch,
            warmup_iterations: 1,
            benchmark_iterations: 3,
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

        if self.benchmark_iterations == 0 {
            return Err(crate::Error::Config("benchmark_iterations must be > 0".to_string()));
        }

        if self.benchmark_mode == BenchmarkMode::SingleFile && self.max_concurrent != 1 {
            return Err(crate::Error::Config(
                "single-file mode requires max_concurrent=1".to_string(),
            ));
        }

        Ok(())
    }
}
