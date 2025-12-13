//! HTML visualization output for benchmark results
//!
//! This module generates static HTML pages with embedded Chart.js visualizations
//! for benchmark results. The output is a single self-contained HTML file that
//! can be viewed in any browser without external dependencies (except Chart.js CDN).

use crate::types::BenchmarkResult;
use crate::{Error, Result};
use minijinja::{AutoEscape, Environment, context};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

/// Chart data aggregated for visualization
#[derive(Debug, Clone, Serialize)]
struct ChartData {
    /// Frameworks included in the dataset
    frameworks: Vec<String>,
    /// File extensions in the dataset
    extensions: Vec<String>,
    /// Per-framework aggregated metrics
    framework_metrics: HashMap<String, AggregatedMetrics>,
    /// Per-extension per-framework metrics
    extension_metrics: HashMap<String, HashMap<String, AggregatedMetrics>>,
    /// Benchmark run date (when the benchmark was actually executed)
    benchmark_run_date: Option<String>,
    /// HTML generation timestamp (when the HTML file was created)
    generated_at: String,
}

/// Aggregated metrics for a framework or framework-extension combination
#[derive(Debug, Clone, Serialize)]
struct AggregatedMetrics {
    /// Number of files processed
    count: usize,
    /// Number of successful extractions
    successful: usize,
    /// Success rate (0.0-1.0)
    success_rate: f64,
    /// Mean duration in milliseconds
    mean_duration_ms: f64,
    /// Median duration in milliseconds
    median_duration_ms: f64,
    /// P95 duration in milliseconds
    p95_duration_ms: f64,
    /// P99 duration in milliseconds (if available)
    p99_duration_ms: Option<f64>,
    /// Average throughput in MB/s
    avg_throughput_mbps: f64,
    /// Peak memory in MB
    peak_memory_mb: f64,
    /// P95 memory in MB
    p95_memory_mb: f64,
    /// P99 memory in MB
    p99_memory_mb: f64,
    /// Average CPU percentage
    avg_cpu_percent: f64,
}

/// Static template environment (initialized once)
static TEMPLATE_ENV: OnceLock<Environment<'static>> = OnceLock::new();

/// Initialize the MiniJinja template environment with all templates
fn init_template_env() -> Environment<'static> {
    let mut env = Environment::new();

    // Add all templates using include_str! for compile-time embedding
    env.add_template("base.html.jinja", include_str!("../templates/base.html.jinja"))
        .expect("Failed to add base template");

    // Components
    env.add_template(
        "components/header.html.jinja",
        include_str!("../templates/components/header.html.jinja"),
    )
    .expect("Failed to add header template");
    env.add_template(
        "components/tabs.html.jinja",
        include_str!("../templates/components/tabs.html.jinja"),
    )
    .expect("Failed to add tabs template");
    env.add_template(
        "components/success_summary.html.jinja",
        include_str!("../templates/components/success_summary.html.jinja"),
    )
    .expect("Failed to add success_summary template");
    env.add_template(
        "components/empty_state.html.jinja",
        include_str!("../templates/components/empty_state.html.jinja"),
    )
    .expect("Failed to add empty_state template");

    // Charts - HTML
    env.add_template(
        "charts/duration.html.jinja",
        include_str!("../templates/charts/duration.html.jinja"),
    )
    .expect("Failed to add duration chart template");
    env.add_template(
        "charts/throughput.html.jinja",
        include_str!("../templates/charts/throughput.html.jinja"),
    )
    .expect("Failed to add throughput chart template");
    env.add_template(
        "charts/memory.html.jinja",
        include_str!("../templates/charts/memory.html.jinja"),
    )
    .expect("Failed to add memory chart template");
    env.add_template(
        "charts/filetype.html.jinja",
        include_str!("../templates/charts/filetype.html.jinja"),
    )
    .expect("Failed to add filetype chart template");
    env.add_template(
        "charts/success.html.jinja",
        include_str!("../templates/charts/success.html.jinja"),
    )
    .expect("Failed to add success chart template");

    // Charts - Scripts
    env.add_template(
        "charts/duration_script.js.jinja",
        include_str!("../templates/charts/duration_script.js.jinja"),
    )
    .expect("Failed to add duration script template");
    env.add_template(
        "charts/throughput_script.js.jinja",
        include_str!("../templates/charts/throughput_script.js.jinja"),
    )
    .expect("Failed to add throughput script template");
    env.add_template(
        "charts/memory_script.js.jinja",
        include_str!("../templates/charts/memory_script.js.jinja"),
    )
    .expect("Failed to add memory script template");
    env.add_template(
        "charts/filetype_script.js.jinja",
        include_str!("../templates/charts/filetype_script.js.jinja"),
    )
    .expect("Failed to add filetype script template");
    env.add_template(
        "charts/success_script.js.jinja",
        include_str!("../templates/charts/success_script.js.jinja"),
    )
    .expect("Failed to add success script template");

    // Styles
    env.add_template(
        "styles/variables.css.jinja",
        include_str!("../templates/styles/variables.css.jinja"),
    )
    .expect("Failed to add variables CSS template");
    env.add_template(
        "styles/layout.css.jinja",
        include_str!("../templates/styles/layout.css.jinja"),
    )
    .expect("Failed to add layout CSS template");
    env.add_template(
        "styles/components.css.jinja",
        include_str!("../templates/styles/components.css.jinja"),
    )
    .expect("Failed to add components CSS template");
    env.add_template(
        "styles/charts.css.jinja",
        include_str!("../templates/styles/charts.css.jinja"),
    )
    .expect("Failed to add charts CSS template");

    // Enable auto-escaping for HTML and CSS files
    env.set_auto_escape_callback(|name| {
        if name.ends_with(".html.jinja") || name.ends_with(".css.jinja") {
            AutoEscape::Html
        } else {
            AutoEscape::None
        }
    });

    env
}

/// Get the template environment (initializes on first call)
fn get_template_env() -> &'static Environment<'static> {
    TEMPLATE_ENV.get_or_init(init_template_env)
}

/// Write benchmark results as interactive HTML visualization
///
/// Generates a single self-contained HTML file with embedded Chart.js charts
/// and benchmark data. The output includes 5 chart types:
/// - Duration comparison (p95, p50)
/// - Throughput comparison
/// - Memory analysis (peak, p95, p99)
/// - File type breakdown
/// - Success rate dashboard
///
/// # Arguments
/// * `results` - Vector of benchmark results to visualize
/// * `output_path` - Path to output HTML file
/// * `benchmark_date` - Optional benchmark execution date (e.g., "2025-12-13 14:30:00 UTC").
///   If not provided, current timestamp is used as fallback
pub fn write_html(results: &[BenchmarkResult], output_path: &Path, benchmark_date: Option<&str>) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(Error::Io)?;
    }

    let chart_data = build_chart_data(results, benchmark_date)?;
    let html = generate_html(&chart_data)?;

    fs::write(output_path, html).map_err(Error::Io)?;

    Ok(())
}

/// Build aggregated chart data from benchmark results
fn build_chart_data(results: &[BenchmarkResult], benchmark_date: Option<&str>) -> Result<ChartData> {
    let mut frameworks = Vec::new();
    let mut extensions = Vec::new();
    let mut framework_results: HashMap<String, Vec<&BenchmarkResult>> = HashMap::new();
    let mut extension_results: HashMap<String, HashMap<String, Vec<&BenchmarkResult>>> = HashMap::new();

    // Group results by framework and extension
    for result in results {
        if !frameworks.contains(&result.framework) {
            frameworks.push(result.framework.clone());
        }
        if !extensions.contains(&result.file_extension) {
            extensions.push(result.file_extension.clone());
        }

        framework_results
            .entry(result.framework.clone())
            .or_default()
            .push(result);

        extension_results
            .entry(result.file_extension.clone())
            .or_default()
            .entry(result.framework.clone())
            .or_default()
            .push(result);
    }

    // Sort for consistent output
    frameworks.sort();
    extensions.sort();

    // Calculate per-framework metrics
    let framework_metrics = framework_results
        .iter()
        .map(|(framework, results)| {
            let metrics = calculate_aggregated_metrics(results);
            (framework.clone(), metrics)
        })
        .collect();

    // Calculate per-extension per-framework metrics
    let mut extension_metrics = HashMap::new();
    for (ext, frameworks) in extension_results {
        let framework_stats = frameworks
            .iter()
            .map(|(framework, results)| {
                let metrics = calculate_aggregated_metrics(results);
                (framework.clone(), metrics)
            })
            .collect();
        extension_metrics.insert(ext, framework_stats);
    }

    let benchmark_run_date = benchmark_date.map(|d| d.to_string());
    let generated_at = chrono::Utc::now().to_rfc3339();

    Ok(ChartData {
        frameworks,
        extensions,
        framework_metrics,
        extension_metrics,
        benchmark_run_date,
        generated_at,
    })
}

/// Calculate aggregated metrics from a set of results
fn calculate_aggregated_metrics(results: &[&BenchmarkResult]) -> AggregatedMetrics {
    let count = results.len();
    let successful = results.iter().filter(|r| r.success).count();
    let success_rate = if count > 0 {
        successful as f64 / count as f64
    } else {
        0.0
    };

    let mut durations_ms = Vec::new();
    let mut p95_durations_ms = Vec::new();
    let mut p99_durations_ms = Vec::new();
    let mut throughputs_mbps = Vec::new();
    let mut peak_memories_mb = Vec::new();
    let mut p95_memories_mb = Vec::new();
    let mut p99_memories_mb = Vec::new();
    let mut cpu_percents = Vec::new();

    for result in results {
        durations_ms.push(duration_to_ms(result.duration));

        // Extract percentiles from statistics if available
        if let Some(stats) = &result.statistics {
            p95_durations_ms.push(duration_to_ms(stats.p95));
            p99_durations_ms.push(duration_to_ms(stats.p99));
        } else {
            // Fall back to mean duration
            p95_durations_ms.push(duration_to_ms(result.duration));
            p99_durations_ms.push(duration_to_ms(result.duration));
        }

        throughputs_mbps.push(result.metrics.throughput_bytes_per_sec / 1_000_000.0);
        peak_memories_mb.push(result.metrics.peak_memory_bytes as f64 / 1_048_576.0);
        p95_memories_mb.push(result.metrics.p95_memory_bytes as f64 / 1_048_576.0);
        p99_memories_mb.push(result.metrics.p99_memory_bytes as f64 / 1_048_576.0);
        cpu_percents.push(result.metrics.avg_cpu_percent);
    }

    let mean_duration_ms = calculate_mean(&durations_ms);
    let median_duration_ms = calculate_median(&mut durations_ms);
    let p95_duration_ms = calculate_mean(&p95_durations_ms);
    let p99_duration_ms = if !p99_durations_ms.is_empty() {
        Some(calculate_mean(&p99_durations_ms))
    } else {
        None
    };

    let avg_throughput_mbps = calculate_mean(&throughputs_mbps);
    let peak_memory_mb = calculate_mean(&peak_memories_mb);
    let p95_memory_mb = calculate_mean(&p95_memories_mb);
    let p99_memory_mb = calculate_mean(&p99_memories_mb);
    let avg_cpu_percent = calculate_mean(&cpu_percents);

    AggregatedMetrics {
        count,
        successful,
        success_rate,
        mean_duration_ms,
        median_duration_ms,
        p95_duration_ms,
        p99_duration_ms,
        avg_throughput_mbps,
        peak_memory_mb,
        p95_memory_mb,
        p99_memory_mb,
        avg_cpu_percent,
    }
}

/// Convert Duration to milliseconds as f64
fn duration_to_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

/// Calculate mean of a slice of f64 values
fn calculate_mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

/// Calculate median of a slice of f64 values (modifies the slice)
fn calculate_median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    // Filter out NaN values and log warning if any found
    let valid_values: Vec<f64> = values.iter().copied().filter(|v| !v.is_nan()).collect();
    if valid_values.len() < values.len() {
        eprintln!(
            "Warning: {} NaN values filtered from median calculation",
            values.len() - valid_values.len()
        );
    }

    if valid_values.is_empty() {
        return 0.0;
    }

    let mut sorted_values = valid_values;
    sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let len = sorted_values.len();
    if len.is_multiple_of(2) {
        (sorted_values[len / 2 - 1] + sorted_values[len / 2]) / 2.0
    } else {
        sorted_values[len / 2]
    }
}

/// Generate complete HTML document with embedded charts
///
/// Uses MiniJinja templates to render the benchmark visualization.
/// The template approach provides better separation of concerns and easier maintenance.
fn generate_html(data: &ChartData) -> Result<String> {
    let env = get_template_env();
    let template = env
        .get_template("base.html.jinja")
        .map_err(|e| Error::Benchmark(format!("Template not found: {}", e)))?;
    let html = template
        .render(context! { data => data })
        .map_err(|e| Error::Benchmark(format!("Template render failed: {}", e)))?;
    Ok(html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration_to_ms() {
        let duration = Duration::from_millis(1500);
        assert_eq!(duration_to_ms(duration), 1500.0);
    }

    #[test]
    fn test_calculate_mean() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(calculate_mean(&values), 3.0);
    }

    #[test]
    fn test_calculate_median() {
        let mut values = vec![5.0, 1.0, 3.0, 2.0, 4.0];
        assert_eq!(calculate_median(&mut values), 3.0);

        let mut even_values = vec![4.0, 2.0, 3.0, 1.0];
        assert_eq!(calculate_median(&mut even_values), 2.5);
    }

    #[test]
    fn test_calculate_mean_empty() {
        let values: Vec<f64> = vec![];
        assert_eq!(calculate_mean(&values), 0.0);
    }

    #[test]
    fn test_calculate_median_empty() {
        let mut values: Vec<f64> = vec![];
        assert_eq!(calculate_median(&mut values), 0.0);
    }

    #[test]
    fn test_calculate_median_large_dataset() {
        let mut values: Vec<f64> = (0..1000).map(|i| i as f64).collect();
        let result = calculate_median(&mut values);
        assert_eq!(result, 499.5, "Median of 0-999 should be 499.5");
    }

    #[test]
    fn test_calculate_aggregated_metrics_empty() {
        let results: Vec<&BenchmarkResult> = vec![];
        let metrics = calculate_aggregated_metrics(&results);
        assert_eq!(metrics.count, 0);
        assert_eq!(metrics.successful, 0);
        assert_eq!(metrics.success_rate, 0.0);
        assert_eq!(metrics.mean_duration_ms, 0.0);
        assert_eq!(metrics.median_duration_ms, 0.0);
    }

    #[test]
    fn test_calculate_aggregated_metrics_single() {
        use crate::types::{DurationStatistics, FrameworkCapabilities, PerformanceMetrics};
        use std::path::PathBuf;

        let result = BenchmarkResult {
            framework: "test".to_string(),
            file_path: PathBuf::from("/tmp/test.pdf"),
            file_size: 1000,
            success: true,
            error_message: None,
            duration: Duration::from_millis(100),
            extraction_duration: None,
            subprocess_overhead: None,
            metrics: PerformanceMetrics {
                throughput_bytes_per_sec: 10000.0,
                peak_memory_bytes: 1_000_000,
                p50_memory_bytes: 750_000,
                p95_memory_bytes: 800_000,
                p99_memory_bytes: 900_000,
                avg_cpu_percent: 50.0,
            },
            quality: None,
            iterations: vec![],
            statistics: Some(DurationStatistics {
                mean: Duration::from_millis(100),
                median: Duration::from_millis(100),
                std_dev_ms: 0.0,
                min: Duration::from_millis(100),
                max: Duration::from_millis(100),
                p95: Duration::from_millis(120),
                p99: Duration::from_millis(150),
                sample_count: 1,
            }),
            cold_start_duration: None,
            file_extension: "pdf".to_string(),
            framework_capabilities: FrameworkCapabilities::default(),
            pdf_metadata: None,
        };

        let metrics = calculate_aggregated_metrics(&[&result]);
        assert_eq!(metrics.count, 1);
        assert_eq!(metrics.successful, 1);
        assert_eq!(metrics.success_rate, 1.0);
        assert_eq!(metrics.mean_duration_ms, 100.0);
    }

    #[test]
    fn test_build_chart_data() {
        use crate::types::{DurationStatistics, FrameworkCapabilities, PerformanceMetrics};
        use std::path::PathBuf;

        let result = BenchmarkResult {
            framework: "test-framework".to_string(),
            file_path: PathBuf::from("/tmp/test.pdf"),
            file_size: 1000,
            success: true,
            error_message: None,
            duration: Duration::from_millis(100),
            extraction_duration: None,
            subprocess_overhead: None,
            metrics: PerformanceMetrics {
                throughput_bytes_per_sec: 10000.0,
                peak_memory_bytes: 1_000_000,
                p50_memory_bytes: 750_000,
                p95_memory_bytes: 800_000,
                p99_memory_bytes: 900_000,
                avg_cpu_percent: 50.0,
            },
            quality: None,
            iterations: vec![],
            statistics: Some(DurationStatistics {
                mean: Duration::from_millis(100),
                median: Duration::from_millis(100),
                std_dev_ms: 0.0,
                min: Duration::from_millis(100),
                max: Duration::from_millis(100),
                p95: Duration::from_millis(120),
                p99: Duration::from_millis(150),
                sample_count: 1,
            }),
            cold_start_duration: None,
            file_extension: "pdf".to_string(),
            framework_capabilities: FrameworkCapabilities::default(),
            pdf_metadata: None,
        };

        let chart_data = build_chart_data(&[result], None).unwrap();
        assert_eq!(chart_data.frameworks.len(), 1);
        assert_eq!(chart_data.frameworks[0], "test-framework");
        assert_eq!(chart_data.extensions.len(), 1);
        assert_eq!(chart_data.extensions[0], "pdf");
        assert!(chart_data.framework_metrics.contains_key("test-framework"));
    }
}
