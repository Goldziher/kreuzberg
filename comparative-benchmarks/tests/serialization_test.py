import json
from typing import Any

import msgspec
from src.types import (
    BenchmarkResult,
    BenchmarkSummary,
    DocumentCategory,
    ExtractionStatus,
    FileType,
    Framework,
)


def create_test_result(
    status: ExtractionStatus = ExtractionStatus.SUCCESS, **kwargs: Any
) -> BenchmarkResult:
    defaults = {
        "file_path": "test.pdf",
        "file_size": 1000,
        "file_type": FileType.PDF,
        "category": DocumentCategory.SMALL,
        "framework": Framework.KREUZBERG_V4_SYNC,
        "iteration": 1,
        "extraction_time": 1.5,
        "peak_memory_mb": 100.0,
        "avg_memory_mb": 80.0,
        "peak_cpu_percent": 90.0,
        "avg_cpu_percent": 75.0,
        "status": status,
    }
    defaults.update(kwargs)
    return BenchmarkResult(**defaults)  # type: ignore[arg-type]


def test_benchmark_result_has_required_fields() -> None:
    result = create_test_result()

    assert hasattr(result, "status")
    assert hasattr(result, "file_size")
    assert hasattr(result, "extraction_time")
    assert hasattr(result, "peak_memory_mb")
    assert hasattr(result, "avg_cpu_percent")
    assert hasattr(result, "character_count")


def test_benchmark_result_serialization() -> None:
    result = create_test_result(character_count=1000, error_message="Test error")

    serialized = msgspec.json.encode(result)
    data = json.loads(serialized)

    assert data["file_size"] == 1000
    assert data["extraction_time"] == 1.5
    assert data["peak_memory_mb"] == 100.0
    assert data["avg_cpu_percent"] == 75.0
    assert data["status"] == "success"
    assert data["character_count"] == 1000


def test_failed_result_serialization() -> None:
    results = [
        BenchmarkResult(
            file_path=f"test{i}.pdf",
            file_size=1000 * (i + 1),
            file_type=FileType.PDF,
            category=DocumentCategory.LARGE,
            framework=Framework.KREUZBERG_V4_ASYNC,
            iteration=1,
            extraction_time=0.001,
            peak_memory_mb=50.0,
            avg_memory_mb=40.0,
            peak_cpu_percent=10.0,
            avg_cpu_percent=5.0,
            status=ExtractionStatus.FAILED,
            error_type="TestError",
            error_message="Test error message",
        )
        for i in range(3)
    ]

    for result in results:
        serialized = msgspec.json.encode(result)
        data = json.loads(serialized)

        assert data["status"] == "failed"
        assert data["error_message"] == "Test error message"
        assert data["error_type"] == "TestError"


def test_none_value_serialization() -> None:
    result = create_test_result(
        extraction_time=None,
        character_count=None,
        word_count=None,
        error_message="Error occurred",
    )

    serialized = msgspec.json.encode(result)
    data = json.loads(serialized)

    assert data["extraction_time"] is None
    assert data["character_count"] is None
    assert data["word_count"] is None
    assert data["error_message"] == "Error occurred"


def test_benchmark_summary_with_all_failed() -> None:
    summary = BenchmarkSummary(
        framework=Framework.KREUZBERG_V4_ASYNC,
        category=DocumentCategory.LARGE,
        total_files=9,
        successful_files=0,
        failed_files=9,
        partial_files=0,
        timeout_files=0,
        avg_extraction_time=None,
        median_extraction_time=None,
        min_extraction_time=None,
        max_extraction_time=None,
        avg_peak_memory_mb=None,
        avg_cpu_percent=None,
        success_rate=0.0,
        files_per_second=None,
        mb_per_second=None,
    )

    assert summary.successful_files == 0
    assert summary.failed_files == 9
    assert summary.success_rate == 0.0
    assert summary.files_per_second is None

    serialized = msgspec.json.encode(summary)
    data = json.loads(serialized)
    assert data["success_rate"] == 0.0
