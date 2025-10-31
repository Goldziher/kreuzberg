from __future__ import annotations

from typing import TYPE_CHECKING

import polars as pl
import pytest

if TYPE_CHECKING:
    from pathlib import Path
from src.data_transform import (
    add_derived_metrics,
    aggregate_by_framework,
    aggregate_by_framework_and_category,
    aggregate_by_framework_and_format,
    calculate_percentiles,
    export_to_csv,
    export_to_json,
    export_to_parquet,
    filter_successful_results,
    load_results_from_json,
    results_to_dataframe,
    summaries_to_dataframe,
)
from src.types import (
    BenchmarkResult,
    BenchmarkSummary,
    DocumentCategory,
    ExtractionStatus,
    FileType,
    Framework,
)


def test_results_to_dataframe_converts_empty_list_to_empty_dataframe() -> None:
    df = results_to_dataframe([])

    assert len(df) == 0
    assert isinstance(df, pl.DataFrame)


def test_results_to_dataframe_converts_single_result_correctly() -> None:
    result = BenchmarkResult(
        file_path="/test/file.pdf",
        file_size=1024,
        file_type=FileType.PDF,
        category=DocumentCategory.SMALL,
        framework=Framework.KREUZBERG_V4_SYNC,
        iteration=1,
        extraction_time=0.5,
        peak_memory_mb=100.0,
        avg_memory_mb=80.0,
        peak_cpu_percent=50.0,
        avg_cpu_percent=40.0,
        status=ExtractionStatus.SUCCESS,
        character_count=500,
        word_count=100,
    )

    df = results_to_dataframe([result])

    assert len(df) == 1
    assert df.get_column("file_path").item() == "/test/file.pdf"
    assert df.get_column("extraction_time").item() == 0.5
    assert df.get_column("framework").item() == "kreuzberg_v4_sync"


def test_results_to_dataframe_converts_multiple_results_correctly() -> None:
    results = [
        BenchmarkResult(
            file_path=f"/test/file{i}.pdf",
            file_size=1024 * i,
            file_type=FileType.PDF,
            category=DocumentCategory.SMALL,
            framework=Framework.KREUZBERG_V4_SYNC,
            iteration=1,
            extraction_time=0.5 * i,
            peak_memory_mb=100.0,
            avg_memory_mb=80.0,
            peak_cpu_percent=50.0,
            avg_cpu_percent=40.0,
            status=ExtractionStatus.SUCCESS,
        )
        for i in range(1, 4)
    ]

    df = results_to_dataframe(results)

    assert len(df) == 3
    assert df.get_column("file_path").to_list() == [
        "/test/file1.pdf",
        "/test/file2.pdf",
        "/test/file3.pdf",
    ]


def test_summaries_to_dataframe_converts_empty_list_to_empty_dataframe() -> None:
    df = summaries_to_dataframe([])

    assert len(df) == 0
    assert isinstance(df, pl.DataFrame)


def test_summaries_to_dataframe_converts_summaries_correctly() -> None:
    summary = BenchmarkSummary(
        framework=Framework.KREUZBERG_V4_SYNC,
        category=DocumentCategory.SMALL,
        total_files=10,
        successful_files=9,
        failed_files=1,
        partial_files=0,
        timeout_files=0,
        success_rate=0.9,
    )

    df = summaries_to_dataframe([summary])

    assert len(df) == 1
    assert df.get_column("framework").item() == "kreuzberg_v4_sync"
    assert df.get_column("total_files").item() == 10
    assert df.get_column("success_rate").item() == 0.9


def test_aggregate_by_framework_groups_results_correctly() -> None:
    results = [
        BenchmarkResult(
            file_path=f"/test/file{i}.pdf",
            file_size=1024,
            file_type=FileType.PDF,
            category=DocumentCategory.SMALL,
            framework=Framework.KREUZBERG_V4_SYNC,
            iteration=1,
            extraction_time=0.5,
            peak_memory_mb=100.0,
            avg_memory_mb=80.0,
            peak_cpu_percent=50.0,
            avg_cpu_percent=40.0,
            status=ExtractionStatus.SUCCESS,
        )
        for i in range(3)
    ]

    df = results_to_dataframe(results)
    aggregated = aggregate_by_framework(df)

    assert len(aggregated) == 1
    assert aggregated.get_column("framework").item() == "kreuzberg_v4_sync"
    assert aggregated.get_column("total_files").item() == 3
    assert aggregated.get_column("successful_files").item() == 3


def test_aggregate_by_framework_calculates_success_rate_correctly() -> None:
    results = [
        BenchmarkResult(
            file_path="/test/success.pdf",
            file_size=1024,
            file_type=FileType.PDF,
            category=DocumentCategory.SMALL,
            framework=Framework.KREUZBERG_V4_SYNC,
            iteration=1,
            extraction_time=0.5,
            peak_memory_mb=100.0,
            avg_memory_mb=80.0,
            peak_cpu_percent=50.0,
            avg_cpu_percent=40.0,
            status=ExtractionStatus.SUCCESS,
        ),
        BenchmarkResult(
            file_path="/test/failed.pdf",
            file_size=1024,
            file_type=FileType.PDF,
            category=DocumentCategory.SMALL,
            framework=Framework.KREUZBERG_V4_SYNC,
            iteration=1,
            extraction_time=0.0,
            peak_memory_mb=0.0,
            avg_memory_mb=0.0,
            peak_cpu_percent=0.0,
            avg_cpu_percent=0.0,
            status=ExtractionStatus.FAILED,
        ),
    ]

    df = results_to_dataframe(results)
    aggregated = aggregate_by_framework(df)

    assert aggregated.get_column("success_rate").item() == pytest.approx(0.5)
    assert aggregated.get_column("successful_files").item() == 1
    assert aggregated.get_column("failed_files").item() == 1


def test_aggregate_by_framework_and_format_groups_correctly() -> None:
    results = [
        BenchmarkResult(
            file_path="/test/doc.pdf",
            file_size=1024,
            file_type=FileType.PDF,
            category=DocumentCategory.SMALL,
            framework=Framework.KREUZBERG_V4_SYNC,
            iteration=1,
            extraction_time=0.5,
            peak_memory_mb=100.0,
            avg_memory_mb=80.0,
            peak_cpu_percent=50.0,
            avg_cpu_percent=40.0,
            status=ExtractionStatus.SUCCESS,
        ),
        BenchmarkResult(
            file_path="/test/doc.docx",
            file_size=2048,
            file_type=FileType.DOCX,
            category=DocumentCategory.SMALL,
            framework=Framework.KREUZBERG_V4_SYNC,
            iteration=1,
            extraction_time=0.3,
            peak_memory_mb=80.0,
            avg_memory_mb=60.0,
            peak_cpu_percent=40.0,
            avg_cpu_percent=30.0,
            status=ExtractionStatus.SUCCESS,
        ),
    ]

    df = results_to_dataframe(results)
    aggregated = aggregate_by_framework_and_format(df)

    assert len(aggregated) == 2
    assert set(aggregated.get_column("file_type").to_list()) == {"pdf", "docx"}


def test_aggregate_by_framework_and_category_groups_correctly() -> None:
    results = [
        BenchmarkResult(
            file_path="/test/small.pdf",
            file_size=1024,
            file_type=FileType.PDF,
            category=DocumentCategory.SMALL,
            framework=Framework.KREUZBERG_V4_SYNC,
            iteration=1,
            extraction_time=0.5,
            peak_memory_mb=100.0,
            avg_memory_mb=80.0,
            peak_cpu_percent=50.0,
            avg_cpu_percent=40.0,
            status=ExtractionStatus.SUCCESS,
        ),
        BenchmarkResult(
            file_path="/test/large.pdf",
            file_size=10240,
            file_type=FileType.PDF,
            category=DocumentCategory.LARGE,
            framework=Framework.KREUZBERG_V4_SYNC,
            iteration=1,
            extraction_time=2.5,
            peak_memory_mb=200.0,
            avg_memory_mb=150.0,
            peak_cpu_percent=60.0,
            avg_cpu_percent=50.0,
            status=ExtractionStatus.SUCCESS,
        ),
    ]

    df = results_to_dataframe(results)
    aggregated = aggregate_by_framework_and_category(df)

    assert len(aggregated) == 2
    assert set(aggregated.get_column("category").to_list()) == {"small", "large"}


def test_calculate_percentiles_returns_correct_values() -> None:
    df = pl.DataFrame({"value": [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]})

    percentiles = calculate_percentiles(df, "value", [0.50, 0.95, 0.99])

    assert "p50_value" in percentiles
    assert "p95_value" in percentiles
    assert "p99_value" in percentiles
    assert percentiles["p50_value"] == pytest.approx(5.5, rel=0.1)
    assert percentiles["p95_value"] == pytest.approx(9.55, rel=0.1)


def test_filter_successful_results_filters_correctly() -> None:
    results = [
        BenchmarkResult(
            file_path="/test/success.pdf",
            file_size=1024,
            file_type=FileType.PDF,
            category=DocumentCategory.SMALL,
            framework=Framework.KREUZBERG_V4_SYNC,
            iteration=1,
            extraction_time=0.5,
            peak_memory_mb=100.0,
            avg_memory_mb=80.0,
            peak_cpu_percent=50.0,
            avg_cpu_percent=40.0,
            status=ExtractionStatus.SUCCESS,
        ),
        BenchmarkResult(
            file_path="/test/failed.pdf",
            file_size=1024,
            file_type=FileType.PDF,
            category=DocumentCategory.SMALL,
            framework=Framework.KREUZBERG_V4_SYNC,
            iteration=1,
            extraction_time=0.0,
            peak_memory_mb=0.0,
            avg_memory_mb=0.0,
            peak_cpu_percent=0.0,
            avg_cpu_percent=0.0,
            status=ExtractionStatus.FAILED,
        ),
    ]

    df = results_to_dataframe(results)
    filtered = filter_successful_results(df)

    assert len(filtered) == 1
    assert filtered.get_column("status").item() == "success"


def test_add_derived_metrics_adds_correct_columns() -> None:
    results = [
        BenchmarkResult(
            file_path="/test/file.pdf",
            file_size=1048576,
            file_type=FileType.PDF,
            category=DocumentCategory.SMALL,
            framework=Framework.KREUZBERG_V4_SYNC,
            iteration=1,
            extraction_time=1.0,
            peak_memory_mb=100.0,
            avg_memory_mb=80.0,
            peak_cpu_percent=50.0,
            avg_cpu_percent=40.0,
            status=ExtractionStatus.SUCCESS,
            character_count=1000,
            word_count=200,
        )
    ]

    df = results_to_dataframe(results)
    with_metrics = add_derived_metrics(df)

    assert "file_size_mb" in with_metrics.columns
    assert "chars_per_second" in with_metrics.columns
    assert "words_per_second" in with_metrics.columns
    assert "memory_per_mb" in with_metrics.columns

    assert with_metrics.get_column("file_size_mb").item() == pytest.approx(1.0)
    assert with_metrics.get_column("chars_per_second").item() == pytest.approx(1000.0)
    assert with_metrics.get_column("words_per_second").item() == pytest.approx(200.0)


def test_export_to_csv_creates_file(tmp_path: Path) -> None:
    df = pl.DataFrame({"a": [1, 2, 3], "b": [4, 5, 6]})

    output_path = tmp_path / "test.csv"
    result_path = export_to_csv(df, output_path)

    assert result_path.exists()
    assert result_path == output_path

    loaded_df = pl.read_csv(str(result_path))
    assert len(loaded_df) == 3


def test_export_to_json_creates_file(tmp_path: Path) -> None:
    df = pl.DataFrame({"a": [1, 2, 3], "b": [4, 5, 6]})

    output_path = tmp_path / "test.json"
    result_path = export_to_json(df, output_path)

    assert result_path.exists()
    assert result_path == output_path


def test_export_to_parquet_creates_file(tmp_path: Path) -> None:
    df = pl.DataFrame({"a": [1, 2, 3], "b": [4, 5, 6]})

    output_path = tmp_path / "test.parquet"
    result_path = export_to_parquet(df, output_path)

    assert result_path.exists()
    assert result_path == output_path

    loaded_df = pl.read_parquet(str(result_path))
    assert len(loaded_df) == 3


def test_load_results_from_json_loads_correctly(tmp_path: Path) -> None:
    results = [
        BenchmarkResult(
            file_path="/test/file.pdf",
            file_size=1024,
            file_type=FileType.PDF,
            category=DocumentCategory.SMALL,
            framework=Framework.KREUZBERG_V4_SYNC,
            iteration=1,
            extraction_time=0.5,
            peak_memory_mb=100.0,
            avg_memory_mb=80.0,
            peak_cpu_percent=50.0,
            avg_cpu_percent=40.0,
            status=ExtractionStatus.SUCCESS,
        )
    ]

    import msgspec

    json_path = tmp_path / "results.json"
    json_path.write_bytes(msgspec.json.encode(results))

    df = load_results_from_json(json_path)

    assert len(df) == 1
    assert df.get_column("file_path").item() == "/test/file.pdf"
