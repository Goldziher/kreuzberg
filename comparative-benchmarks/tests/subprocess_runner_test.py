import tempfile
from pathlib import Path

from src.subprocess_runner import ProcessResourceMetrics, SubprocessRunner


def test_subprocess_runner_resource_monitoring() -> None:
    runner = SubprocessRunner(timeout=30.0, monitoring_interval_ms=100)

    with tempfile.NamedTemporaryFile(mode="w", suffix=".txt", delete=False) as f:
        f.write("Simple test content for extraction")
        test_file_path = f.name

    try:
        result = runner.extract_with_crash_detection(
            "kreuzberg_v4_sync", test_file_path
        )

        assert result.resource_metrics is not None
        assert isinstance(result.resource_metrics, ProcessResourceMetrics)

        metrics = result.resource_metrics
        assert metrics.sample_count >= 0
        assert metrics.monitoring_duration >= 0
        assert metrics.peak_memory_mb >= 0
        assert metrics.avg_memory_mb >= 0
        assert metrics.peak_cpu_percent >= 0
        assert metrics.avg_cpu_percent >= 0
        assert metrics.baseline_memory_mb >= 0
        assert metrics.baseline_cpu_percent >= 0

    finally:
        Path(test_file_path).unlink()


def test_subprocess_runner_baseline_establishment() -> None:
    runner = SubprocessRunner(timeout=30.0)

    assert runner._baseline_memory_mb == 0.0
    assert runner._baseline_cpu_percent == 0.0

    runner._establish_system_baseline(duration_seconds=0.1)

    assert runner._baseline_memory_mb > 0.0
    assert runner._baseline_cpu_percent >= 0.0


def test_subprocess_runner_timeout_with_resource_metrics() -> None:
    runner = SubprocessRunner(timeout=0.1, monitoring_interval_ms=50)

    with tempfile.NamedTemporaryFile(mode="w", suffix=".txt", delete=False) as f:
        f.write("Test content")
        test_file_path = f.name

    try:
        result = runner.extract_with_crash_detection(
            "kreuzberg_v4_sync", test_file_path
        )

        assert result.success is False
        assert result.error_type == "TimeoutError"

        if result.resource_metrics is not None:
            assert isinstance(result.resource_metrics, ProcessResourceMetrics)

    finally:
        Path(test_file_path).unlink()


def test_subprocess_runner_initialization() -> None:
    runner1 = SubprocessRunner()
    assert runner1.timeout == 300.0
    assert runner1.monitoring_interval == 0.05

    runner2 = SubprocessRunner(timeout=60.0, monitoring_interval_ms=25)
    assert runner2.timeout == 60.0
    assert runner2.monitoring_interval == 0.025


def test_resource_metrics_structure() -> None:
    metrics = ProcessResourceMetrics(
        peak_memory_mb=100.5,
        avg_memory_mb=75.2,
        peak_cpu_percent=80.0,
        avg_cpu_percent=45.5,
        baseline_memory_mb=50.0,
        baseline_cpu_percent=5.0,
        monitoring_duration=2.5,
        sample_count=25,
    )

    assert metrics.peak_memory_mb == 100.5
    assert metrics.avg_memory_mb == 75.2
    assert metrics.peak_cpu_percent == 80.0
    assert metrics.avg_cpu_percent == 45.5
    assert metrics.baseline_memory_mb == 50.0
    assert metrics.baseline_cpu_percent == 5.0
    assert metrics.monitoring_duration == 2.5
    assert metrics.sample_count == 25
    assert metrics.total_io_read_mb is None
    assert metrics.total_io_write_mb is None
