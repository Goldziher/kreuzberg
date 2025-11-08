//! Output writers for benchmark results
//!
//! This module provides functionality for persisting benchmark results to disk
//! in JSON format.

use crate::types::BenchmarkResult;
use crate::{Error, Result};
use std::fs;
use std::path::Path;

/// Write benchmark results to JSON file
///
/// # Arguments
/// * `results` - Vector of benchmark results to write
/// * `output_path` - Path to output JSON file
pub fn write_json(results: &[BenchmarkResult], output_path: &Path) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(Error::Io)?;
    }

    let json = serde_json::to_string_pretty(results)
        .map_err(|e| Error::Benchmark(format!("Failed to serialize results: {}", e)))?;

    fs::write(output_path, json).map_err(Error::Io)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PerformanceMetrics;
    use std::path::PathBuf;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn test_write_json() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("results.json");

        let results = vec![BenchmarkResult {
            framework: "test-framework".to_string(),
            file_path: PathBuf::from("/tmp/test.txt"),
            file_size: 1024,
            success: true,
            error_message: None,
            duration: Duration::from_secs(1),
            extraction_duration: None,
            subprocess_overhead: None,
            metrics: PerformanceMetrics {
                peak_memory_bytes: 10_000_000,
                avg_cpu_percent: 50.0,
                throughput_bytes_per_sec: 1024.0,
                p50_memory_bytes: 8_000_000,
                p95_memory_bytes: 9_500_000,
                p99_memory_bytes: 9_900_000,
            },
            quality: None,
            iterations: vec![],
            statistics: None,
        }];

        write_json(&results, &output_path).unwrap();

        assert!(output_path.exists());

        let contents = fs::read_to_string(&output_path).unwrap();
        let parsed: Vec<BenchmarkResult> = serde_json::from_str(&contents).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].framework, "test-framework");
    }

    #[test]
    fn test_write_json_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("subdir/results.json");

        let results = vec![];

        write_json(&results, &output_path).unwrap();

        assert!(output_path.exists());
        assert!(output_path.parent().unwrap().exists());
    }
}
