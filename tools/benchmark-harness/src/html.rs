//! HTML visualization output for benchmark results
//!
//! This module generates static HTML pages with embedded Chart.js visualizations
//! for benchmark results. The output is a single self-contained HTML file that
//! can be viewed in any browser without external dependencies (except Chart.js CDN).

use crate::types::BenchmarkResult;
use crate::{Error, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
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
fn generate_html(data: &ChartData) -> Result<String> {
    let data_json = serde_json::to_string_pretty(data)
        .map_err(|e| Error::Benchmark(format!("Failed to serialize chart data: {}", e)))?;

    let charts_js = generate_charts_js(data)?;
    let css = generate_css();
    let success_rate = format!("{:.1}", calculate_overall_success_rate(data) * 100.0);

    // Format metadata section with benchmark run date if available
    let metadata = if let Some(benchmark_date) = &data.benchmark_run_date {
        format!(
            "Benchmark Run: {}<br>Visualization Generated: {}<br>Frameworks: {} | Extensions: {}",
            benchmark_date,
            data.generated_at,
            data.frameworks.len(),
            data.extensions.len()
        )
    } else {
        format!(
            "Generated: {} | Frameworks: {} | Extensions: {}",
            data.generated_at,
            data.frameworks.len(),
            data.extensions.len()
        )
    };

    let html = format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n    <meta charset=\"UTF-8\">\n    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n    <title>Kreuzberg Benchmark Results</title>\n    <script src=\"https://cdn.jsdelivr.net/npm/chart.js@4.4.0/dist/chart.umd.min.js\"></script>\n    <style>\n{}\n    </style>\n</head>\n<body>\n    <div class=\"container\">\n        <header>\n            <h1>Kreuzberg Benchmark Results</h1>\n            <p class=\"metadata\">{}</p>\n        </header>\n\n        <nav class=\"tabs\">\n            <button class=\"tab-button active\" data-tab=\"duration\">Duration</button>\n            <button class=\"tab-button\" data-tab=\"throughput\">Throughput</button>\n            <button class=\"tab-button\" data-tab=\"memory\">Memory</button>\n            <button class=\"tab-button\" data-tab=\"filetype\">File Types</button>\n            <button class=\"tab-button\" data-tab=\"success\">Success Rates</button>\n        </nav>\n\n        <section id=\"duration\" class=\"tab-content active\">\n            <h2>Duration Comparison</h2>\n            <p>Average latency across all file types (lower is better)</p>\n            <canvas id=\"duration-chart\"></canvas>\n        </section>\n\n        <section id=\"throughput\" class=\"tab-content\">\n            <h2>Throughput Comparison</h2>\n            <p>Processing speed in MB/s (higher is better)</p>\n            <canvas id=\"throughput-chart\"></canvas>\n        </section>\n\n        <section id=\"memory\" class=\"tab-content\">\n            <h2>Memory Usage Analysis</h2>\n            <p>Memory consumption metrics in MB</p>\n            <canvas id=\"memory-chart\"></canvas>\n        </section>\n\n        <section id=\"filetype\" class=\"tab-content\">\n            <h2>Performance by File Type</h2>\n            <p>Duration comparison across document formats</p>\n            <canvas id=\"filetype-chart\"></canvas>\n        </section>\n\n        <section id=\"success\" class=\"tab-content\">\n            <h2>Success Rate Dashboard</h2>\n            <div class=\"success-summary\">\n                <div class=\"summary-card\">\n                    <h3>Total Files</h3>\n                    <p class=\"metric\">{}</p>\n                </div>\n                <div class=\"summary-card\">\n                    <h3>Overall Success Rate</h3>\n                    <p class=\"metric\">{}%</p>\n                </div>\n            </div>\n            <canvas id=\"success-chart\"></canvas>\n        </section>\n    </div>\n\n    <script>\n        const benchmarkData = {};\n\n{}\n    </script>\n</body>\n</html>",
        css,
        metadata,
        calculate_max_files_per_framework(data),
        success_rate,
        data_json,
        charts_js,
    );

    Ok(html)
}

/// Generate CSS styles for the HTML
fn generate_css() -> String {
    r#"
* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
    background: #f5f7fa;
    color: #2d3748;
    line-height: 1.6;
}

.container {
    max-width: 1400px;
    margin: 0 auto;
    padding: 2rem;
}

header {
    text-align: center;
    margin-bottom: 3rem;
    padding: 2rem;
    background: white;
    border-radius: 8px;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

header h1 {
    font-size: 2.5rem;
    color: #1a202c;
    margin-bottom: 0.5rem;
}

.metadata {
    color: #718096;
    font-size: 0.9rem;
}

.tabs {
    display: flex;
    gap: 1rem;
    margin-bottom: 2rem;
    flex-wrap: wrap;
}

.tab-button {
    padding: 0.75rem 1.5rem;
    background: white;
    border: 2px solid #e2e8f0;
    border-radius: 6px;
    cursor: pointer;
    font-size: 1rem;
    font-weight: 500;
    color: #4a5568;
    transition: all 0.2s;
}

.tab-button:hover {
    border-color: #4299e1;
    color: #2b6cb0;
}

.tab-button.active {
    background: #4299e1;
    border-color: #4299e1;
    color: white;
}

.tab-content {
    display: none;
    padding: 2rem;
    background: white;
    border-radius: 8px;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

.tab-content.active {
    display: block;
}

.tab-content h2 {
    font-size: 1.75rem;
    margin-bottom: 0.5rem;
    color: #1a202c;
}

.tab-content p {
    color: #718096;
    margin-bottom: 1.5rem;
}

canvas {
    max-height: 500px;
    margin-top: 1rem;
}

.success-summary {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1.5rem;
    margin-bottom: 2rem;
}

.summary-card {
    padding: 1.5rem;
    background: #f7fafc;
    border-radius: 6px;
    text-align: center;
}

.summary-card h3 {
    font-size: 0.875rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #718096;
    margin-bottom: 0.5rem;
}

.summary-card .metric {
    font-size: 2rem;
    font-weight: 700;
    color: #2d3748;
}

@media (max-width: 768px) {
    .container {
        padding: 1rem;
    }

    header h1 {
        font-size: 1.75rem;
    }

    .tabs {
        gap: 0.5rem;
    }

    .tab-button {
        padding: 0.5rem 1rem;
        font-size: 0.875rem;
    }

    canvas {
        max-height: 300px;
    }
}
"#
    .to_string()
}

/// Generate JavaScript code for charts
fn generate_charts_js(data: &ChartData) -> Result<String> {
    let duration_chart = generate_duration_chart_js(data)?;
    let throughput_chart = generate_throughput_chart_js(data)?;
    let memory_chart = generate_memory_chart_js(data)?;
    let filetype_chart = generate_filetype_chart_js(data)?;
    let success_chart = generate_success_chart_js(data)?;

    Ok(format!(
        r#"
// Tab switching functionality
const tabButtons = document.querySelectorAll('.tab-button');
const tabContents = document.querySelectorAll('.tab-content');

tabButtons.forEach(button => {{
    button.addEventListener('click', () => {{
        const tabName = button.getAttribute('data-tab');

        tabButtons.forEach(btn => btn.classList.remove('active'));
        tabContents.forEach(content => content.classList.remove('active'));

        button.classList.add('active');
        document.getElementById(tabName).classList.add('active');
    }});
}});

// Chart.js default configuration
Chart.defaults.font.family = "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif";
Chart.defaults.color = '#4a5568';

// Color palette for frameworks
const colors = [
    '#4299e1', '#48bb78', '#ed8936', '#9f7aea', '#f56565',
    '#38b2ac', '#ecc94b', '#ed64a6', '#667eea', '#fc8181'
];

// Duration Chart
{duration_chart}

// Throughput Chart
{throughput_chart}

// Memory Chart
{memory_chart}

// File Type Chart
{filetype_chart}

// Success Rate Chart
{success_chart}
"#,
        duration_chart = duration_chart,
        throughput_chart = throughput_chart,
        memory_chart = memory_chart,
        filetype_chart = filetype_chart,
        success_chart = success_chart,
    ))
}

/// Generate duration chart JavaScript
fn generate_duration_chart_js(data: &ChartData) -> Result<String> {
    let p95_data = extract_framework_metric(data, |m| m.p95_duration_ms);
    let p50_data = extract_framework_metric(data, |m| m.median_duration_ms);

    Ok(format!(
        r#"new Chart(document.getElementById('duration-chart'), {{
    type: 'bar',
    data: {{
        labels: {},
        datasets: [
            {{
                label: 'p95 Duration (ms)',
                data: {},
                backgroundColor: 'rgba(66, 153, 225, 0.8)',
                borderColor: 'rgba(66, 153, 225, 1)',
                borderWidth: 1
            }},
            {{
                label: 'p50 Duration (ms)',
                data: {},
                backgroundColor: 'rgba(72, 187, 120, 0.8)',
                borderColor: 'rgba(72, 187, 120, 1)',
                borderWidth: 1
            }}
        ]
    }},
    options: {{
        responsive: true,
        maintainAspectRatio: true,
        plugins: {{
            title: {{
                display: true,
                text: 'Processing Duration Comparison (Lower is Better)'
            }},
            legend: {{
                position: 'bottom'
            }},
            tooltip: {{
                callbacks: {{
                    label: (context) => {{
                        return context.dataset.label + ': ' + context.parsed.y.toFixed(2) + ' ms';
                    }}
                }}
            }}
        }},
        scales: {{
            y: {{
                beginAtZero: true,
                title: {{
                    display: true,
                    text: 'Duration (ms)'
                }}
            }}
        }}
    }}
}});"#,
        serialize_json(&data.frameworks)?,
        serialize_json(&p95_data)?,
        serialize_json(&p50_data)?,
    ))
}

/// Generate throughput chart JavaScript
fn generate_throughput_chart_js(data: &ChartData) -> Result<String> {
    let throughput_data = extract_framework_metric(data, |m| m.avg_throughput_mbps);

    Ok(format!(
        r#"new Chart(document.getElementById('throughput-chart'), {{
    type: 'bar',
    data: {{
        labels: {},
        datasets: [
            {{
                label: 'Throughput (MB/s)',
                data: {},
                backgroundColor: 'rgba(237, 137, 54, 0.8)',
                borderColor: 'rgba(237, 137, 54, 1)',
                borderWidth: 1
            }}
        ]
    }},
    options: {{
        responsive: true,
        maintainAspectRatio: true,
        indexAxis: 'y',
        plugins: {{
            title: {{
                display: true,
                text: 'Throughput Comparison (Higher is Better)'
            }},
            legend: {{
                display: false
            }},
            tooltip: {{
                callbacks: {{
                    label: (context) => {{
                        return 'Throughput: ' + context.parsed.x.toFixed(2) + ' MB/s';
                    }}
                }}
            }}
        }},
        scales: {{
            x: {{
                beginAtZero: true,
                title: {{
                    display: true,
                    text: 'Throughput (MB/s)'
                }}
            }}
        }}
    }}
}});"#,
        serialize_json(&data.frameworks)?,
        serialize_json(&throughput_data)?,
    ))
}

/// Generate memory chart JavaScript
fn generate_memory_chart_js(data: &ChartData) -> Result<String> {
    let peak_data = extract_framework_metric(data, |m| m.peak_memory_mb);
    let p95_data = extract_framework_metric(data, |m| m.p95_memory_mb);
    let p99_data = extract_framework_metric(data, |m| m.p99_memory_mb);

    Ok(format!(
        r#"new Chart(document.getElementById('memory-chart'), {{
    type: 'bar',
    data: {{
        labels: {},
        datasets: [
            {{
                label: 'Peak Memory (MB)',
                data: {},
                backgroundColor: 'rgba(159, 122, 234, 0.8)',
                borderColor: 'rgba(159, 122, 234, 1)',
                borderWidth: 1
            }},
            {{
                label: 'p95 Memory (MB)',
                data: {},
                backgroundColor: 'rgba(56, 178, 172, 0.8)',
                borderColor: 'rgba(56, 178, 172, 1)',
                borderWidth: 1
            }},
            {{
                label: 'p99 Memory (MB)',
                data: {},
                backgroundColor: 'rgba(236, 201, 75, 0.8)',
                borderColor: 'rgba(236, 201, 75, 1)',
                borderWidth: 1
            }}
        ]
    }},
    options: {{
        responsive: true,
        maintainAspectRatio: true,
        plugins: {{
            title: {{
                display: true,
                text: 'Memory Usage Analysis (Lower is Better)'
            }},
            legend: {{
                position: 'bottom'
            }},
            tooltip: {{
                callbacks: {{
                    label: (context) => {{
                        return context.dataset.label + ': ' + context.parsed.y.toFixed(2) + ' MB';
                    }}
                }}
            }}
        }},
        scales: {{
            y: {{
                beginAtZero: true,
                title: {{
                    display: true,
                    text: 'Memory (MB)'
                }}
            }}
        }}
    }}
}});"#,
        serialize_json(&data.frameworks)?,
        serialize_json(&peak_data)?,
        serialize_json(&p95_data)?,
        serialize_json(&p99_data)?,
    ))
}

/// Generate file type chart JavaScript
fn generate_filetype_chart_js(data: &ChartData) -> Result<String> {
    let mut datasets: Vec<String> = Vec::new();
    for (i, framework) in data.frameworks.iter().enumerate() {
        let data_points: Vec<f64> = data
            .extensions
            .iter()
            .map(|ext| {
                data.extension_metrics
                    .get(ext)
                    .and_then(|fm| fm.get(framework))
                    .map(|m| m.p95_duration_ms)
                    .unwrap_or(0.0)
            })
            .collect();

        let color_idx = i % 10;
        let dataset_json = serialize_json(&data_points)?;
        let dataset = format!(
            r#"{{
                label: '{}',
                data: {},
                backgroundColor: colors[{}],
                borderWidth: 1
            }}"#,
            framework, dataset_json, color_idx
        );
        datasets.push(dataset);
    }

    let extensions_json = serialize_json(&data.extensions)?;

    Ok(format!(
        r#"new Chart(document.getElementById('filetype-chart'), {{
    type: 'bar',
    data: {{
        labels: {},
        datasets: [
            {}
        ]
    }},
    options: {{
        responsive: true,
        maintainAspectRatio: true,
        plugins: {{
            title: {{
                display: true,
                text: 'Duration by File Type (p95, ms)'
            }},
            legend: {{
                position: 'bottom',
                labels: {{
                    boxWidth: 12
                }}
            }},
            tooltip: {{
                callbacks: {{
                    label: (context) => {{
                        return context.dataset.label + ': ' + context.parsed.y.toFixed(2) + ' ms';
                    }}
                }}
            }}
        }},
        scales: {{
            y: {{
                beginAtZero: true,
                title: {{
                    display: true,
                    text: 'Duration (ms)'
                }}
            }}
        }}
    }}
}});"#,
        extensions_json,
        datasets.join(",\n            ")
    ))
}

/// Generate success rate chart JavaScript
fn generate_success_chart_js(data: &ChartData) -> Result<String> {
    let success_rates = extract_framework_metric(data, |m| m.success_rate * 100.0);

    Ok(format!(
        r#"new Chart(document.getElementById('success-chart'), {{
    type: 'bar',
    data: {{
        labels: {},
        datasets: [
            {{
                label: 'Success Rate (%)',
                data: {},
                backgroundColor: 'rgba(72, 187, 120, 0.8)',
                borderColor: 'rgba(72, 187, 120, 1)',
                borderWidth: 1
            }}
        ]
    }},
    options: {{
        responsive: true,
        maintainAspectRatio: true,
        plugins: {{
            title: {{
                display: true,
                text: 'Framework Success Rates'
            }},
            legend: {{
                display: false
            }},
            tooltip: {{
                callbacks: {{
                    label: (context) => {{
                        return 'Success Rate: ' + context.parsed.y.toFixed(1) + '%';
                    }}
                }}
            }}
        }},
        scales: {{
            y: {{
                beginAtZero: true,
                max: 100,
                title: {{
                    display: true,
                    text: 'Success Rate (%)'
                }}
            }}
        }}
    }}
}});"#,
        serialize_json(&data.frameworks)?,
        serialize_json(&success_rates)?,
    ))
}

/// Calculate maximum number of files processed by any framework
fn calculate_max_files_per_framework(data: &ChartData) -> usize {
    data.framework_metrics.values().map(|m| m.count).max().unwrap_or(0)
}

/// Calculate overall success rate across all frameworks
fn calculate_overall_success_rate(data: &ChartData) -> f64 {
    let total_count: usize = data.framework_metrics.values().map(|m| m.count).sum();
    let total_successful: usize = data.framework_metrics.values().map(|m| m.successful).sum();

    if total_count > 0 {
        total_successful as f64 / total_count as f64
    } else {
        0.0
    }
}

/// Serialize data to JSON string
///
/// Returns an error if serialization fails.
fn serialize_json<T: Serialize>(data: &T) -> Result<String> {
    serde_json::to_string(data).map_err(|e| Error::Benchmark(format!("Failed to serialize data to JSON: {}", e)))
}

/// Extract framework metrics using provided accessor function
///
/// This helper function reduces code duplication across chart generation functions
/// by applying a consistent pattern for extracting metrics by framework.
fn extract_framework_metric<F>(data: &ChartData, accessor: F) -> Vec<f64>
where
    F: Fn(&AggregatedMetrics) -> f64,
{
    data.frameworks
        .iter()
        .map(|fw| data.framework_metrics.get(fw).map(&accessor).unwrap_or(0.0))
        .collect()
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
    fn test_extract_framework_metric() {
        let mut data = ChartData {
            frameworks: vec!["fw1".to_string(), "fw2".to_string()],
            extensions: vec![],
            framework_metrics: HashMap::new(),
            extension_metrics: HashMap::new(),
            benchmark_run_date: None,
            generated_at: "2025-12-13".to_string(),
        };

        data.framework_metrics.insert(
            "fw1".to_string(),
            AggregatedMetrics {
                count: 10,
                successful: 10,
                success_rate: 1.0,
                mean_duration_ms: 100.0,
                median_duration_ms: 95.0,
                p95_duration_ms: 150.0,
                p99_duration_ms: Some(200.0),
                avg_throughput_mbps: 50.0,
                peak_memory_mb: 512.0,
                p95_memory_mb: 400.0,
                p99_memory_mb: 450.0,
                avg_cpu_percent: 75.0,
            },
        );

        data.framework_metrics.insert(
            "fw2".to_string(),
            AggregatedMetrics {
                count: 5,
                successful: 5,
                success_rate: 1.0,
                mean_duration_ms: 120.0,
                median_duration_ms: 110.0,
                p95_duration_ms: 180.0,
                p99_duration_ms: Some(220.0),
                avg_throughput_mbps: 45.0,
                peak_memory_mb: 600.0,
                p95_memory_mb: 500.0,
                p99_memory_mb: 550.0,
                avg_cpu_percent: 80.0,
            },
        );

        let p95_data = extract_framework_metric(&data, |m| m.p95_duration_ms);
        assert_eq!(p95_data, vec![150.0, 180.0]);

        let throughput_data = extract_framework_metric(&data, |m| m.avg_throughput_mbps);
        assert_eq!(throughput_data, vec![50.0, 45.0]);
    }

    #[test]
    fn test_extract_framework_metric_missing_framework() {
        let data = ChartData {
            frameworks: vec!["fw1".to_string(), "fw2".to_string()],
            extensions: vec![],
            framework_metrics: HashMap::new(),
            extension_metrics: HashMap::new(),
            benchmark_run_date: None,
            generated_at: "2025-12-13".to_string(),
        };

        let p95_data = extract_framework_metric(&data, |m| m.p95_duration_ms);
        assert_eq!(p95_data, vec![0.0, 0.0]);
    }

    #[test]
    fn test_serialize_json_vec_string() {
        let data = vec!["test1".to_string(), "test2".to_string()];
        let json = serialize_json(&data).unwrap();
        assert_eq!(json, r#"["test1","test2"]"#);
    }

    #[test]
    fn test_serialize_json_vec_f64() {
        let data = vec![1.5, 2.5, 3.5];
        let json = serialize_json(&data).unwrap();
        assert_eq!(json, "[1.5,2.5,3.5]");
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
    fn test_write_html_integration() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test.html");

        // Create sample benchmark result
        let results = vec![BenchmarkResult {
            framework: "kreuzberg-native".to_string(),
            file_path: "/tmp/test.pdf".into(),
            file_size: 1000,
            success: true,
            error_message: None,
            duration: Duration::new(0, 100_000_000), // 100ms
            extraction_duration: None,
            subprocess_overhead: None,
            metrics: crate::types::PerformanceMetrics {
                throughput_bytes_per_sec: 10000.0,
                peak_memory_bytes: 10_000_000,
                p50_memory_bytes: 5_000_000,
                p95_memory_bytes: 8_000_000,
                p99_memory_bytes: 9_000_000,
                avg_cpu_percent: 50.0,
            },
            quality: None,
            iterations: vec![],
            statistics: Some(crate::types::DurationStatistics {
                mean: Duration::from_millis(100),
                median: Duration::from_millis(100),
                std_dev_ms: 5.0,
                min: Duration::from_millis(95),
                max: Duration::from_millis(105),
                p95: Duration::from_millis(102),
                p99: Duration::from_millis(103),
                sample_count: 10,
            }),
            cold_start_duration: None,
            file_extension: "pdf".to_string(),
            framework_capabilities: crate::types::FrameworkCapabilities::default(),
            pdf_metadata: None,
        }];

        // Write HTML
        let result = write_html(&results, &output_path, None);
        assert!(result.is_ok(), "write_html should succeed");
        assert!(output_path.exists(), "HTML file should exist");

        // Verify HTML content
        let html = fs::read_to_string(&output_path).unwrap();
        assert!(html.contains("Kreuzberg Benchmark Results"), "Should contain title");
        assert!(html.contains("Chart.js"), "Should include Chart.js");
        assert!(html.contains("benchmarkData"), "Should embed benchmark data");
        assert!(html.contains("kreuzberg-native"), "Should include framework name");
    }
}
