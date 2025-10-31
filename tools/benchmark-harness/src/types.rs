//! Core types for benchmark results and metrics

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Complete benchmark result for a single file extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Framework that performed the extraction
    pub framework: String,

    /// Path to the test document
    pub file_path: PathBuf,

    /// File size in bytes
    pub file_size: u64,

    /// Whether extraction succeeded
    pub success: bool,

    /// Error message if extraction failed
    pub error_message: Option<String>,

    /// Extraction duration
    pub duration: Duration,

    /// Performance metrics
    pub metrics: PerformanceMetrics,

    /// Quality metrics (if ground truth available)
    pub quality: Option<QualityMetrics>,
}

/// Performance metrics collected during extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Peak memory usage in bytes
    pub peak_memory_bytes: u64,

    /// Average CPU usage percentage (0-100)
    pub avg_cpu_percent: f64,

    /// Throughput in bytes per second
    pub throughput_bytes_per_sec: f64,

    /// 50th percentile memory usage in bytes
    pub p50_memory_bytes: u64,

    /// 95th percentile memory usage in bytes
    pub p95_memory_bytes: u64,

    /// 99th percentile memory usage in bytes
    pub p99_memory_bytes: u64,
}

/// Quality metrics comparing extraction output to ground truth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    /// Text token F1 score (0.0-1.0)
    pub f1_score_text: f64,

    /// Numeric token F1 score (0.0-1.0)
    pub f1_score_numeric: f64,

    /// Layout/structure F1 score (0.0-1.0)
    pub f1_score_layout: f64,

    /// Overall text quality score (0.0-1.0)
    pub quality_score: f64,
}

/// Summary statistics for all extractions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSummary {
    /// Framework name
    pub framework: String,

    /// Total number of files processed
    pub total_files: usize,

    /// Number of successful extractions
    pub successful: usize,

    /// Number of failed extractions
    pub failed: usize,

    /// Success rate (0.0-1.0)
    pub success_rate: f64,

    /// Average extraction duration
    pub avg_duration: Duration,

    /// Average throughput in bytes per second
    pub avg_throughput: f64,

    /// Average peak memory usage in bytes
    pub avg_peak_memory: u64,

    /// 95th percentile duration
    pub p95_duration: Duration,

    /// 99th percentile duration
    pub p99_duration: Duration,

    /// Average quality metrics (if available)
    pub avg_quality: Option<QualityMetrics>,
}
