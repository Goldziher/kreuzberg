//! Benchmark runner for executing and collecting results
//!
//! This module orchestrates benchmark execution across multiple fixtures and frameworks,
//! with support for concurrent execution and progress reporting.

use crate::adapter::FrameworkAdapter;
use crate::config::{BenchmarkConfig, BenchmarkMode};
use crate::fixture::FixtureManager;
use crate::registry::AdapterRegistry;
use crate::types::{BenchmarkResult, DurationStatistics, IterationResult, PerformanceMetrics};
use crate::{Error, Result};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;

/// Calculate percentile from duration values
///
/// # Arguments
/// * `values` - Duration values (will be sorted)
/// * `percentile` - Percentile to calculate (0.0 - 1.0)
fn calculate_duration_percentile(mut values: Vec<Duration>, percentile: f64) -> Duration {
    if values.is_empty() {
        return Duration::from_secs(0);
    }

    values.sort();
    let index = ((values.len() as f64 - 1.0) * percentile).max(0.0) as usize;
    values[index]
}

/// Calculate statistics from iteration results
///
/// # Arguments
/// * `iterations` - Vector of iteration results to analyze
///
/// # Returns
/// Duration statistics including mean, median, std dev, and percentiles
fn calculate_statistics(iterations: &[IterationResult]) -> DurationStatistics {
    if iterations.is_empty() {
        return DurationStatistics {
            mean: Duration::from_secs(0),
            median: Duration::from_secs(0),
            std_dev_ms: 0.0,
            min: Duration::from_secs(0),
            max: Duration::from_secs(0),
            p95: Duration::from_secs(0),
            p99: Duration::from_secs(0),
            sample_count: 0,
        };
    }

    let durations: Vec<Duration> = iterations.iter().map(|i| i.duration).collect();

    let min = *durations.iter().min().unwrap();
    let max = *durations.iter().max().unwrap();

    let total_ms: f64 = durations.iter().map(|d| d.as_secs_f64() * 1000.0).sum();
    let mean_ms = total_ms / durations.len() as f64;
    let mean = Duration::from_secs_f64(mean_ms / 1000.0);

    let median = calculate_duration_percentile(durations.clone(), 0.50);

    let variance: f64 = durations
        .iter()
        .map(|d| {
            let diff = d.as_secs_f64() * 1000.0 - mean_ms;
            diff * diff
        })
        .sum::<f64>()
        / durations.len() as f64;

    let std_dev_ms = variance.sqrt();

    let p95 = calculate_duration_percentile(durations.clone(), 0.95);
    let p99 = calculate_duration_percentile(durations, 0.99);

    DurationStatistics {
        mean,
        median,
        std_dev_ms,
        min,
        max,
        p95,
        p99,
        sample_count: iterations.len(),
    }
}

/// Aggregate performance metrics from iterations (average)
fn aggregate_metrics(iterations: &[IterationResult]) -> PerformanceMetrics {
    if iterations.is_empty() {
        return PerformanceMetrics::default();
    }

    let count = iterations.len() as f64;

    let peak_memory_bytes = iterations
        .iter()
        .map(|i| i.metrics.peak_memory_bytes)
        .max()
        .unwrap_or(0);

    let avg_cpu_percent = iterations.iter().map(|i| i.metrics.avg_cpu_percent).sum::<f64>() / count;

    let throughput_bytes_per_sec = iterations
        .iter()
        .map(|i| i.metrics.throughput_bytes_per_sec)
        .sum::<f64>()
        / count;

    let p50_memory_bytes = (iterations.iter().map(|i| i.metrics.p50_memory_bytes).sum::<u64>() as f64 / count) as u64;

    let p95_memory_bytes = (iterations.iter().map(|i| i.metrics.p95_memory_bytes).sum::<u64>() as f64 / count) as u64;

    let p99_memory_bytes = (iterations.iter().map(|i| i.metrics.p99_memory_bytes).sum::<u64>() as f64 / count) as u64;

    PerformanceMetrics {
        peak_memory_bytes,
        avg_cpu_percent,
        throughput_bytes_per_sec,
        p50_memory_bytes,
        p95_memory_bytes,
        p99_memory_bytes,
    }
}

/// Orchestrates benchmark execution across fixtures and frameworks
pub struct BenchmarkRunner {
    config: BenchmarkConfig,
    registry: AdapterRegistry,
    fixtures: FixtureManager,
}

impl BenchmarkRunner {
    /// Create a new benchmark runner
    pub fn new(config: BenchmarkConfig, registry: AdapterRegistry) -> Self {
        Self {
            config,
            registry,
            fixtures: FixtureManager::new(),
        }
    }

    /// Load fixtures from a directory or file
    pub fn load_fixtures(&mut self, path: &PathBuf) -> Result<()> {
        if path.is_dir() {
            self.fixtures.load_fixtures_from_dir(path)?;
        } else {
            self.fixtures.load_fixture(path)?;
        }
        Ok(())
    }

    /// Filter fixtures by file type (to be implemented when needed)
    ///
    /// For now, filtering is done during execution based on adapter support
    pub fn filter_fixtures(&mut self, _file_types: &[String]) {
        // TODO: Implement fixture filtering if needed
        // For now, we filter during execution based on adapter.supports_format()
    }

    /// Get count of loaded fixtures
    pub fn fixture_count(&self) -> usize {
        self.fixtures.len()
    }

    /// Run multiple iterations of a single extraction task (static method for async spawning)
    ///
    /// # Arguments
    /// * `file_path` - Path to file to extract
    /// * `adapter` - Framework adapter to use
    /// * `config` - Benchmark configuration
    ///
    /// # Returns
    /// Aggregated benchmark result with iterations and statistics
    async fn run_iterations_static(
        file_path: &PathBuf,
        adapter: Arc<dyn FrameworkAdapter>,
        config: &BenchmarkConfig,
    ) -> Result<BenchmarkResult> {
        let total_iterations = config.warmup_iterations + config.benchmark_iterations;
        let mut all_results = Vec::new();

        // Run all iterations (warmup + benchmark)
        for iteration in 0..total_iterations {
            let result = adapter.extract(file_path, config.timeout).await?;

            // Only keep benchmark iterations (skip warmup)
            if iteration >= config.warmup_iterations {
                all_results.push(result);
            }
        }

        // If only 1 benchmark iteration, return it directly without aggregation
        if config.benchmark_iterations == 1 && !all_results.is_empty() {
            return Ok(all_results.into_iter().next().unwrap());
        }

        // Aggregate multiple iterations
        if all_results.is_empty() {
            return Err(Error::Benchmark("No successful iterations".to_string()));
        }

        // Extract iteration results
        let iterations: Vec<IterationResult> = all_results
            .iter()
            .enumerate()
            .map(|(idx, result)| IterationResult {
                iteration: idx + 1,
                duration: result.duration,
                extraction_duration: result.extraction_duration,
                metrics: result.metrics.clone(),
            })
            .collect();

        // Calculate statistics
        let statistics = calculate_statistics(&iterations);

        // Aggregate metrics
        let aggregated_metrics = aggregate_metrics(&iterations);

        // Calculate average extraction_duration if available
        let extraction_durations: Vec<Duration> = all_results.iter().filter_map(|r| r.extraction_duration).collect();

        let avg_extraction_duration = if !extraction_durations.is_empty() {
            let total_ms: f64 = extraction_durations.iter().map(|d| d.as_secs_f64() * 1000.0).sum();
            Some(Duration::from_secs_f64(
                total_ms / extraction_durations.len() as f64 / 1000.0,
            ))
        } else {
            None
        };

        // Calculate subprocess overhead from mean duration and avg extraction duration
        let subprocess_overhead = avg_extraction_duration.map(|ext| statistics.mean.saturating_sub(ext));

        // Use first result as template for aggregated result
        let first_result = &all_results[0];

        Ok(BenchmarkResult {
            framework: first_result.framework.clone(),
            file_path: first_result.file_path.clone(),
            file_size: first_result.file_size,
            success: true,
            error_message: None,
            duration: statistics.mean,
            extraction_duration: avg_extraction_duration,
            subprocess_overhead,
            metrics: aggregated_metrics,
            quality: first_result.quality.clone(),
            iterations,
            statistics: Some(statistics),
        })
    }

    /// Run benchmarks for specified frameworks
    ///
    /// # Arguments
    /// * `framework_names` - Names of frameworks to benchmark (empty = all registered)
    ///
    /// # Returns
    /// Vector of benchmark results
    pub async fn run(&self, framework_names: &[String]) -> Result<Vec<BenchmarkResult>> {
        // Determine which frameworks to benchmark
        let frameworks = if framework_names.is_empty() {
            // Use all registered adapters
            self.registry
                .adapter_names()
                .into_iter()
                .filter_map(|name| self.registry.get(&name))
                .collect::<Vec<_>>()
        } else {
            // Use specified adapters
            framework_names
                .iter()
                .filter_map(|name| self.registry.get(name))
                .collect::<Vec<_>>()
        };

        if frameworks.is_empty() {
            return Err(Error::Benchmark("No frameworks available for benchmarking".to_string()));
        }

        // Setup all frameworks
        for adapter in &frameworks {
            adapter.setup().await?;
        }

        let mut results = Vec::new();

        // Determine concurrency based on benchmark mode
        let max_concurrent = match self.config.benchmark_mode {
            BenchmarkMode::SingleFile => 1, // Sequential execution for fair latency comparison
            BenchmarkMode::Batch => self.config.max_concurrent, // Concurrent for throughput
        };

        let mut tasks = JoinSet::new();
        let mut active_count = 0;

        // Create task queue: (fixture, adapter)
        let mut task_queue: Vec<(PathBuf, String, Arc<dyn FrameworkAdapter>)> = Vec::new();

        for (fixture_path, fixture) in self.fixtures.fixtures() {
            for adapter in &frameworks {
                // Check if adapter supports this format
                if !adapter.supports_format(&fixture.file_type) {
                    continue;
                }

                // Resolve document path relative to fixture directory
                let fixture_dir = fixture_path.parent().unwrap_or_else(|| std::path::Path::new("."));
                let document_path = fixture.resolve_document_path(fixture_dir);

                task_queue.push((document_path, adapter.name().to_string(), Arc::clone(adapter)));
            }
        }

        let _total_tasks = task_queue.len();
        let mut task_iter = task_queue.into_iter();

        // Clone config data needed for async blocks
        let config = self.config.clone();

        // Fill initial task pool
        while active_count < max_concurrent {
            if let Some((file_path, _framework_name, adapter)) = task_iter.next() {
                let config = config.clone();

                tasks.spawn(async move { Self::run_iterations_static(&file_path, adapter, &config).await });

                active_count += 1;
            } else {
                break;
            }
        }

        // Process tasks as they complete
        while let Some(task_result) = tasks.join_next().await {
            // Handle task completion
            match task_result {
                Ok(Ok(result)) => {
                    results.push(result);
                }
                Ok(Err(e)) => {
                    eprintln!("Benchmark task failed: {}", e);
                }
                Err(e) => {
                    eprintln!("Task join error: {}", e);
                }
            }

            active_count -= 1;

            // Spawn next task if available
            if let Some((file_path, _framework_name, adapter)) = task_iter.next() {
                let config = config.clone();

                tasks.spawn(async move { Self::run_iterations_static(&file_path, adapter, &config).await });

                active_count += 1;
            }
        }

        // Teardown all frameworks
        for adapter in &frameworks {
            adapter.teardown().await?;
        }

        Ok(results)
    }

    /// Get reference to benchmark configuration
    pub fn config(&self) -> &BenchmarkConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::NativeAdapter;

    #[tokio::test]
    async fn test_benchmark_runner_creation() {
        let config = BenchmarkConfig::default();
        let registry = AdapterRegistry::new();
        let runner = BenchmarkRunner::new(config, registry);

        assert_eq!(runner.fixture_count(), 0);
    }

    #[tokio::test]
    async fn test_run_with_no_frameworks() {
        let config = BenchmarkConfig::default();
        let registry = AdapterRegistry::new();
        let runner = BenchmarkRunner::new(config, registry);

        let result = runner.run(&[]).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No frameworks available"));
    }

    #[tokio::test]
    async fn test_run_with_native_adapter() {
        let config = BenchmarkConfig::default();
        let mut registry = AdapterRegistry::new();
        registry.register(Arc::new(NativeAdapter::new())).unwrap();

        let runner = BenchmarkRunner::new(config, registry);

        // Running with no fixtures should return empty results
        let results = runner.run(&[]).await.unwrap();
        assert_eq!(results.len(), 0);
    }
}
