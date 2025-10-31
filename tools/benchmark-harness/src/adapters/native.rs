//! Native Kreuzberg Rust adapter
//!
//! This adapter uses the Kreuzberg Rust core library directly for maximum performance.
//! It serves as the baseline for comparing language bindings.

use crate::adapter::FrameworkAdapter;
use crate::monitoring::ResourceMonitor;
use crate::types::{BenchmarkResult, PerformanceMetrics};
use crate::{Error, Result};
use async_trait::async_trait;
use kreuzberg::{ExtractionConfig, extract_file};
use std::path::Path;
use std::time::{Duration, Instant};

/// Native Rust adapter using kreuzberg crate directly
pub struct NativeAdapter {
    config: ExtractionConfig,
}

impl NativeAdapter {
    /// Create a new native adapter with default configuration
    pub fn new() -> Self {
        Self {
            config: ExtractionConfig::default(),
        }
    }

    /// Create a new native adapter with custom configuration
    pub fn with_config(config: ExtractionConfig) -> Self {
        Self { config }
    }
}

impl Default for NativeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FrameworkAdapter for NativeAdapter {
    fn name(&self) -> &str {
        "kreuzberg-native"
    }

    fn supports_format(&self, file_type: &str) -> bool {
        // Kreuzberg supports a wide range of formats
        matches!(
            file_type.to_lowercase().as_str(),
            "pdf"
                | "docx"
                | "doc"
                | "xlsx"
                | "xls"
                | "pptx"
                | "ppt"
                | "txt"
                | "md"
                | "html"
                | "xml"
                | "json"
                | "yaml"
                | "toml"
                | "eml"
                | "msg"
                | "zip"
                | "tar"
                | "gz"
                | "jpg"
                | "jpeg"
                | "png"
                | "gif"
                | "bmp"
                | "tiff"
                | "webp"
        )
    }

    async fn extract(&self, file_path: &Path, timeout: Duration) -> Result<BenchmarkResult> {
        let file_size = std::fs::metadata(file_path).map_err(Error::Io)?.len();

        // Start resource monitoring
        let monitor = ResourceMonitor::new();
        monitor.start(Duration::from_millis(10)).await;

        let start = Instant::now();

        // Execute extraction with timeout
        let extraction_result = tokio::time::timeout(timeout, extract_file(file_path, None, &self.config))
            .await
            .map_err(|_| Error::Timeout(format!("Extraction exceeded {:?}", timeout)))?
            .map_err(|e| Error::Benchmark(format!("Extraction failed: {}", e)));

        let duration = start.elapsed();

        // Stop monitoring and collect samples
        let samples = monitor.stop().await;
        let resource_stats = ResourceMonitor::calculate_stats(&samples);

        // Calculate throughput
        let throughput = if duration.as_secs_f64() > 0.0 {
            file_size as f64 / duration.as_secs_f64()
        } else {
            0.0
        };

        // Handle extraction failure
        if let Err(e) = extraction_result {
            return Ok(BenchmarkResult {
                framework: self.name().to_string(),
                file_path: file_path.to_path_buf(),
                file_size,
                success: false,
                error_message: Some(e.to_string()),
                duration,
                metrics: PerformanceMetrics {
                    peak_memory_bytes: resource_stats.peak_memory_bytes,
                    avg_cpu_percent: resource_stats.avg_cpu_percent,
                    throughput_bytes_per_sec: 0.0,
                    p50_memory_bytes: resource_stats.p50_memory_bytes,
                    p95_memory_bytes: resource_stats.p95_memory_bytes,
                    p99_memory_bytes: resource_stats.p99_memory_bytes,
                },
                quality: None,
            });
        }

        // Success - return metrics with resource stats
        let metrics = PerformanceMetrics {
            peak_memory_bytes: resource_stats.peak_memory_bytes,
            avg_cpu_percent: resource_stats.avg_cpu_percent,
            throughput_bytes_per_sec: throughput,
            p50_memory_bytes: resource_stats.p50_memory_bytes,
            p95_memory_bytes: resource_stats.p95_memory_bytes,
            p99_memory_bytes: resource_stats.p99_memory_bytes,
        };

        Ok(BenchmarkResult {
            framework: self.name().to_string(),
            file_path: file_path.to_path_buf(),
            file_size,
            success: true,
            error_message: None,
            duration,
            metrics,
            quality: None,
        })
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    async fn setup(&self) -> Result<()> {
        // No setup required for native adapter
        Ok(())
    }

    async fn teardown(&self) -> Result<()> {
        // No cleanup required for native adapter
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_native_adapter_creation() {
        let adapter = NativeAdapter::new();
        assert_eq!(adapter.name(), "kreuzberg-native");
    }

    #[tokio::test]
    async fn test_supports_format() {
        let adapter = NativeAdapter::new();
        assert!(adapter.supports_format("pdf"));
        assert!(adapter.supports_format("docx"));
        assert!(adapter.supports_format("txt"));
        assert!(!adapter.supports_format("unknown"));
    }

    #[tokio::test]
    async fn test_extract_text_file() {
        let adapter = NativeAdapter::new();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "Hello, world!").unwrap();

        let result = adapter.extract(&file_path, Duration::from_secs(10)).await.unwrap();

        assert!(result.success);
        assert_eq!(result.framework, "kreuzberg-native");
        assert!(result.duration.as_millis() < 1000);
    }
}
