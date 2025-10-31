import asyncio
import time

import pytest

from src.profiler import (
    AsyncPerformanceProfiler,
    PerformanceMetrics,
    ResourceMonitor,
    profile_performance,
)


def test_resource_monitor_initialization() -> None:
    monitor = ResourceMonitor(sampling_interval_ms=100)

    assert monitor.sampling_interval == 0.1
    assert monitor.metrics_buffer == []
    assert not monitor.monitoring
    assert monitor.process is not None


@pytest.mark.asyncio
async def test_resource_monitor_async_context() -> None:
    monitor = ResourceMonitor(sampling_interval_ms=50)

    await monitor.start()
    await asyncio.sleep(0.1)
    metrics = await monitor.stop()

    assert isinstance(metrics, PerformanceMetrics)
    assert metrics.extraction_time >= 0
    assert metrics.peak_memory_mb >= 0
    assert metrics.avg_memory_mb >= 0


def test_profile_performance_context_manager() -> None:
    def short_task() -> str:
        time.sleep(0.1)
        return "completed"

    with profile_performance(sampling_interval_ms=50) as metrics:
        result = short_task()

    assert result == "completed"
    assert isinstance(metrics, PerformanceMetrics)
    assert metrics.extraction_time >= 0.1
    assert metrics.peak_memory_mb >= 0
    assert len(metrics.samples) > 0


@pytest.mark.asyncio
async def test_async_performance_profiler() -> None:
    async def short_async_task() -> str:
        await asyncio.sleep(0.1)
        return "completed"

    async with AsyncPerformanceProfiler(sampling_interval_ms=50) as metrics:
        result = await short_async_task()

    assert result == "completed"
    assert isinstance(metrics, PerformanceMetrics)
    assert metrics.extraction_time >= 0.001
    assert metrics.peak_memory_mb >= 0
    assert len(metrics.samples) >= 0


def test_performance_metrics_creation() -> None:
    metrics = PerformanceMetrics(
        extraction_time=1.5,
        peak_memory_mb=256.0,
        avg_memory_mb=200.0,
        peak_cpu_percent=80.0,
        avg_cpu_percent=60.0,
    )

    assert metrics.extraction_time == 1.5
    assert metrics.peak_memory_mb == 256.0
    assert metrics.avg_memory_mb == 200.0
    assert metrics.peak_cpu_percent == 80.0
    assert metrics.avg_cpu_percent == 60.0
    assert metrics.total_io_read_mb is None
    assert metrics.total_io_write_mb is None
    assert metrics.samples == []


def test_profile_performance_memory_tracking() -> None:
    def memory_task() -> list[bytes]:
        return [b"x" * 1000 for _i in range(1000)]

    with profile_performance(sampling_interval_ms=10) as metrics:
        result = memory_task()

    assert len(result) == 1000
    assert metrics.peak_memory_mb >= 0
    assert metrics.avg_memory_mb >= 0
    assert len(metrics.samples) >= 1


def test_resource_monitor_psutil_error() -> None:
    monitor = ResourceMonitor()
    assert monitor is not None
    assert monitor.process is not None


def test_profile_performance_fast_operation() -> None:
    def fast_task() -> int:
        return 42

    with profile_performance(sampling_interval_ms=50) as metrics:
        result = fast_task()

    assert result == 42
    assert isinstance(metrics, PerformanceMetrics)
    assert metrics.extraction_time >= 0
    assert metrics.peak_memory_mb >= 0
    assert len(metrics.samples) >= 1


@pytest.mark.asyncio
async def test_resource_monitor_no_samples() -> None:
    monitor = ResourceMonitor(sampling_interval_ms=1000)

    await monitor.start()
    metrics = await monitor.stop()

    assert isinstance(metrics, PerformanceMetrics)
    assert metrics.extraction_time >= 0
    assert metrics.peak_memory_mb >= 0
    assert metrics.avg_memory_mb >= 0


def test_profile_performance_exception_handling() -> None:
    def failing_task() -> None:
        raise ValueError("Test error")

    with (
        pytest.raises(ValueError, match="Test error"),
        profile_performance() as metrics,
    ):
        failing_task()

    assert isinstance(metrics, PerformanceMetrics)
    assert metrics.extraction_time >= 0


@pytest.mark.asyncio
async def test_async_profiler_exception_handling() -> None:
    async def failing_async_task() -> None:
        raise ValueError("Async test error")

    with pytest.raises(ValueError, match="Async test error"):
        async with AsyncPerformanceProfiler() as metrics:
            await failing_async_task()

    assert isinstance(metrics, PerformanceMetrics)
    assert metrics.extraction_time >= 0


def test_profile_performance_multiple_samples() -> None:
    def longer_task() -> str:
        time.sleep(0.2)
        return "done"

    with profile_performance(sampling_interval_ms=20) as metrics:
        result = longer_task()

    assert result == "done"
    assert metrics.extraction_time >= 0.2
    assert len(metrics.samples) >= 2


@pytest.mark.asyncio
async def test_async_profiler_concurrent_operations() -> None:
    async def task_a() -> int:
        await asyncio.sleep(0.1)
        return 1

    async def task_b() -> int:
        await asyncio.sleep(0.1)
        return 2

    async with AsyncPerformanceProfiler(sampling_interval_ms=25) as metrics:
        results = await asyncio.gather(task_a(), task_b())

    assert list(results) == [1, 2]
    assert metrics.extraction_time >= 0.05
    assert len(metrics.samples) >= 0


def test_performance_metrics_with_io_data() -> None:
    metrics = PerformanceMetrics(
        extraction_time=2.0,
        peak_memory_mb=512.0,
        avg_memory_mb=400.0,
        peak_cpu_percent=90.0,
        avg_cpu_percent=70.0,
        total_io_read_mb=10.5,
        total_io_write_mb=5.2,
        startup_time=0.5,
    )

    assert metrics.total_io_read_mb == 10.5
    assert metrics.total_io_write_mb == 5.2
    assert metrics.startup_time == 0.5


@pytest.mark.asyncio
async def test_resource_monitor_start_stop_multiple() -> None:
    monitor = ResourceMonitor(sampling_interval_ms=50)

    await monitor.start()
    await asyncio.sleep(0.05)
    metrics1 = await monitor.stop()

    await monitor.start()
    await asyncio.sleep(0.05)
    metrics2 = await monitor.stop()

    assert isinstance(metrics1, PerformanceMetrics)
    assert isinstance(metrics2, PerformanceMetrics)
    assert metrics1.extraction_time >= 0
    assert metrics2.extraction_time >= 0
