from __future__ import annotations

from typing import TYPE_CHECKING, cast

import polars as pl

if TYPE_CHECKING:
    from pathlib import Path
from src.docs_generator import (
    DocConfig,
    generate_detailed_results_page,
    generate_framework_comparison_page,
    generate_index_page,
    generate_methodology_page,
)


def test_generate_index_page_creates_file(tmp_path: Path) -> None:
    df = pl.DataFrame(
        {
            "framework": ["kreuzberg_v4_sync", "extractous"],
            "avg_extraction_time": [0.5, 0.8],
            "success_rate": [0.95, 0.90],
            "avg_peak_memory_mb": [100.0, 150.0],
            "total_files": [100, 100],
            "successful_files": [95, 90],
            "median_extraction_time": [0.45, 0.75],
            "p95_extraction_time": [0.9, 1.5],
            "p99_extraction_time": [1.2, 2.0],
            "files_per_second": [2.0, 1.25],
        }
    )

    output_path = tmp_path / "index.md"
    result_path = generate_index_page(df, output_path)

    assert result_path.exists()
    assert result_path == output_path


def test_generate_index_page_contains_expected_content(tmp_path: Path) -> None:
    df = pl.DataFrame(
        {
            "framework": ["kreuzberg_v4_sync"],
            "avg_extraction_time": [0.5],
            "success_rate": [0.95],
            "avg_peak_memory_mb": [100.0],
            "total_files": [100],
            "successful_files": [95],
            "median_extraction_time": [0.45],
            "p95_extraction_time": [0.9],
            "p99_extraction_time": [1.2],
            "files_per_second": [2.0],
        }
    )

    output_path = tmp_path / "index.md"
    result_path = generate_index_page(df, output_path)

    content = result_path.read_text()
    assert "Performance Leaders" in content
    assert "kreuzberg_v4_sync" in content
    assert "Framework Comparison" in content


def test_generate_index_page_without_timestamp(tmp_path: Path) -> None:
    df = pl.DataFrame(
        {
            "framework": ["kreuzberg_v4_sync"],
            "avg_extraction_time": [0.5],
            "success_rate": [0.95],
            "avg_peak_memory_mb": [100.0],
            "total_files": [100],
            "successful_files": [95],
            "median_extraction_time": [0.45],
            "p95_extraction_time": [0.9],
            "p99_extraction_time": [1.2],
            "files_per_second": [2.0],
        }
    )

    output_path = tmp_path / "index_no_timestamp.md"
    config = cast("DocConfig", {"include_timestamp": False})
    result_path = generate_index_page(df, output_path, config)

    content = result_path.read_text()
    assert "Last updated:" in content


def test_generate_detailed_results_page_creates_file(tmp_path: Path) -> None:
    summary_df = pl.DataFrame(
        {
            "framework": ["kreuzberg_v4_sync", "extractous"],
            "total_files": [100, 100],
            "successful_files": [95, 90],
            "failed_files": [5, 10],
            "timeout_files": [0, 0],
            "success_rate": [0.95, 0.90],
            "median_extraction_time": [0.45, 0.75],
            "p95_extraction_time": [0.9, 1.5],
            "p99_extraction_time": [1.2, 2.0],
            "avg_peak_memory_mb": [100.0, 150.0],
        }
    )

    output_path = tmp_path / "detailed.md"
    result_path = generate_detailed_results_page(summary_df, None, output_path)

    assert result_path.exists()
    assert result_path == output_path


def test_generate_detailed_results_page_with_format_breakdown(tmp_path: Path) -> None:
    summary_df = pl.DataFrame(
        {
            "framework": ["kreuzberg_v4_sync"],
            "total_files": [100],
            "successful_files": [95],
            "failed_files": [5],
            "timeout_files": [0],
            "success_rate": [0.95],
            "median_extraction_time": [0.45],
            "p95_extraction_time": [0.9],
            "p99_extraction_time": [1.2],
            "avg_peak_memory_mb": [100.0],
        }
    )

    format_df = pl.DataFrame(
        {
            "framework": ["kreuzberg_v4_sync", "kreuzberg_v4_sync"],
            "file_type": ["pdf", "docx"],
            "total_files": [50, 50],
            "success_rate": [0.96, 0.94],
            "median_extraction_time": [0.5, 0.4],
            "avg_peak_memory_mb": [110.0, 90.0],
        }
    )

    output_path = tmp_path / "detailed_with_format.md"
    result_path = generate_detailed_results_page(summary_df, format_df, output_path)

    content = result_path.read_text()
    assert "Performance by File Format" in content
    assert "PDF" in content or "DOCX" in content


def test_generate_detailed_results_page_contains_metrics(tmp_path: Path) -> None:
    summary_df = pl.DataFrame(
        {
            "framework": ["kreuzberg_v4_sync"],
            "total_files": [100],
            "successful_files": [95],
            "failed_files": [5],
            "timeout_files": [0],
            "success_rate": [0.95],
            "median_extraction_time": [0.45],
            "p95_extraction_time": [0.9],
            "p99_extraction_time": [1.2],
            "avg_peak_memory_mb": [100.0],
        }
    )

    output_path = tmp_path / "detailed_metrics.md"
    result_path = generate_detailed_results_page(summary_df, None, output_path)

    content = result_path.read_text()
    assert "Success Rate" in content or "success" in content.lower()
    assert "Median" in content
    assert "P95" in content
    assert "P99" in content


def test_generate_methodology_page_creates_file(tmp_path: Path) -> None:
    output_path = tmp_path / "methodology.md"
    result_path = generate_methodology_page(output_path)

    assert result_path.exists()
    assert result_path == output_path


def test_generate_methodology_page_contains_expected_sections(tmp_path: Path) -> None:
    output_path = tmp_path / "methodology_sections.md"
    result_path = generate_methodology_page(output_path)

    content = result_path.read_text()
    assert "Benchmark Methodology" in content
    assert "Test Environment" in content
    assert "Frameworks Tested" in content
    assert "Metrics Collected" in content
    assert "Test Execution" in content


def test_generate_framework_comparison_page_creates_file(tmp_path: Path) -> None:
    df = pl.DataFrame(
        {
            "framework": ["kreuzberg_v4_sync", "extractous"],
            "avg_extraction_time": [0.5, 0.8],
            "success_rate": [0.95, 0.90],
            "avg_peak_memory_mb": [100.0, 150.0],
        }
    )

    output_path = tmp_path / "comparison.md"
    result_path = generate_framework_comparison_page(df, output_path)

    assert result_path.exists()
    assert result_path == output_path


def test_generate_framework_comparison_page_contains_frameworks(tmp_path: Path) -> None:
    df = pl.DataFrame(
        {
            "framework": ["kreuzberg_v4_sync", "extractous"],
            "avg_extraction_time": [0.5, 0.8],
            "success_rate": [0.95, 0.90],
            "avg_peak_memory_mb": [100.0, 150.0],
        }
    )

    output_path = tmp_path / "comparison_frameworks.md"
    result_path = generate_framework_comparison_page(df, output_path)

    content = result_path.read_text()
    assert "kreuzberg_v4_sync" in content
    assert "extractous" in content
    assert "Framework Comparison" in content


def test_generate_framework_comparison_page_has_recommendations(tmp_path: Path) -> None:
    df = pl.DataFrame(
        {
            "framework": ["kreuzberg_v4_sync"],
            "avg_extraction_time": [0.5],
            "success_rate": [0.95],
            "avg_peak_memory_mb": [100.0],
        }
    )

    output_path = tmp_path / "comparison_recs.md"
    result_path = generate_framework_comparison_page(df, output_path)

    content = result_path.read_text()
    assert "Recommendations" in content


def test_all_generators_create_parent_directories(tmp_path: Path) -> None:
    nested_path = tmp_path / "docs" / "benchmarks" / "index.md"

    df = pl.DataFrame(
        {
            "framework": ["kreuzberg_v4_sync"],
            "avg_extraction_time": [0.5],
            "success_rate": [0.95],
            "avg_peak_memory_mb": [100.0],
            "total_files": [100],
            "successful_files": [95],
            "median_extraction_time": [0.45],
            "p95_extraction_time": [0.9],
            "p99_extraction_time": [1.2],
            "files_per_second": [2.0],
        }
    )

    result_path = generate_index_page(df, nested_path)

    assert result_path.exists()
    assert result_path.parent.exists()


def test_generate_index_page_with_multiple_frameworks_orders_by_speed(
    tmp_path: Path,
) -> None:
    df = pl.DataFrame(
        {
            "framework": ["slow_framework", "kreuzberg_v4_sync", "medium_framework"],
            "avg_extraction_time": [2.0, 0.5, 1.0],
            "success_rate": [0.85, 0.95, 0.90],
            "avg_peak_memory_mb": [200.0, 100.0, 150.0],
            "total_files": [100, 100, 100],
            "successful_files": [85, 95, 90],
            "median_extraction_time": [1.9, 0.45, 0.95],
            "p95_extraction_time": [3.0, 0.9, 1.8],
            "p99_extraction_time": [4.0, 1.2, 2.5],
            "files_per_second": [0.5, 2.0, 1.0],
        }
    )

    output_path = tmp_path / "index_ordered.md"
    result_path = generate_index_page(df, output_path)

    content = result_path.read_text()
    kreuzberg_pos = content.find("kreuzberg_v4_sync")
    slow_pos = content.find("slow_framework")
    assert kreuzberg_pos < slow_pos


def test_generate_detailed_results_page_with_empty_format_df(tmp_path: Path) -> None:
    summary_df = pl.DataFrame(
        {
            "framework": ["kreuzberg_v4_sync"],
            "total_files": [100],
            "successful_files": [95],
            "failed_files": [5],
            "timeout_files": [0],
            "success_rate": [0.95],
            "median_extraction_time": [0.45],
            "p95_extraction_time": [0.9],
            "p99_extraction_time": [1.2],
            "avg_peak_memory_mb": [100.0],
        }
    )

    empty_format_df = pl.DataFrame(
        {
            "framework": [],
            "file_type": [],
            "total_files": [],
            "success_rate": [],
            "median_extraction_time": [],
            "avg_peak_memory_mb": [],
        }
    )

    output_path = tmp_path / "detailed_empty_format.md"
    result_path = generate_detailed_results_page(
        summary_df, empty_format_df, output_path
    )

    assert result_path.exists()


def test_methodology_page_is_static_and_consistent(tmp_path: Path) -> None:
    output_path1 = tmp_path / "methodology1.md"
    output_path2 = tmp_path / "methodology2.md"

    result1 = generate_methodology_page(output_path1)
    result2 = generate_methodology_page(output_path2)

    content1 = result1.read_text()
    content2 = result2.read_text()

    assert content1 == content2
