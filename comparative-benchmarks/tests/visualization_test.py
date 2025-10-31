from __future__ import annotations

from typing import TYPE_CHECKING, cast

import polars as pl

if TYPE_CHECKING:
    from pathlib import Path
from src.visualization import (
    VisualizationConfig,
    create_interactive_dashboard,
    create_memory_usage_chart,
    create_per_format_heatmap,
    create_performance_comparison_chart,
    create_throughput_chart,
    create_time_distribution_chart,
)


def test_create_performance_comparison_chart_creates_file(tmp_path: Path) -> None:
    df = pl.DataFrame(
        {
            "framework": ["kreuzberg_v4_sync", "extractous"],
            "avg_extraction_time": [0.5, 0.8],
            "success_rate": [0.95, 0.90],
            "total_files": [100, 100],
        }
    )

    output_path = tmp_path / "performance.html"
    result_path = create_performance_comparison_chart(df, output_path)

    assert result_path.exists()
    assert result_path == output_path
    assert result_path.stat().st_size > 0


def test_create_performance_comparison_chart_with_single_row(tmp_path: Path) -> None:
    df = pl.DataFrame(
        {
            "framework": ["kreuzberg_v4_sync"],
            "avg_extraction_time": [0.5],
            "success_rate": [0.95],
            "total_files": [100],
        }
    )

    output_path = tmp_path / "performance_single.html"
    result_path = create_performance_comparison_chart(df, output_path)

    assert result_path.exists()


def test_create_performance_comparison_chart_with_custom_config(tmp_path: Path) -> None:
    df = pl.DataFrame(
        {
            "framework": ["kreuzberg_v4_sync"],
            "avg_extraction_time": [0.5],
            "success_rate": [0.95],
            "total_files": [100],
        }
    )

    output_path = tmp_path / "performance_custom.html"
    config = cast(
        "VisualizationConfig", {"width": 1600, "height": 900, "template": "plotly"}
    )
    result_path = create_performance_comparison_chart(df, output_path, config)

    assert result_path.exists()
    content = result_path.read_text()
    assert '"width":1600' in content
    assert '"height":900' in content


def test_create_memory_usage_chart_creates_file(tmp_path: Path) -> None:
    df = pl.DataFrame(
        {
            "framework": ["kreuzberg_v4_sync", "extractous"],
            "avg_peak_memory_mb": [100.0, 150.0],
            "total_files": [100, 100],
        }
    )

    output_path = tmp_path / "memory.html"
    result_path = create_memory_usage_chart(df, output_path)

    assert result_path.exists()
    assert result_path == output_path
    assert result_path.stat().st_size > 0


def test_create_throughput_chart_creates_file(tmp_path: Path) -> None:
    df = pl.DataFrame(
        {
            "framework": ["kreuzberg_v4_sync", "extractous"],
            "files_per_second": [2.5, 1.8],
            "mb_per_second": [5.0, 3.6],
        }
    )

    output_path = tmp_path / "throughput.html"
    result_path = create_throughput_chart(df, output_path)

    assert result_path.exists()
    assert result_path == output_path


def test_create_throughput_chart_handles_missing_columns(tmp_path: Path) -> None:
    df = pl.DataFrame(
        {
            "framework": ["kreuzberg_v4_sync"],
            "files_per_second": [2.5],
        }
    )

    output_path = tmp_path / "throughput_partial.html"
    result_path = create_throughput_chart(df, output_path)

    assert result_path.exists()


def test_create_time_distribution_chart_creates_file(tmp_path: Path) -> None:
    df = pl.DataFrame(
        {
            "framework": [
                "kreuzberg_v4_sync",
                "kreuzberg_v4_sync",
                "extractous",
                "extractous",
            ],
            "extraction_time": [0.5, 0.6, 0.8, 0.9],
            "file_path": ["/test/1.pdf", "/test/2.pdf", "/test/3.pdf", "/test/4.pdf"],
        }
    )

    output_path = tmp_path / "distribution.html"
    result_path = create_time_distribution_chart(df, output_path)

    assert result_path.exists()
    assert result_path == output_path


def test_create_time_distribution_chart_with_single_framework(tmp_path: Path) -> None:
    df = pl.DataFrame(
        {
            "framework": ["kreuzberg_v4_sync", "kreuzberg_v4_sync"],
            "extraction_time": [0.5, 0.6],
            "file_path": ["/test/1.pdf", "/test/2.pdf"],
        }
    )

    output_path = tmp_path / "distribution_single.html"
    result_path = create_time_distribution_chart(df, output_path)

    assert result_path.exists()


def test_create_interactive_dashboard_creates_file(tmp_path: Path) -> None:
    df = pl.DataFrame(
        {
            "framework": ["kreuzberg_v4_sync", "extractous"],
            "avg_extraction_time": [0.5, 0.8],
            "success_rate": [0.95, 0.90],
            "avg_peak_memory_mb": [100.0, 150.0],
            "files_per_second": [2.5, 1.8],
            "total_files": [100, 100],
        }
    )

    output_path = tmp_path / "dashboard.html"
    result_path = create_interactive_dashboard(df, output_path)

    assert result_path.exists()
    assert result_path == output_path
    assert result_path.stat().st_size > 0


def test_create_interactive_dashboard_content_includes_all_charts(
    tmp_path: Path,
) -> None:
    df = pl.DataFrame(
        {
            "framework": ["kreuzberg_v4_sync", "extractous"],
            "avg_extraction_time": [0.5, 0.8],
            "success_rate": [0.95, 0.90],
            "avg_peak_memory_mb": [100.0, 150.0],
            "files_per_second": [2.5, 1.8],
            "total_files": [100, 100],
        }
    )

    output_path = tmp_path / "dashboard_full.html"
    result_path = create_interactive_dashboard(df, output_path)

    content = result_path.read_text()
    assert "plotly" in content.lower()


def test_create_per_format_heatmap_creates_file(tmp_path: Path) -> None:
    df = pl.DataFrame(
        {
            "framework": [
                "kreuzberg_v4_sync",
                "kreuzberg_v4_sync",
                "extractous",
                "extractous",
            ],
            "file_type": ["pdf", "docx", "pdf", "docx"],
            "success_rate": [0.95, 0.93, 0.90, 0.88],
        }
    )

    output_path = tmp_path / "heatmap.html"
    result_path = create_per_format_heatmap(df, output_path)

    assert result_path.exists()
    assert result_path == output_path


def test_create_per_format_heatmap_with_multiple_formats(tmp_path: Path) -> None:
    df = pl.DataFrame(
        {
            "framework": [
                "kreuzberg_v4_sync",
                "kreuzberg_v4_sync",
                "kreuzberg_v4_sync",
            ],
            "file_type": ["pdf", "docx", "xlsx"],
            "success_rate": [0.95, 0.93, 0.91],
        }
    )

    output_path = tmp_path / "heatmap_multi.html"
    result_path = create_per_format_heatmap(df, output_path)

    assert result_path.exists()
    content = result_path.read_text()
    assert (
        "pdf" in content.lower()
        or "docx" in content.lower()
        or "xlsx" in content.lower()
    )


def test_all_charts_create_parent_directories(tmp_path: Path) -> None:
    nested_path = tmp_path / "charts" / "subdir" / "performance.html"
    df = pl.DataFrame(
        {
            "framework": ["kreuzberg_v4_sync"],
            "avg_extraction_time": [0.5],
            "success_rate": [0.95],
            "total_files": [100],
        }
    )

    result_path = create_performance_comparison_chart(df, nested_path)

    assert result_path.exists()
    assert result_path.parent.exists()


def test_charts_handle_special_characters_in_framework_names(tmp_path: Path) -> None:
    df = pl.DataFrame(
        {
            "framework": [
                "kreuzberg_v4_sync",
                "framework-with-dashes",
                "framework_with_underscores",
            ],
            "avg_extraction_time": [0.5, 0.6, 0.7],
            "success_rate": [0.95, 0.93, 0.91],
            "total_files": [100, 100, 100],
        }
    )

    output_path = tmp_path / "special_chars.html"
    result_path = create_performance_comparison_chart(df, output_path)

    assert result_path.exists()


def test_memory_chart_with_zero_memory_values(tmp_path: Path) -> None:
    df = pl.DataFrame(
        {
            "framework": ["kreuzberg_v4_sync"],
            "avg_peak_memory_mb": [0.0],
            "total_files": [100],
        }
    )

    output_path = tmp_path / "memory_zero.html"
    result_path = create_memory_usage_chart(df, output_path)

    assert result_path.exists()


def test_time_distribution_with_extreme_values(tmp_path: Path) -> None:
    df = pl.DataFrame(
        {
            "framework": [
                "kreuzberg_v4_sync",
                "kreuzberg_v4_sync",
                "kreuzberg_v4_sync",
            ],
            "extraction_time": [0.1, 5.0, 100.0],
            "file_path": ["/test/fast.pdf", "/test/medium.pdf", "/test/slow.pdf"],
        }
    )

    output_path = tmp_path / "distribution_extreme.html"
    result_path = create_time_distribution_chart(df, output_path)

    assert result_path.exists()
