//! Output writers for benchmark results
//!
//! This module provides functionality for persisting benchmark results to disk
//! in JSON format.

use crate::types::BenchmarkResult;
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

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

/// Per-framework statistics for a specific file extension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkExtensionStats {
    /// Number of files tested
    pub count: usize,
    /// Number of successful extractions
    pub successful: usize,
    /// Success rate (0.0-1.0)
    pub success_rate: f64,
    /// Average duration in milliseconds
    pub avg_duration_ms: f64,
    /// Median duration in milliseconds
    pub median_duration_ms: f64,
    /// P95 duration in milliseconds
    pub p95_duration_ms: f64,
    /// Average throughput in MB/s
    pub avg_throughput_mbps: f64,
    /// Average peak memory in MB
    pub avg_peak_memory_mb: f64,
}

/// Analysis of results grouped by file extension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionAnalysis {
    /// Total number of files with this extension
    pub total_files: usize,
    /// Per-framework performance statistics
    pub framework_stats: HashMap<String, FrameworkExtensionStats>,
}

/// Complete by-extension analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByExtensionReport {
    /// Per-extension analysis
    pub by_extension: HashMap<String, ExtensionAnalysis>,
}

/// Analyze benchmark results by file extension
///
/// Groups results by file extension and calculates per-framework statistics
/// for each extension.
///
/// # Arguments
/// * `results` - Vector of benchmark results to analyze
///
/// # Returns
/// * ByExtensionReport with statistics grouped by extension and framework
pub fn analyze_by_extension(results: &[BenchmarkResult]) -> ByExtensionReport {
    let mut by_extension: HashMap<String, HashMap<String, Vec<&BenchmarkResult>>> = HashMap::new();

    // Group results by extension and framework
    for result in results {
        let ext = result.file_extension.clone();
        let framework = result.framework.clone();

        by_extension
            .entry(ext)
            .or_default()
            .entry(framework)
            .or_default()
            .push(result);
    }

    // Calculate statistics for each extension
    let mut report = HashMap::new();
    for (ext, framework_results) in by_extension {
        let total_files = framework_results.values().map(|v| v.len()).max().unwrap_or(0);

        let mut framework_stats = HashMap::new();
        for (framework, results) in framework_results {
            let stats = calculate_framework_stats(&results);
            framework_stats.insert(framework, stats);
        }

        report.insert(
            ext,
            ExtensionAnalysis {
                total_files,
                framework_stats,
            },
        );
    }

    ByExtensionReport { by_extension: report }
}

/// Calculate statistics for a framework's results
fn calculate_framework_stats(results: &[&BenchmarkResult]) -> FrameworkExtensionStats {
    let count = results.len();
    let successful = results.iter().filter(|r| r.success).count();
    let success_rate = if count > 0 {
        successful as f64 / count as f64
    } else {
        0.0
    };

    let successful_results: Vec<&&BenchmarkResult> = results.iter().filter(|r| r.success).collect();

    let avg_duration_ms = if !successful_results.is_empty() {
        successful_results
            .iter()
            .map(|r| r.duration.as_secs_f64() * 1000.0)
            .sum::<f64>()
            / successful_results.len() as f64
    } else {
        0.0
    };

    let mut durations: Vec<f64> = successful_results
        .iter()
        .map(|r| r.duration.as_secs_f64() * 1000.0)
        .collect();
    durations.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let median_duration_ms = if !durations.is_empty() {
        let mid = durations.len() / 2;
        if durations.len() % 2 == 0 {
            (durations[mid - 1] + durations[mid]) / 2.0
        } else {
            durations[mid]
        }
    } else {
        0.0
    };

    let p95_duration_ms = if !durations.is_empty() {
        let idx = ((durations.len() as f64 * 0.95) as usize).min(durations.len() - 1);
        durations[idx]
    } else {
        0.0
    };

    let avg_throughput_mbps = if !successful_results.is_empty() {
        successful_results
            .iter()
            .map(|r| r.metrics.throughput_bytes_per_sec / 1_000_000.0)
            .sum::<f64>()
            / successful_results.len() as f64
    } else {
        0.0
    };

    let avg_peak_memory_mb = if !successful_results.is_empty() {
        successful_results
            .iter()
            .map(|r| r.metrics.peak_memory_bytes as f64 / 1_000_000.0)
            .sum::<f64>()
            / successful_results.len() as f64
    } else {
        0.0
    };

    FrameworkExtensionStats {
        count,
        successful,
        success_rate,
        avg_duration_ms,
        median_duration_ms,
        p95_duration_ms,
        avg_throughput_mbps,
        avg_peak_memory_mb,
    }
}

/// Write by-extension analysis to JSON file
///
/// # Arguments
/// * `results` - Vector of benchmark results to analyze
/// * `output_path` - Path to output JSON file (e.g., "by-extension.json")
pub fn write_by_extension_analysis(results: &[BenchmarkResult], output_path: &Path) -> Result<()> {
    let report = analyze_by_extension(results);

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(Error::Io)?;
    }

    let json = serde_json::to_string_pretty(&report)
        .map_err(|e| Error::Benchmark(format!("Failed to serialize extension analysis: {}", e)))?;

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
            cold_start_duration: None,
            file_extension: "txt".to_string(),
            framework_capabilities: Default::default(),
            pdf_metadata: None,
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
