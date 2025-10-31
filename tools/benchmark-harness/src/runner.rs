//! Benchmark runner for executing and collecting results
//!
//! This module orchestrates benchmark execution across multiple fixtures and frameworks,
//! with support for concurrent execution and progress reporting.

use crate::adapter::FrameworkAdapter;
use crate::config::BenchmarkConfig;
use crate::fixture::FixtureManager;
use crate::registry::AdapterRegistry;
use crate::types::BenchmarkResult;
use crate::{Error, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::task::JoinSet;

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
        let mut tasks = JoinSet::new();
        let mut active_count = 0;

        // Create task queue: (fixture, adapter)
        let mut task_queue: Vec<(PathBuf, String, Arc<dyn FrameworkAdapter>)> = Vec::new();

        for (_fixture_path, fixture) in self.fixtures.fixtures() {
            for adapter in &frameworks {
                // Check if adapter supports this format
                if !adapter.supports_format(&fixture.file_type) {
                    continue;
                }

                task_queue.push((
                    fixture.document.clone(),
                    adapter.name().to_string(),
                    Arc::clone(adapter),
                ));
            }
        }

        let _total_tasks = task_queue.len();
        let mut task_iter = task_queue.into_iter();

        // Fill initial task pool
        while active_count < self.config.max_concurrent {
            if let Some((file_path, _framework_name, adapter)) = task_iter.next() {
                let timeout = self.config.timeout;

                tasks.spawn(async move { adapter.extract(&file_path, timeout).await });

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
                let timeout = self.config.timeout;

                tasks.spawn(async move { adapter.extract(&file_path, timeout).await });

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
