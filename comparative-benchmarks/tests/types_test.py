import msgspec

from src.types import (
    AggregatedResults,
    BenchmarkConfig,
    BenchmarkResult,
    BenchmarkSummary,
    DocumentCategory,
    ExtractionStatus,
    FileType,
    Framework,
    ResourceMetrics,
)


def test_framework_enum() -> None:
    expected_frameworks = [
        Framework.KREUZBERG_V4_SYNC,
        Framework.KREUZBERG_V4_ASYNC,
        Framework.DOCLING,
        Framework.UNSTRUCTURED,
        Framework.EXTRACTOUS,
        Framework.MARKITDOWN,
    ]

    for framework in expected_frameworks:
        assert isinstance(framework, Framework)
        assert isinstance(framework.value, str)


def test_file_type_enum() -> None:
    expected_types = [
        FileType.PDF,
        FileType.DOCX,
        FileType.PPTX,
        FileType.HTML,
        FileType.TXT,
        FileType.IMAGE_PNG,
        FileType.CSV,
        FileType.JSON,
    ]

    for file_type in expected_types:
        assert isinstance(file_type, FileType)
        assert isinstance(file_type.value, str)


def test_document_category_enum() -> None:
    expected_categories = [
        DocumentCategory.TINY,
        DocumentCategory.SMALL,
        DocumentCategory.MEDIUM,
        DocumentCategory.LARGE,
        DocumentCategory.HUGE,
        DocumentCategory.PDF_STANDARD,
        DocumentCategory.OFFICE,
        DocumentCategory.WEB,
        DocumentCategory.IMAGES,
    ]

    for category in expected_categories:
        assert isinstance(category, DocumentCategory)
        assert isinstance(category.value, str)


def test_extraction_status_enum() -> None:
    expected_statuses = [
        ExtractionStatus.SUCCESS,
        ExtractionStatus.PARTIAL,
        ExtractionStatus.FAILED,
        ExtractionStatus.TIMEOUT,
        ExtractionStatus.SKIPPED,
    ]

    for status in expected_statuses:
        assert isinstance(status, ExtractionStatus)
        assert isinstance(status.value, str)


def test_resource_metrics_creation() -> None:
    metrics = ResourceMetrics(
        timestamp=1234567890.0,
        cpu_percent=50.0,
        memory_rss=1024 * 1024 * 100,
        memory_vms=1024 * 1024 * 200,
        num_threads=4,
        open_files=10,
        io_read_bytes=1024,
        io_write_bytes=512,
        io_read_count=5,
        io_write_count=3,
    )

    assert metrics.timestamp == 1234567890.0
    assert metrics.cpu_percent == 50.0
    assert metrics.memory_rss == 1024 * 1024 * 100
    assert metrics.memory_vms == 1024 * 1024 * 200
    assert metrics.num_threads == 4
    assert metrics.open_files == 10
    assert metrics.io_read_bytes == 1024
    assert metrics.io_write_bytes == 512
    assert metrics.io_read_count == 5
    assert metrics.io_write_count == 3


def test_benchmark_result_creation() -> None:
    result = BenchmarkResult(
        file_path="test.pdf",
        file_size=1024,
        file_type=FileType.PDF,
        category=DocumentCategory.SMALL,
        framework=Framework.KREUZBERG_V4_SYNC,
        iteration=1,
        extraction_time=1.5,
        peak_memory_mb=256.0,
        avg_memory_mb=200.0,
        peak_cpu_percent=80.0,
        avg_cpu_percent=60.0,
        status=ExtractionStatus.SUCCESS,
        character_count=5000,
        word_count=800,
    )

    assert result.file_path == "test.pdf"
    assert result.file_size == 1024
    assert result.file_type == FileType.PDF
    assert result.category == DocumentCategory.SMALL
    assert result.framework == Framework.KREUZBERG_V4_SYNC
    assert result.iteration == 1
    assert result.extraction_time == 1.5
    assert result.peak_memory_mb == 256.0
    assert result.avg_memory_mb == 200.0
    assert result.peak_cpu_percent == 80.0
    assert result.avg_cpu_percent == 60.0
    assert result.status == ExtractionStatus.SUCCESS
    assert result.character_count == 5000
    assert result.word_count == 800


def test_benchmark_result_with_optional_fields() -> None:
    result = BenchmarkResult(
        file_path="test.pdf",
        file_size=1024,
        file_type=FileType.PDF,
        category=DocumentCategory.SMALL,
        framework=Framework.KREUZBERG_V4_SYNC,
        iteration=1,
        extraction_time=1.5,
        peak_memory_mb=256.0,
        avg_memory_mb=200.0,
        peak_cpu_percent=80.0,
        avg_cpu_percent=60.0,
        status=ExtractionStatus.SUCCESS,
        startup_time=0.5,
        total_io_mb=10.5,
        error_type="TestError",
        error_message="Test error message",
        extracted_text="Sample extracted text",
        extracted_metadata={"pages": 5, "title": "Test Document"},
    )

    assert result.startup_time == 0.5
    assert result.total_io_mb == 10.5
    assert result.error_type == "TestError"
    assert result.error_message == "Test error message"
    assert result.extracted_text == "Sample extracted text"
    assert result.extracted_metadata == {"pages": 5, "title": "Test Document"}


def test_benchmark_summary_creation() -> None:
    summary = BenchmarkSummary(
        framework=Framework.KREUZBERG_V4_SYNC,
        category=DocumentCategory.SMALL,
        total_files=10,
        successful_files=9,
        failed_files=1,
        partial_files=0,
        timeout_files=0,
        success_rate=90.0,
        avg_extraction_time=1.2,
        median_extraction_time=1.0,
        avg_peak_memory_mb=200.0,
        avg_cpu_percent=50.0,
        files_per_second=0.8,
        mb_per_second=40.0,
    )

    assert summary.framework == Framework.KREUZBERG_V4_SYNC
    assert summary.category == DocumentCategory.SMALL
    assert summary.total_files == 10
    assert summary.successful_files == 9
    assert summary.failed_files == 1
    assert summary.partial_files == 0
    assert summary.timeout_files == 0
    assert summary.success_rate == 90.0
    assert summary.avg_extraction_time == 1.2
    assert summary.median_extraction_time == 1.0


def test_benchmark_summary_consistency() -> None:
    summary = BenchmarkSummary(
        framework=Framework.KREUZBERG_V4_SYNC,
        category=DocumentCategory.SMALL,
        total_files=10,
        successful_files=7,
        failed_files=2,
        partial_files=0,
        timeout_files=1,
        success_rate=70.0,
        avg_extraction_time=1.2,
        median_extraction_time=1.0,
        avg_peak_memory_mb=200.0,
        avg_cpu_percent=50.0,
        files_per_second=0.8,
        mb_per_second=40.0,
    )

    assert summary.total_files == (
        summary.successful_files
        + summary.failed_files
        + summary.partial_files
        + summary.timeout_files
    )

    expected_success_rate = summary.successful_files / summary.total_files * 100
    assert summary.success_rate == expected_success_rate


def test_aggregated_results_creation() -> None:
    summary = BenchmarkSummary(
        framework=Framework.KREUZBERG_V4_SYNC,
        category=DocumentCategory.SMALL,
        total_files=1,
        successful_files=1,
        failed_files=0,
        partial_files=0,
        timeout_files=0,
        success_rate=100.0,
        avg_extraction_time=1.5,
        median_extraction_time=1.5,
        avg_peak_memory_mb=256.0,
        avg_cpu_percent=75.0,
        files_per_second=0.67,
        mb_per_second=0.67,
    )

    aggregated = AggregatedResults(
        total_runs=1,
        total_files_processed=1,
        total_time_seconds=1.5,
        framework_summaries={Framework.KREUZBERG_V4_SYNC: [summary]},
        category_summaries={DocumentCategory.SMALL: [summary]},
        framework_category_matrix={"kreuzberg_sync_small": summary},
        failure_patterns={},
        timeout_files=[],
        performance_over_iterations={Framework.KREUZBERG_V4_SYNC: [1.5]},
        platform_results={"Linux": {Framework.KREUZBERG_V4_SYNC: summary}},
    )

    assert aggregated.total_runs == 1
    assert aggregated.total_files_processed == 1
    assert aggregated.total_time_seconds == 1.5
    assert len(aggregated.framework_summaries) == 1
    assert len(aggregated.category_summaries) == 1
    assert len(aggregated.framework_category_matrix) == 1


def test_benchmark_config_creation() -> None:
    config = BenchmarkConfig(
        iterations=5,
        warmup_runs=2,
        cooldown_seconds=10,
        timeout_seconds=600,
        max_memory_mb=2048,
        frameworks=[Framework.KREUZBERG_V4_SYNC, Framework.EXTRACTOUS],
        categories=[DocumentCategory.SMALL, DocumentCategory.MEDIUM],
        file_types=[FileType.PDF, FileType.DOCX],
        save_extracted_text=True,
        detailed_errors=True,
    )

    assert config.iterations == 5
    assert config.warmup_runs == 2
    assert config.cooldown_seconds == 10
    assert config.timeout_seconds == 600
    assert config.max_memory_mb == 2048
    assert len(config.frameworks) == 2
    assert len(config.categories) == 2
    assert config.file_types is not None
    assert len(config.file_types) == 2
    assert config.save_extracted_text is True
    assert config.detailed_errors is True


def test_benchmark_config_defaults() -> None:
    config = BenchmarkConfig()

    assert config.iterations == 20
    assert config.warmup_runs == 3
    assert config.cooldown_seconds == 5
    assert config.timeout_seconds == 1800
    assert config.max_memory_mb == 4096
    assert config.max_retries == 3
    assert config.continue_on_error is True
    assert config.save_extracted_text is False
    assert config.compression is True


def test_msgspec_serialization() -> None:
    result = BenchmarkResult(
        file_path="test.pdf",
        file_size=1024,
        file_type=FileType.PDF,
        category=DocumentCategory.SMALL,
        framework=Framework.KREUZBERG_V4_SYNC,
        iteration=1,
        extraction_time=1.5,
        peak_memory_mb=256.0,
        avg_memory_mb=200.0,
        peak_cpu_percent=80.0,
        avg_cpu_percent=60.0,
        status=ExtractionStatus.SUCCESS,
        character_count=5000,
        word_count=800,
    )

    encoded = msgspec.json.encode(result)
    assert isinstance(encoded, bytes)

    decoded = msgspec.json.decode(encoded, type=BenchmarkResult)
    assert decoded.file_path == "test.pdf"
    assert decoded.extraction_time == 1.5
    assert decoded.framework == Framework.KREUZBERG_V4_SYNC
    assert decoded.status == ExtractionStatus.SUCCESS


def test_resource_metrics_serialization() -> None:
    metrics = ResourceMetrics(
        timestamp=1234567890.0,
        cpu_percent=50.0,
        memory_rss=1024 * 1024 * 100,
        memory_vms=1024 * 1024 * 200,
        num_threads=4,
        open_files=10,
    )

    encoded = msgspec.json.encode(metrics)
    assert isinstance(encoded, bytes)

    decoded = msgspec.json.decode(encoded, type=ResourceMetrics)
    assert decoded.timestamp == 1234567890.0
    assert decoded.cpu_percent == 50.0
    assert decoded.memory_rss == 1024 * 1024 * 100


def test_benchmark_result_validation() -> None:
    result = BenchmarkResult(
        file_path="test.pdf",
        file_size=1024,
        file_type=FileType.PDF,
        category=DocumentCategory.SMALL,
        framework=Framework.KREUZBERG_V4_SYNC,
        iteration=1,
        extraction_time=1.5,
        peak_memory_mb=256.0,
        avg_memory_mb=200.0,
        peak_cpu_percent=80.0,
        avg_cpu_percent=60.0,
        status=ExtractionStatus.SUCCESS,
    )

    assert result.file_size > 0
    assert result.extraction_time > 0
    assert result.peak_memory_mb >= 0
    assert result.avg_memory_mb >= 0
    assert 0 <= result.peak_cpu_percent <= 100
    assert 0 <= result.avg_cpu_percent <= 100


def test_benchmark_result_with_error() -> None:
    result = BenchmarkResult(
        file_path="test.pdf",
        file_size=1024,
        file_type=FileType.PDF,
        category=DocumentCategory.SMALL,
        framework=Framework.KREUZBERG_V4_SYNC,
        iteration=1,
        extraction_time=0.1,
        peak_memory_mb=50.0,
        avg_memory_mb=45.0,
        peak_cpu_percent=10.0,
        avg_cpu_percent=5.0,
        status=ExtractionStatus.FAILED,
        error_type="ExtractionError",
        error_message="Failed to extract text",
    )

    assert result.status == ExtractionStatus.FAILED
    assert result.error_type == "ExtractionError"
    assert result.error_message == "Failed to extract text"
    assert result.character_count is None
    assert result.word_count is None


def test_nested_structure_serialization() -> None:
    results = []
    summaries = []

    for i in range(3):
        result = BenchmarkResult(
            file_path=f"test_{i}.pdf",
            file_size=1024 * (i + 1),
            file_type=FileType.PDF,
            category=DocumentCategory.SMALL,
            framework=Framework.KREUZBERG_V4_SYNC,
            iteration=1,
            extraction_time=1.5 + i * 0.5,
            peak_memory_mb=256.0 + i * 50,
            avg_memory_mb=200.0 + i * 40,
            peak_cpu_percent=80.0 - i * 10,
            avg_cpu_percent=60.0 - i * 5,
            status=ExtractionStatus.SUCCESS,
        )
        results.append(result)

        summary = BenchmarkSummary(
            framework=Framework.KREUZBERG_V4_SYNC,
            category=DocumentCategory.SMALL,
            total_files=1,
            successful_files=1,
            failed_files=0,
            partial_files=0,
            timeout_files=0,
            success_rate=100.0,
            avg_extraction_time=1.5 + i * 0.5,
            median_extraction_time=1.5 + i * 0.5,
            avg_peak_memory_mb=256.0 + i * 50,
            avg_cpu_percent=60.0 - i * 5,
            files_per_second=0.67,
            mb_per_second=0.67 + i * 0.1,
        )
        summaries.append(summary)

    aggregated = AggregatedResults(
        total_runs=3,
        total_files_processed=3,
        total_time_seconds=6.0,
        framework_summaries={Framework.KREUZBERG_V4_SYNC: summaries},
        category_summaries={DocumentCategory.SMALL: summaries},
        framework_category_matrix={
            f"kreuzberg_sync_small_{i}": summary for i, summary in enumerate(summaries)
        },
        failure_patterns={},
        timeout_files=[],
        performance_over_iterations={Framework.KREUZBERG_V4_SYNC: [1.5, 2.0, 2.5]},
        platform_results={"Linux": {Framework.KREUZBERG_V4_SYNC: summaries[0]}},
    )

    encoded = msgspec.json.encode(aggregated)
    decoded = msgspec.json.decode(encoded, type=AggregatedResults)

    assert decoded.total_runs == 3
    assert decoded.total_files_processed == 3
    assert len(decoded.framework_summaries[Framework.KREUZBERG_V4_SYNC]) == 3
    assert len(decoded.category_summaries[DocumentCategory.SMALL]) == 3


def test_partial_data_handling() -> None:
    result = BenchmarkResult(
        file_path="test.pdf",
        file_size=1024,
        file_type=FileType.PDF,
        category=DocumentCategory.SMALL,
        framework=Framework.KREUZBERG_V4_SYNC,
        iteration=1,
        extraction_time=0.0,
        peak_memory_mb=50.0,
        avg_memory_mb=45.0,
        peak_cpu_percent=0.0,
        avg_cpu_percent=0.0,
        status=ExtractionStatus.FAILED,
        error_type="TestError",
        error_message="Test error",
    )

    encoded = msgspec.json.encode(result)
    decoded = msgspec.json.decode(encoded, type=BenchmarkResult)

    assert decoded.status == ExtractionStatus.FAILED
    assert decoded.error_message == "Test error"
    assert decoded.character_count is None
    assert decoded.word_count is None
