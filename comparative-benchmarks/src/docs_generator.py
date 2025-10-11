from __future__ import annotations

from datetime import datetime, timezone
from typing import TYPE_CHECKING, TypedDict

import polars as pl

if TYPE_CHECKING:
    from pathlib import Path


class DocConfig(TypedDict, total=False):
    title: str
    include_timestamp: bool
    include_methodology: bool
    charts_dir: Path | None


DEFAULT_DOC_CONFIG: DocConfig = {
    "title": "Kreuzberg Performance Benchmarks",
    "include_timestamp": True,
    "include_methodology": True,
    "charts_dir": None,
}

FRAMEWORK_EMOJI: dict[str, str] = {
    "kreuzberg": "🚀",
    "docling": "📄",
    "markitdown": "📝",
    "unstructured": "🔧",
    "extractous": "⚡",
}


def _format_number(value: float | None, decimals: int = 2) -> str:
    if value is None:
        return "N/A"
    return f"{value:.{decimals}f}"


def _format_percentage(value: float | None) -> str:
    if value is None:
        return "N/A"
    return f"{value * 100:.1f}%"


def _format_memory(value: float | None) -> str:
    if value is None:
        return "N/A"
    return f"{value:.1f} MB"


def _get_framework_emoji(framework: str) -> str:
    for key, emoji in FRAMEWORK_EMOJI.items():
        if key in framework.lower():
            return emoji
    return "📊"


def generate_index_page(
    summary_df: pl.DataFrame,
    output_path: Path,
    config: DocConfig | None = None,
) -> Path:
    cfg = {**DEFAULT_DOC_CONFIG, **(config or {})}
    charts_dir: Path | None = cfg.get("charts_dir")  # type: ignore[assignment]

    timestamp = (
        datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
        if cfg["include_timestamp"]
        else ""
    )

    best_speed = summary_df.sort("avg_extraction_time").head(1)
    best_memory = summary_df.sort("avg_peak_memory_mb").head(1)
    best_success = summary_df.sort("success_rate", descending=True).head(1)

    content = f"""# {cfg["title"]}

> Last updated: {timestamp} | [View Detailed Results →](latest-results.md)

## 🏆 Performance Leaders

| Metric | Winner | Score |
|--------|--------|-------|
| **Speed Champion** | {_get_framework_emoji(best_speed.get_column("framework").item())} {best_speed.get_column("framework").item()} | {_format_number(best_speed.get_column("avg_extraction_time").item())}s avg |
| **Memory Efficient** | {_get_framework_emoji(best_memory.get_column("framework").item())} {best_memory.get_column("framework").item()} | {_format_memory(best_memory.get_column("avg_peak_memory_mb").item())} |
| **Best Success Rate** | {_get_framework_emoji(best_success.get_column("framework").item())} {best_success.get_column("framework").item()} | {_format_percentage(best_success.get_column("success_rate").item())} |

## 📊 Latest Benchmark Run

"""

    if charts_dir and (charts_dir / "dashboard.html").exists():
        content += '<iframe src="charts/dashboard.html" width="100%" height="850" frameborder="0"></iframe>\n\n'

    content += f"""## Quick Stats

- **Total Files Tested**: {summary_df.get_column("total_files").sum():,}
- **Frameworks Tested**: {len(summary_df)}

## Framework Comparison

| Framework | Success Rate | Median Time | P95 Time | P99 Time | Avg Memory | Throughput |
|-----------|-------------|-------------|----------|----------|------------|------------|
"""

    for row in summary_df.sort("avg_extraction_time").iter_rows(named=True):
        emoji = _get_framework_emoji(row["framework"])
        content += f"| {emoji} **{row['framework']}** | "
        content += f"{_format_percentage(row['success_rate'])} | "
        content += f"{_format_number(row.get('median_extraction_time'))}s | "
        content += f"{_format_number(row.get('p95_extraction_time'))}s | "
        content += f"{_format_number(row.get('p99_extraction_time'))}s | "
        content += f"{_format_memory(row['avg_peak_memory_mb'])} | "
        content += f"{_format_number(row.get('files_per_second'), 2)} files/s |\n"

    content += """

## Navigation

- 📈 [**Performance Comparison** →](framework-comparison.md) - Head-to-head framework analysis
- 📊 [**Detailed Results** →](latest-results.md) - Complete benchmark data
- 🔬 [**Methodology** →](methodology.md) - How we benchmark

## Interactive Charts

- [Performance Comparison](charts/performance_comparison.html)
- [Memory Usage Analysis](charts/memory_usage.html)
- [Throughput Metrics](charts/throughput.html)
- [Time Distribution](charts/time_distribution.html)
- [Per-Format Heatmap](charts/format_heatmap.html)
"""

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(content)

    return output_path


def generate_detailed_results_page(
    summary_df: pl.DataFrame,
    by_format_df: pl.DataFrame | None,
    output_path: Path,
    config: DocConfig | None = None,
) -> Path:
    cfg = {**DEFAULT_DOC_CONFIG, **(config or {})}
    charts_dir: Path | None = cfg.get("charts_dir")  # type: ignore[assignment]

    timestamp = (
        datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
        if cfg["include_timestamp"]
        else ""
    )

    content = f"""# Latest Benchmark Results

> Generated: {timestamp}

## Summary Statistics

- **Total Files Processed**: {summary_df.get_column("total_files").sum():,}
- **Frameworks Tested**: {len(summary_df)}

## Performance by Framework

"""

    if charts_dir and (charts_dir / "performance_comparison.html").exists():
        content += '<iframe src="charts/performance_comparison.html" width="100%" height="550" frameborder="0"></iframe>\n\n'

    content += """### Detailed Metrics

| Framework | Files | Success | Failed | Timeout | Success Rate | Median | P95 | P99 | Avg Memory |
|-----------|-------|---------|--------|---------|--------------|--------|-----|-----|------------|
"""

    for row in summary_df.iter_rows(named=True):
        content += f"| **{row['framework']}** | "
        content += f"{row['total_files']} | "
        content += f"{row['successful_files']} | "
        content += f"{row['failed_files']} | "
        content += f"{row['timeout_files']} | "
        content += f"{_format_percentage(row['success_rate'])} | "
        content += f"{_format_number(row.get('median_extraction_time'))}s | "
        content += f"{_format_number(row.get('p95_extraction_time'))}s | "
        content += f"{_format_number(row.get('p99_extraction_time'))}s | "
        content += f"{_format_memory(row['avg_peak_memory_mb'])} |\n"

    if by_format_df is not None and len(by_format_df) > 0:
        content += """

## Performance by File Format

"""
        if charts_dir and (charts_dir / "format_heatmap.html").exists():
            content += '<iframe src="charts/format_heatmap.html" width="100%" height="550" frameborder="0"></iframe>\n\n'

        for file_type in by_format_df.get_column("file_type").unique().sort():
            format_data = by_format_df.filter(pl.col("file_type") == file_type)

            content += f"### {file_type.upper()} Files\n\n"
            content += (
                "| Framework | Files | Success Rate | Median Time | Avg Memory |\n"
            )
            content += (
                "|-----------|-------|--------------|-------------|------------|\n"
            )

            for row in format_data.iter_rows(named=True):
                content += f"| {row['framework']} | "
                content += f"{row['total_files']} | "
                content += f"{_format_percentage(row['success_rate'])} | "
                content += f"{_format_number(row.get('median_extraction_time'))}s | "
                content += f"{_format_memory(row['avg_peak_memory_mb'])} |\n"

            content += "\n"

    content += """

## Raw Data

- [Download CSV](data/latest.csv)
- [Download JSON](data/latest.json)
- [Download Parquet](data/latest.parquet)
"""

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(content)

    return output_path


def generate_methodology_page(output_path: Path) -> Path:
    content = """# Benchmark Methodology

## Overview

Our benchmarking process provides fair, reproducible, and comprehensive performance measurements across text extraction frameworks.

## Test Environment

### Hardware
- **Platform**: GitHub Actions Ubuntu Latest
- **CPU**: Variable (cloud environment)
- **Memory**: Variable (typically 7GB available)
- **Storage**: SSD

### Software
- **Python**: 3.11+
- **OS**: Ubuntu Linux
- **Dependencies**: Latest stable versions

## Frameworks Tested

| Framework | Description |
|-----------|-------------|
| **kreuzberg** | High-performance extraction library (this project) |
| **docling** | IBM's document understanding library |
| **markitdown** | Microsoft's markdown conversion tool |
| **unstructured** | Unstructured data processing library |
| **extractous** | Fast Rust-based extraction library |

## Test Corpus

### Document Categories

- **Tiny** (<100KB): Small text files, simple documents
- **Small** (100KB-1MB): Typical documents, reports
- **Medium** (1MB-10MB): Large documents, books
- **Large** (10MB-50MB): Very large documents
- **Huge** (>50MB): Extreme cases

### File Formats

- PDF documents (standard, scanned, complex)
- Office documents (DOCX, XLSX, PPTX)
- Images (PNG, JPEG, TIFF)
- Web formats (HTML, XML)
- Plain text and markdown
- Specialized formats (EPUB, RTF, etc.)

## Metrics Collected

### Performance Metrics
- **Extraction Time**: Wall-clock time for complete extraction
- **Median Time (P50)**: Middle value when sorted
- **P95 Time**: 95th percentile (tail latency)
- **P99 Time**: 99th percentile (worst-case performance)
- **Memory Usage**: Peak RSS during extraction
- **Throughput**: Files processed per second

### Quality Metrics
- **Character Count**: Total extracted characters
- **Word Count**: Total extracted words
- **Success Rate**: Percentage of successful extractions

### Reliability Metrics
- **Success Rate**: Percentage successful
- **Timeout Rate**: Files exceeding time limit
- **Error Categories**: Types of failures

## Test Execution

### Process
1. **Cache Clearing**: Framework caches cleared before each run
2. **Warm-up**: Initial extraction to eliminate cold-start effects
3. **Multiple Iterations**: Default 3 iterations per file
4. **Isolation**: Each framework tested separately
5. **Timeout Protection**: Configurable timeout per file

### Resource Monitoring
- CPU and memory sampled at 100ms intervals
- Process-level monitoring using psutil
- Subprocess isolation for crash protection

## Statistical Analysis

### Central Tendency
- **Mean**: Average across all iterations
- **Median (P50)**: Middle value (more robust than mean)
- **P95/P99**: Tail latency percentiles

### Variability
- **Standard Deviation**: Measure of result spread
- **Min/Max**: Range of observed values

## Reproducibility

### Version Control
- All benchmark code version controlled
- Framework versions pinned in CI
- Test document set versioned

### CI/CD Integration
- Automated execution via GitHub Actions
- Results archived as artifacts
- Historical data preserved

## Limitations

- **Cloud Environment**: Performance varies with cloud instance
- **Single-threaded**: Testing one file at a time
- **Format Support**: Frameworks tested only on supported formats

## Running Benchmarks

### Local Execution
```bash
cd benchmarks
uv run python -m src.cli benchmark --framework all --iterations 3
uv run python -m src.cli visualize
uv run python -m src.cli generate-docs
```

### CI Execution
Triggered via GitHub Actions workflow dispatch with configurable parameters.
"""

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(content)

    return output_path


def generate_framework_comparison_page(
    summary_df: pl.DataFrame,
    output_path: Path,
    config: DocConfig | None = None,
) -> Path:
    cfg = {**DEFAULT_DOC_CONFIG, **(config or {})}
    charts_dir: Path | None = cfg.get("charts_dir")  # type: ignore[assignment]

    content = """# Framework Comparison

> Head-to-head comparison of text extraction frameworks

## Performance Overview

"""

    if charts_dir and (charts_dir / "dashboard.html").exists():
        content += '<iframe src="charts/dashboard.html" width="100%" height="850" frameborder="0"></iframe>\n\n'

    content += """## Detailed Comparison

| Framework | Speed | Memory | Reliability | Best For |
|-----------|-------|--------|-------------|----------|
"""

    for row in summary_df.sort("avg_extraction_time").iter_rows(named=True):
        emoji = _get_framework_emoji(row["framework"])

        speed_rating = (
            "⚡⚡⚡"
            if row["avg_extraction_time"] < 0.5
            else "⚡⚡"
            if row["avg_extraction_time"] < 2
            else "⚡"
        )
        memory_rating = (
            "💾💾💾"
            if row["avg_peak_memory_mb"] < 500
            else "💾💾"
            if row["avg_peak_memory_mb"] < 1500
            else "💾"
        )
        reliability_rating = (
            "✅✅✅"
            if row["success_rate"] > 0.98
            else "✅✅"
            if row["success_rate"] > 0.95
            else "✅"
        )

        best_for = (
            "Speed & efficiency"
            if row["avg_extraction_time"] < 1 and row["avg_peak_memory_mb"] < 600
            else "Accuracy"
            if row["success_rate"] > 0.99
            else "General use"
        )

        content += f"| {emoji} **{row['framework']}** | "
        content += f"{speed_rating} | "
        content += f"{memory_rating} | "
        content += f"{reliability_rating} | "
        content += f"{best_for} |\n"

    content += """

## Recommendations

### For Speed-Critical Applications
Choose frameworks with median extraction time < 1s and high throughput.

### For Memory-Constrained Environments
Select frameworks with average peak memory < 500MB.

### For Production Reliability
Prioritize frameworks with success rate > 98%.

### For Large-Scale Processing
Consider throughput (files/second) and memory efficiency together.

## Additional Resources

- [Detailed Results](latest-results.md)
- [Benchmark Methodology](methodology.md)
- [Raw Data Downloads](latest-results.md#raw-data)
"""

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(content)

    return output_path
