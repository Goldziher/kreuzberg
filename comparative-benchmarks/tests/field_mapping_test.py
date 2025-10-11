import pytest
from src.types import (
    BenchmarkResult,
    BenchmarkSummary,
    DocumentCategory,
    ExtractionStatus,
    FileType,
    Framework,
)


class TestFieldMapping:
    def test_benchmark_result_field_mapping(self) -> None:
        result = BenchmarkResult(
            file_path="test.pdf",
            file_size=1000,
            file_type=FileType.PDF,
            category=DocumentCategory.SMALL,
            framework=Framework.KREUZBERG_V4_SYNC,
            iteration=1,
            extraction_time=1.5,
            peak_memory_mb=100.0,
            avg_memory_mb=80.0,
            peak_cpu_percent=90.0,
            avg_cpu_percent=75.0,
            status=ExtractionStatus.SUCCESS,
        )

        assert result.file_size == 1000
        assert result.extraction_time == 1.5
        assert result.peak_memory_mb == 100.0
        assert result.avg_cpu_percent == 75.0
        assert result.status == ExtractionStatus.SUCCESS
        assert result.character_count is None

    def test_benchmark_summary_field_mapping(self) -> None:
        summary = BenchmarkSummary(
            framework=Framework.KREUZBERG_V4_SYNC,
            category=DocumentCategory.SMALL,
            total_files=10,
            successful_files=8,
            failed_files=2,
            partial_files=0,
            timeout_files=0,
            avg_extraction_time=2.5,
            median_extraction_time=2.0,
            min_extraction_time=1.0,
            max_extraction_time=5.0,
            avg_peak_memory_mb=100.0,
            avg_cpu_percent=75.0,
            success_rate=0.8,
            files_per_second=4.0,
            mb_per_second=10.0,
        )

        assert summary.successful_files == 8
        assert summary.failed_files == 2
        assert summary.avg_extraction_time == 2.5
        assert summary.median_extraction_time == 2.0
        assert summary.avg_peak_memory_mb == 100.0
        assert summary.avg_cpu_percent == 75.0
        assert not hasattr(summary, "file_type")
        assert not hasattr(summary, "total_time_seconds")

    def test_reporting_assumptions(self) -> None:
        summary_transformations = {
            "successful_extractions": lambda s: s.successful_files,
            "failed_extractions": lambda s: s.failed_files,
            "average_time_seconds": lambda s: s.avg_extraction_time,
            "median_time_seconds": lambda s: s.median_extraction_time,
            "min_time_seconds": lambda s: s.min_extraction_time,
            "max_time_seconds": lambda s: s.max_extraction_time,
            "average_memory_mb": lambda s: s.avg_peak_memory_mb,
            "average_cpu_percent": lambda s: s.avg_cpu_percent,
            "total_time_seconds": lambda s: s.avg_extraction_time * s.successful_files
            if s.avg_extraction_time
            else 0,
        }

        result_transformations = {
            "file_size_bytes": lambda r: r.file_size,
            "extraction_time_seconds": lambda r: r.extraction_time,
            "memory_peak_mb": lambda r: r.peak_memory_mb,
            "cpu_percent": lambda r: r.avg_cpu_percent,
            "success": lambda r: r.status == ExtractionStatus.SUCCESS,
            "extracted_text_length": lambda r: r.character_count,
        }

        assert len(summary_transformations) == 9
        assert len(result_transformations) == 6


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
