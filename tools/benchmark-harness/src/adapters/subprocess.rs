//! Subprocess-based adapter for language bindings
//!
//! This adapter provides a base for running extraction via subprocess.
//! It's used by Python, Node.js, and Ruby adapters to execute extraction
//! in separate processes while monitoring resource usage.

use crate::adapter::FrameworkAdapter;
use crate::monitoring::ResourceMonitor;
use crate::types::{BenchmarkResult, PerformanceMetrics};
use crate::{Error, Result};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::Command;

/// Base adapter for subprocess-based extraction
///
/// This adapter spawns a subprocess to perform extraction and monitors
/// its resource usage. Subclasses implement the specific command construction
/// for each language binding.
pub struct SubprocessAdapter {
    name: String,
    command: PathBuf,
    args: Vec<String>,
    env: Vec<(String, String)>,
}

impl SubprocessAdapter {
    /// Create a new subprocess adapter
    ///
    /// # Arguments
    /// * `name` - Framework name (e.g., "kreuzberg-python")
    /// * `command` - Path to executable (e.g., "python3", "node")
    /// * `args` - Base arguments (e.g., ["-m", "kreuzberg"])
    /// * `env` - Environment variables
    pub fn new(
        name: impl Into<String>,
        command: impl Into<PathBuf>,
        args: Vec<String>,
        env: Vec<(String, String)>,
    ) -> Self {
        Self {
            name: name.into(),
            command: command.into(),
            args,
            env,
        }
    }

    /// Execute the extraction subprocess
    async fn execute_subprocess(&self, file_path: &Path, timeout: Duration) -> Result<(String, String, Duration)> {
        let start = Instant::now();

        // Build command with file path argument
        let mut cmd = Command::new(&self.command);
        cmd.args(&self.args);
        cmd.arg(file_path.to_string_lossy().as_ref());

        // Set environment variables
        for (key, value) in &self.env {
            cmd.env(key, value);
        }

        // Configure stdio
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Spawn process
        let child = cmd
            .spawn()
            .map_err(|e| Error::Benchmark(format!("Failed to spawn subprocess: {}", e)))?;

        // Wait for completion with timeout
        let output = match tokio::time::timeout(timeout, child.wait_with_output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                return Err(Error::Benchmark(format!("Failed to wait for subprocess: {}", e)));
            }
            Err(_) => {
                return Err(Error::Timeout(format!("Subprocess exceeded {:?}", timeout)));
            }
        };

        let duration = start.elapsed();

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            return Err(Error::Benchmark(format!(
                "Subprocess failed with exit code {:?}\nstderr: {}",
                output.status.code(),
                stderr
            )));
        }

        Ok((stdout, stderr, duration))
    }

    /// Parse extraction result from subprocess output
    ///
    /// Expected output format: JSON with `content` and optional `metadata` fields
    fn parse_output(&self, stdout: &str) -> Result<serde_json::Value> {
        serde_json::from_str(stdout).map_err(|e| Error::Benchmark(format!("Failed to parse subprocess output: {}", e)))
    }
}

#[async_trait]
impl FrameworkAdapter for SubprocessAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn supports_format(&self, file_type: &str) -> bool {
        // Assume subprocess adapters support the same formats as native
        // Subclasses can override for specific limitations
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

        // Execute subprocess
        let (stdout, _stderr, duration) = match self.execute_subprocess(file_path, timeout).await {
            Ok(result) => result,
            Err(e) => {
                // Stop monitoring and collect samples
                let samples = monitor.stop().await;
                let resource_stats = ResourceMonitor::calculate_stats(&samples);

                // Return error result with resource stats
                return Ok(BenchmarkResult {
                    framework: self.name.clone(),
                    file_path: file_path.to_path_buf(),
                    file_size,
                    success: false,
                    error_message: Some(e.to_string()),
                    duration: Duration::from_secs(0),
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
        };

        // Stop monitoring and collect samples
        let samples = monitor.stop().await;
        let resource_stats = ResourceMonitor::calculate_stats(&samples);

        // Parse output
        if let Err(e) = self.parse_output(&stdout) {
            return Ok(BenchmarkResult {
                framework: self.name.clone(),
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

        // Calculate throughput
        let throughput = if duration.as_secs_f64() > 0.0 {
            file_size as f64 / duration.as_secs_f64()
        } else {
            0.0
        };

        // Metrics with resource stats
        let metrics = PerformanceMetrics {
            peak_memory_bytes: resource_stats.peak_memory_bytes,
            avg_cpu_percent: resource_stats.avg_cpu_percent,
            throughput_bytes_per_sec: throughput,
            p50_memory_bytes: resource_stats.p50_memory_bytes,
            p95_memory_bytes: resource_stats.p95_memory_bytes,
            p99_memory_bytes: resource_stats.p99_memory_bytes,
        };

        Ok(BenchmarkResult {
            framework: self.name.clone(),
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
        // Try to get version from subprocess
        // This is a best-effort attempt
        "unknown".to_string()
    }

    async fn setup(&self) -> Result<()> {
        // Verify command exists
        which::which(&self.command)
            .map_err(|e| Error::Benchmark(format!("Command '{}' not found: {}", self.command.display(), e)))?;

        Ok(())
    }

    async fn teardown(&self) -> Result<()> {
        Ok(())
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            peak_memory_bytes: 0,
            avg_cpu_percent: 0.0,
            throughput_bytes_per_sec: 0.0,
            p50_memory_bytes: 0,
            p95_memory_bytes: 0,
            p99_memory_bytes: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subprocess_adapter_creation() {
        let adapter = SubprocessAdapter::new("test-adapter", "echo", vec!["test".to_string()], vec![]);
        assert_eq!(adapter.name(), "test-adapter");
    }

    #[test]
    fn test_supports_format() {
        let adapter = SubprocessAdapter::new("test", "echo", vec![], vec![]);
        assert!(adapter.supports_format("pdf"));
        assert!(adapter.supports_format("docx"));
        assert!(!adapter.supports_format("unknown"));
    }
}
