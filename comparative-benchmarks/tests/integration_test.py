from __future__ import annotations

import json
from typing import TYPE_CHECKING

from click.testing import CliRunner

if TYPE_CHECKING:
    from pathlib import Path
from src.cli import cli


def test_full_pipeline_with_visualize_and_docs(tmp_path: Path) -> None:
    runner = CliRunner()

    results_data = [
        {
            "file_path": "/test/doc1.pdf",
            "file_size": 1024,
            "file_type": "pdf",
            "category": "small",
            "framework": "kreuzberg_v4_sync",
            "iteration": 1,
            "extraction_time": 0.5,
            "peak_memory_mb": 100.0,
            "avg_memory_mb": 80.0,
            "peak_cpu_percent": 50.0,
            "avg_cpu_percent": 40.0,
            "status": "success",
            "character_count": 500,
            "word_count": 100,
            "platform": "linux",
        },
        {
            "file_path": "/test/doc2.pdf",
            "file_size": 2048,
            "file_type": "pdf",
            "category": "small",
            "framework": "kreuzberg_v4_sync",
            "iteration": 1,
            "extraction_time": 0.8,
            "peak_memory_mb": 120.0,
            "avg_memory_mb": 90.0,
            "peak_cpu_percent": 55.0,
            "avg_cpu_percent": 45.0,
            "status": "success",
            "character_count": 800,
            "word_count": 150,
            "platform": "linux",
        },
        {
            "file_path": "/test/doc1.docx",
            "file_size": 512,
            "file_type": "docx",
            "category": "tiny",
            "framework": "kreuzberg_v4_sync",
            "iteration": 1,
            "extraction_time": 0.3,
            "peak_memory_mb": 80.0,
            "avg_memory_mb": 60.0,
            "peak_cpu_percent": 40.0,
            "avg_cpu_percent": 30.0,
            "status": "success",
            "character_count": 300,
            "word_count": 60,
            "platform": "linux",
        },
    ]

    results_file = tmp_path / "results.json"
    results_file.write_text(json.dumps(results_data))

    charts_dir = tmp_path / "charts"
    docs_dir = tmp_path / "docs"

    result = runner.invoke(
        cli,
        [
            "visualize",
            "--input",
            str(results_file),
            "--output-dir",
            str(charts_dir),
        ],
    )

    assert result.exit_code == 0, f"Visualize failed: {result.output}"
    assert "Generated 6 visualizations" in result.output

    assert (charts_dir / "performance_comparison.html").exists()
    assert (charts_dir / "memory_usage.html").exists()
    assert (charts_dir / "throughput.html").exists()
    assert (charts_dir / "time_distribution.html").exists()
    assert (charts_dir / "dashboard.html").exists()
    assert (charts_dir / "format_heatmap.html").exists()

    result = runner.invoke(
        cli,
        [
            "generate-docs",
            "--input",
            str(results_file),
            "--output-dir",
            str(docs_dir),
            "--charts-dir",
            str(charts_dir),
        ],
    )

    assert result.exit_code == 0, f"Generate-docs failed: {result.output}"
    assert "Generated documentation" in result.output

    assert (docs_dir / "index.md").exists()
    assert (docs_dir / "latest-results.md").exists()
    assert (docs_dir / "methodology.md").exists()
    assert (docs_dir / "framework-comparison.md").exists()

    assert (docs_dir / "data" / "latest.csv").exists()
    assert (docs_dir / "data" / "latest.json").exists()
    assert (docs_dir / "data" / "latest.parquet").exists()

    index_content = (docs_dir / "index.md").read_text()
    assert "Performance Leaders" in index_content
    assert "kreuzberg_v4_sync" in index_content


def test_visualize_command_creates_all_charts(tmp_path: Path) -> None:
    runner = CliRunner()

    results_data = [
        {
            "file_path": "/test/doc.pdf",
            "file_size": 1024,
            "file_type": "pdf",
            "category": "small",
            "framework": "kreuzberg_v4_sync",
            "iteration": 1,
            "extraction_time": 0.5,
            "peak_memory_mb": 100.0,
            "avg_memory_mb": 80.0,
            "peak_cpu_percent": 50.0,
            "avg_cpu_percent": 40.0,
            "status": "success",
            "platform": "linux",
        }
    ]

    results_file = tmp_path / "results.json"
    results_file.write_text(json.dumps(results_data))

    charts_dir = tmp_path / "charts"

    result = runner.invoke(
        cli,
        [
            "visualize",
            "--input",
            str(results_file),
            "--output-dir",
            str(charts_dir),
        ],
    )

    assert result.exit_code == 0

    expected_charts = [
        "performance_comparison.html",
        "memory_usage.html",
        "throughput.html",
        "time_distribution.html",
        "dashboard.html",
        "format_heatmap.html",
    ]

    for chart in expected_charts:
        chart_path = charts_dir / chart
        assert chart_path.exists(), f"Chart {chart} not created"
        assert chart_path.stat().st_size > 0, f"Chart {chart} is empty"


def test_generate_docs_command_creates_all_files(tmp_path: Path) -> None:
    runner = CliRunner()

    results_data = [
        {
            "file_path": "/test/doc.pdf",
            "file_size": 1024,
            "file_type": "pdf",
            "category": "small",
            "framework": "kreuzberg_v4_sync",
            "iteration": 1,
            "extraction_time": 0.5,
            "peak_memory_mb": 100.0,
            "avg_memory_mb": 80.0,
            "peak_cpu_percent": 50.0,
            "avg_cpu_percent": 40.0,
            "status": "success",
            "platform": "linux",
        }
    ]

    results_file = tmp_path / "results.json"
    results_file.write_text(json.dumps(results_data))

    docs_dir = tmp_path / "docs"

    result = runner.invoke(
        cli,
        [
            "generate-docs",
            "--input",
            str(results_file),
            "--output-dir",
            str(docs_dir),
        ],
    )

    assert result.exit_code == 0

    expected_docs = [
        "index.md",
        "latest-results.md",
        "methodology.md",
        "framework-comparison.md",
    ]

    for doc in expected_docs:
        doc_path = docs_dir / doc
        assert doc_path.exists(), f"Doc {doc} not created"
        assert doc_path.stat().st_size > 0, f"Doc {doc} is empty"
