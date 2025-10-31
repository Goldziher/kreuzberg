import asyncio

import pytest

from src.profiler import (
    AsyncPerformanceProfiler,
    ResourceMonitor,
)


@pytest.mark.asyncio
async def test_resource_monitor_baseline_establishment() -> None:
    monitor = ResourceMonitor(sampling_interval_ms=50)

    await monitor._establish_baseline(duration_seconds=0.2)

    assert monitor._baseline_memory_mb > 0
    assert len(monitor._baseline_cpu_samples) > 0
    assert monitor._get_baseline_cpu_percent() >= 0


@pytest.mark.asyncio
async def test_resource_monitor_baseline_subtraction() -> None:
    monitor = ResourceMonitor(sampling_interval_ms=100)

    await monitor.start()
    await asyncio.sleep(0.2)
    metrics = await monitor.stop()

    assert metrics.baseline_cpu_percent >= 0
    assert metrics.baseline_memory_mb > 0
    assert metrics.cpu_measurement_accuracy is not None

    assert metrics.peak_cpu_percent >= 0
    assert metrics.avg_cpu_percent >= 0
    assert metrics.peak_memory_mb >= 0
    assert metrics.avg_memory_mb >= 0


@pytest.mark.asyncio
async def test_cpu_measurement_accuracy_calculation() -> None:
    monitor = ResourceMonitor(sampling_interval_ms=50)

    await monitor._establish_baseline(duration_seconds=0.3)

    accuracy = monitor._get_cpu_measurement_accuracy()
    assert accuracy is not None
    assert 0.0 <= accuracy <= 1.0


@pytest.mark.asyncio
async def test_async_profiler_with_baseline() -> None:
    async def simple_task() -> str:
        await asyncio.sleep(0.1)
        return "done"

    async with AsyncPerformanceProfiler(sampling_interval_ms=100) as metrics:
        result = await simple_task()

    assert result == "done"
    assert metrics.baseline_cpu_percent >= 0
    assert metrics.baseline_memory_mb > 0
    assert metrics.cpu_measurement_accuracy is not None


@pytest.mark.asyncio
async def test_baseline_with_memory_allocation() -> None:
    monitor = ResourceMonitor(sampling_interval_ms=50)

    await monitor.start()

    data = [b"x" * 1000 for _ in range(1000)]

    await asyncio.sleep(0.2)
    metrics = await monitor.stop()

    assert metrics.peak_memory_mb >= 0
    assert metrics.baseline_memory_mb > 0

    del data


def test_baseline_cpu_percent_empty_samples() -> None:
    monitor = ResourceMonitor()

    baseline_cpu = monitor._get_baseline_cpu_percent()
    assert baseline_cpu == 0.0


def test_cpu_measurement_accuracy_no_validation() -> None:
    monitor = ResourceMonitor()

    accuracy = monitor._get_cpu_measurement_accuracy()
    assert accuracy is None


@pytest.mark.asyncio
async def test_baseline_establishment_with_errors() -> None:
    monitor = ResourceMonitor(sampling_interval_ms=10)

    await monitor._establish_baseline(duration_seconds=0.01)

    assert monitor._baseline_memory_mb >= 0
    assert isinstance(monitor._baseline_cpu_samples, list)
