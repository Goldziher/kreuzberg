//! Resource monitoring for benchmark execution
//!
//! This module provides real-time monitoring of CPU and memory usage during
//! document extraction, with percentile calculations for performance analysis.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tokio::sync::Mutex;

/// Sample of resource usage at a point in time
#[derive(Debug, Clone, Copy)]
pub struct ResourceSample {
    /// Memory usage in bytes
    pub memory_bytes: u64,
    /// CPU usage percentage (0.0 - 100.0 * num_cpus)
    pub cpu_percent: f64,
    /// Timestamp when sample was taken (relative to monitoring start)
    pub timestamp_ms: u64,
}

/// Resource monitor that samples CPU and memory usage periodically
pub struct ResourceMonitor {
    samples: Arc<Mutex<Vec<ResourceSample>>>,
    running: Arc<AtomicBool>,
    pid: Pid,
}

impl ResourceMonitor {
    /// Create a new resource monitor for the current process
    pub fn new() -> Self {
        let pid = sysinfo::get_current_pid().expect("Failed to get current PID");
        Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            running: Arc::new(AtomicBool::new(false)),
            pid,
        }
    }

    /// Start monitoring resources in the background
    ///
    /// Spawns a background task that samples memory and CPU usage at the specified interval.
    ///
    /// # Arguments
    /// * `sample_interval` - How often to sample (e.g., Duration::from_millis(10))
    pub async fn start(&self, sample_interval: Duration) {
        if self.running.swap(true, Ordering::SeqCst) {
            // Already running
            return;
        }

        let samples = Arc::clone(&self.samples);
        let running = Arc::clone(&self.running);
        let pid = self.pid;

        tokio::spawn(async move {
            let mut system = System::new();
            let start = std::time::Instant::now();

            // Use faster refresh - only update what we need
            let refresh_kind = ProcessRefreshKind::nothing().with_memory().with_cpu();

            while running.load(Ordering::SeqCst) {
                // Refresh only the specific process
                system.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), false, refresh_kind);

                if let Some(process) = system.process(pid) {
                    let sample = ResourceSample {
                        memory_bytes: process.memory(),
                        cpu_percent: process.cpu_usage() as f64,
                        timestamp_ms: start.elapsed().as_millis() as u64,
                    };

                    samples.lock().await.push(sample);
                }

                tokio::time::sleep(sample_interval).await;
            }
        });
    }

    /// Stop monitoring and return collected samples
    pub async fn stop(&self) -> Vec<ResourceSample> {
        self.running.store(false, Ordering::SeqCst);

        // Give the background task a moment to finish
        tokio::time::sleep(Duration::from_millis(20)).await;

        let samples = self.samples.lock().await;
        samples.clone()
    }

    /// Calculate percentile from samples
    ///
    /// # Arguments
    /// * `samples` - Sorted samples (will be sorted if not already)
    /// * `percentile` - Percentile to calculate (0.0 - 1.0)
    fn calculate_percentile(mut values: Vec<u64>, percentile: f64) -> u64 {
        if values.is_empty() {
            return 0;
        }

        values.sort_unstable();
        let index = ((values.len() as f64 - 1.0) * percentile) as usize;
        values[index]
    }

    /// Calculate resource statistics from samples
    pub fn calculate_stats(samples: &[ResourceSample]) -> ResourceStats {
        if samples.is_empty() {
            return ResourceStats::default();
        }

        let memory_values: Vec<u64> = samples.iter().map(|s| s.memory_bytes).collect();
        let cpu_values: Vec<f64> = samples.iter().map(|s| s.cpu_percent).collect();

        let peak_memory = *memory_values.iter().max().unwrap_or(&0);
        let avg_cpu = cpu_values.iter().sum::<f64>() / cpu_values.len() as f64;

        ResourceStats {
            peak_memory_bytes: peak_memory,
            avg_cpu_percent: avg_cpu,
            p50_memory_bytes: Self::calculate_percentile(memory_values.clone(), 0.50),
            p95_memory_bytes: Self::calculate_percentile(memory_values.clone(), 0.95),
            p99_memory_bytes: Self::calculate_percentile(memory_values, 0.99),
            sample_count: samples.len(),
        }
    }
}

impl Default for ResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Resource usage statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct ResourceStats {
    /// Peak memory usage in bytes
    pub peak_memory_bytes: u64,
    /// Average CPU usage percentage
    pub avg_cpu_percent: f64,
    /// 50th percentile (median) memory usage
    pub p50_memory_bytes: u64,
    /// 95th percentile memory usage
    pub p95_memory_bytes: u64,
    /// 99th percentile memory usage
    pub p99_memory_bytes: u64,
    /// Number of samples collected
    pub sample_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_percentile() {
        let values = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        assert_eq!(ResourceMonitor::calculate_percentile(values.clone(), 0.0), 1);
        assert_eq!(ResourceMonitor::calculate_percentile(values.clone(), 0.5), 5);
        assert_eq!(ResourceMonitor::calculate_percentile(values.clone(), 0.95), 9); // p95 of 10 values = index 8
        assert_eq!(ResourceMonitor::calculate_percentile(values, 1.0), 10);
    }

    #[test]
    fn test_calculate_percentile_single_value() {
        let values = vec![42];
        assert_eq!(ResourceMonitor::calculate_percentile(values, 0.5), 42);
    }

    #[test]
    fn test_calculate_percentile_empty() {
        let values = vec![];
        assert_eq!(ResourceMonitor::calculate_percentile(values, 0.5), 0);
    }

    #[tokio::test]
    async fn test_resource_monitor_basic() {
        let monitor = ResourceMonitor::new();

        monitor.start(Duration::from_millis(10)).await;
        tokio::time::sleep(Duration::from_millis(50)).await;
        let samples = monitor.stop().await;

        // Should have collected some samples
        assert!(!samples.is_empty(), "Should have collected samples");
        assert!(samples.len() >= 3, "Should have at least 3 samples");
    }

    #[tokio::test]
    async fn test_resource_stats_calculation() {
        let samples = vec![
            ResourceSample {
                memory_bytes: 100,
                cpu_percent: 10.0,
                timestamp_ms: 0,
            },
            ResourceSample {
                memory_bytes: 200,
                cpu_percent: 20.0,
                timestamp_ms: 10,
            },
            ResourceSample {
                memory_bytes: 150,
                cpu_percent: 15.0,
                timestamp_ms: 20,
            },
        ];

        let stats = ResourceMonitor::calculate_stats(&samples);

        assert_eq!(stats.peak_memory_bytes, 200);
        assert_eq!(stats.p50_memory_bytes, 150);
        assert!((stats.avg_cpu_percent - 15.0).abs() < 0.1);
        assert_eq!(stats.sample_count, 3);
    }

    #[tokio::test]
    async fn test_resource_stats_empty() {
        let stats = ResourceMonitor::calculate_stats(&[]);
        assert_eq!(stats.peak_memory_bytes, 0);
        assert_eq!(stats.sample_count, 0);
    }
}
