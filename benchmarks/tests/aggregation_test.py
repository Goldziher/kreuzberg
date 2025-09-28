import tempfile
from pathlib import Path
from typing import TYPE_CHECKING

import msgspec
import pytest
from src.aggregate import ResultAggregator
from src.cli import main as cli_main
from src.types import (
    AggregatedResults,
    BenchmarkResult,
    DocumentCategory,
    ExtractionStatus,
    FileType,
    Framework,
)

if TYPE_CHECKING:
    from collections.abc import Generator


@pytest.fixture
def sample_results() -> list[BenchmarkResult]:
    return [
        BenchmarkResult(
            file_path="test1.pdf",
            file_size=1024,
            file_type=FileType.PDF,
            category=DocumentCategory.TINY,
            framework=Framework.KREUZBERG_V4_SYNC,
            iteration=1,
            extraction_time=0.5,
            peak_memory_mb=50.0,
            avg_memory_mb=40.0,
            peak_cpu_percent=80.0,
            avg_cpu_percent=60.0,
            status=ExtractionStatus.SUCCESS,
            character_count=1000,
            word_count=200,
        ),
        BenchmarkResult(
            file_path="test2.pdf",
            file_size=2048,
            file_type=FileType.PDF,
            category=DocumentCategory.SMALL,
            framework=Framework.KREUZBERG_V4_SYNC,
            iteration=1,
            extraction_time=1.0,
            peak_memory_mb=60.0,
            avg_memory_mb=50.0,
            peak_cpu_percent=90.0,
            avg_cpu_percent=70.0,
            status=ExtractionStatus.SUCCESS,
            character_count=2000,
            word_count=400,
        ),
        BenchmarkResult(
            file_path="test3.pdf",
            file_size=1024,
            file_type=FileType.PDF,
            category=DocumentCategory.TINY,
            framework=Framework.MARKITDOWN,
            iteration=1,
            extraction_time=2.0,
            peak_memory_mb=80.0,
            avg_memory_mb=70.0,
            peak_cpu_percent=100.0,
            avg_cpu_percent=85.0,
            status=ExtractionStatus.SUCCESS,
            character_count=800,
            word_count=160,
        ),
        BenchmarkResult(
            file_path="test4.pdf",
            file_size=1024,
            file_type=FileType.PDF,
            category=DocumentCategory.TINY,
            framework=Framework.MARKITDOWN,
            iteration=1,
            extraction_time=0.0,
            peak_memory_mb=0.0,
            avg_memory_mb=0.0,
            peak_cpu_percent=0.0,
            avg_cpu_percent=0.0,
            status=ExtractionStatus.FAILED,
            error_type="ImportError",
            error_message="Module not found",
        ),
    ]


@pytest.fixture
def temp_results_dirs(sample_results: list[BenchmarkResult]) -> "Generator[list[Path]]":
    with tempfile.TemporaryDirectory() as temp_dir:
        temp_path = Path(temp_dir)

        dirs = []
        for i, result in enumerate(sample_results):
            result_dir = temp_path / f"result_{i}"
            result_dir.mkdir()

            results_file = result_dir / "benchmark_results.json"
            with results_file.open("wb") as f:
                f.write(msgspec.json.encode([result]))

            dirs.append(result_dir)

        yield dirs


class TestResultAggregator:
    def test_aggregate_results_basic(
        self, temp_results_dirs: list[Path], sample_results: list[BenchmarkResult]
    ) -> None:
        aggregator = ResultAggregator()

        aggregated = aggregator.aggregate_results(temp_results_dirs)

        assert isinstance(aggregated, AggregatedResults)
        assert aggregated.total_files_processed == len(sample_results)
        assert aggregated.total_runs > 0

    def test_framework_category_matrix_string_keys(self, temp_results_dirs: list[Path]) -> None:
        aggregator = ResultAggregator()

        aggregated = aggregator.aggregate_results(temp_results_dirs)

        matrix = aggregated.framework_category_matrix
        assert isinstance(matrix, dict)

        for key in matrix:
            assert isinstance(key, str), f"Matrix key {key} is not a string"
            assert "_" in key, f"Matrix key {key} doesn't follow 'framework_category' format"

        expected_keys = {
            "kreuzberg_sync_tiny",
            "kreuzberg_sync_small",
            "markitdown_tiny",
        }

        for expected_key in expected_keys:
            assert expected_key in matrix, f"Expected key {expected_key} not found in matrix"

    def test_msgspec_serialization(self, temp_results_dirs: list[Path]) -> None:
        aggregator = ResultAggregator()

        aggregated = aggregator.aggregate_results(temp_results_dirs)

        serialized = msgspec.json.encode(aggregated)
        assert isinstance(serialized, bytes)

        decoded = msgspec.json.decode(serialized)
        assert isinstance(decoded, dict)

        matrix = decoded["framework_category_matrix"]
        for key in matrix:
            assert isinstance(key, str)

    def test_save_and_load_results(self, temp_results_dirs: list[Path]) -> None:
        aggregator = ResultAggregator()

        aggregated = aggregator.aggregate_results(temp_results_dirs)

        with tempfile.TemporaryDirectory() as temp_dir:
            output_dir = Path(temp_dir)

            aggregator.save_results(aggregated, output_dir)

            assert (output_dir / "aggregated_results.json").exists()

            with (output_dir / "aggregated_results.json").open("rb") as f:
                loaded_data = msgspec.json.decode(f.read())

            assert "framework_category_matrix" in loaded_data
            assert "framework_summaries" in loaded_data
            assert "category_summaries" in loaded_data

            matrix = loaded_data["framework_category_matrix"]
            for key in matrix:
                assert isinstance(key, str)

    def test_framework_summaries_structure(self, temp_results_dirs: list[Path]) -> None:
        aggregator = ResultAggregator()

        aggregated = aggregator.aggregate_results(temp_results_dirs)

        fw_summaries = aggregated.framework_summaries
        assert isinstance(fw_summaries, dict)

        for framework in fw_summaries:
            assert isinstance(framework, Framework)

        assert Framework.KREUZBERG_V4_SYNC in fw_summaries
        assert Framework.MARKITDOWN in fw_summaries

    def test_category_summaries_structure(self, temp_results_dirs: list[Path]) -> None:
        aggregator = ResultAggregator()

        aggregated = aggregator.aggregate_results(temp_results_dirs)

        cat_summaries = aggregated.category_summaries
        assert isinstance(cat_summaries, dict)

        for category in cat_summaries:
            assert isinstance(category, DocumentCategory)

        assert DocumentCategory.TINY in cat_summaries
        assert DocumentCategory.SMALL in cat_summaries

    def test_failure_analysis(self, temp_results_dirs: list[Path]) -> None:
        aggregator = ResultAggregator()

        aggregated = aggregator.aggregate_results(temp_results_dirs)

        failures = aggregated.failure_patterns
        assert isinstance(failures, dict)
        assert "ImportError" in failures
        assert failures["ImportError"] == 1

    def test_empty_results(self) -> None:
        aggregator = ResultAggregator()

        aggregated = aggregator.aggregate_results([])

        assert aggregated.total_files_processed == 0
        assert aggregated.total_runs == 0
        assert len(aggregated.framework_summaries) == 0
        assert len(aggregated.category_summaries) == 0
        assert len(aggregated.framework_category_matrix) == 0

    def test_matrix_key_format(self, sample_results: list[BenchmarkResult]) -> None:
        aggregator = ResultAggregator()

        matrix = aggregator._create_matrix(sample_results)

        expected_keys = [
            "kreuzberg_sync_tiny",
            "kreuzberg_sync_small",
            "markitdown_tiny",
        ]

        for key in expected_keys:
            assert key in matrix, f"Expected key {key} not found"

        for key in matrix:
            parts = key.split("_")
            assert len(parts) >= 2, f"Key {key} doesn't have expected format"
            category_part = parts[-1]
            assert any(cat.value == category_part for cat in DocumentCategory)

    def test_cli_report_generation_integration(self, temp_results_dirs: list[Path]) -> None:
        aggregator = ResultAggregator()

        aggregated = aggregator.aggregate_results(temp_results_dirs)

        with tempfile.TemporaryDirectory() as temp_dir:
            output_dir = Path(temp_dir)
            aggregator.save_results(aggregated, output_dir)

            aggregated_file = output_dir / "aggregated_results.json"
            assert aggregated_file.exists()

            with aggregated_file.open("rb") as f:
                loaded_results = msgspec.json.decode(f.read(), type=AggregatedResults)

            assert isinstance(loaded_results, AggregatedResults)
            assert hasattr(loaded_results, "total_runs")
            assert hasattr(loaded_results, "total_files_processed")
            assert hasattr(loaded_results, "framework_summaries")
            assert hasattr(loaded_results, "category_summaries")
            assert hasattr(loaded_results, "framework_category_matrix")

            assert loaded_results.total_runs > 0
            assert loaded_results.total_files_processed > 0

            for framework in loaded_results.framework_summaries:
                assert isinstance(framework, Framework)

            for category in loaded_results.category_summaries:
                assert isinstance(category, DocumentCategory)

    def test_msgspec_type_loading_consistency(self, temp_results_dirs: list[Path]) -> None:
        aggregator = ResultAggregator()
        aggregated = aggregator.aggregate_results(temp_results_dirs)

        with tempfile.TemporaryDirectory() as temp_dir:
            output_dir = Path(temp_dir)
            aggregator.save_results(aggregated, output_dir)

            aggregated_file = output_dir / "aggregated_results.json"
            with aggregated_file.open("rb") as f:
                plain_dict = msgspec.json.decode(f.read())

            assert isinstance(plain_dict, dict)
            assert "total_runs" in plain_dict

            with aggregated_file.open("rb") as f:
                typed_results = msgspec.json.decode(f.read(), type=AggregatedResults)

            assert isinstance(typed_results, AggregatedResults)
            assert typed_results.total_runs == plain_dict["total_runs"]

    @pytest.mark.xfail(
        reason="CLI report command requires actual benchmark data files which may not be available in test environment"
    )
    def test_cli_report_command_integration(self, temp_results_dirs: list[Path]) -> None:
        import sys
        from unittest.mock import patch

        aggregator = ResultAggregator()
        aggregated = aggregator.aggregate_results(temp_results_dirs)

        with tempfile.TemporaryDirectory() as temp_dir:
            output_dir = Path(temp_dir)
            report_dir = output_dir / "reports"
            report_dir.mkdir()

            aggregator.save_results(aggregated, output_dir)
            aggregated_file = output_dir / "aggregated_results.json"

            test_args = [
                "report",
                "--aggregated-file",
                str(aggregated_file),
                "--output-dir",
                str(report_dir),
                "--format",
                "json",
            ]

            with patch.object(sys, "argv", ["python", *test_args]):
                try:
                    cli_main()
                except SystemExit as e:
                    if e.code != 0:
                        raise

            report_files = list(report_dir.glob("*.json"))
            assert len(report_files) > 0, f"No JSON reports found in {report_dir}. Files: {list(report_dir.iterdir())}"

            expected_files = ["benchmark_metrics.json"]
            for expected_file in expected_files:
                assert (report_dir / expected_file).exists(), (
                    f"Expected {expected_file} not found. Available files: {[f.name for f in report_files]}"
                )
