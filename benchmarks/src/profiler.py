from __future__ import annotations

import asyncio
import gc
import time
from contextlib import contextmanager, suppress
from dataclasses import dataclass, field
from typing import TYPE_CHECKING

import psutil

from src.logger import get_logger
from src.types import ResourceMetrics

if TYPE_CHECKING:
    import types
    from collections.abc import Iterator

logger = get_logger(__name__)


@dataclass
class PerformanceMetrics:
    extraction_time: float
    peak_memory_mb: float
    avg_memory_mb: float
    peak_cpu_percent: float
    avg_cpu_percent: float
    total_io_read_mb: float | None = None
    total_io_write_mb: float | None = None
    samples: list[ResourceMetrics] = field(default_factory=list)
    startup_time: float | None = None
    baseline_cpu_percent: float = 0.0
    baseline_memory_mb: float = 0.0
    cpu_measurement_accuracy: float | None = None


class EnhancedResourceMonitor:
    def __init__(self, sampling_interval_ms: int = 50) -> None:
        self.sampling_interval = sampling_interval_ms / 1000.0
        self.metrics_buffer: list[ResourceMetrics] = []
        self.monitoring = False
        self.process = psutil.Process()
        self._monitor_task: asyncio.Task[None] | None = None
        self._baseline_io: dict[str, int] | None = None
        self._baseline_cpu_samples: list[float] = []
        self._baseline_memory_mb: float = 0.0
        self._cpu_validation_samples: list[float] = []

    def _get_io_counters(self) -> dict[str, int] | None:
        try:
            io = self.process.io_counters()
            return {
                "read_bytes": io.read_bytes,
                "write_bytes": io.write_bytes,
                "read_count": io.read_count,
                "write_count": io.write_count,
            }
        except (AttributeError, psutil.AccessDenied):
            return None

    def _get_open_files_count(self) -> int:
        try:
            return len(self.process.open_files())
        except (psutil.AccessDenied, AttributeError):
            return 0

    async def _establish_baseline(self, duration_seconds: float = 1.0) -> None:
        logger.debug("Establishing baseline measurements", duration_seconds=f"{duration_seconds:.1f}s")

        self.process.cpu_percent(interval=None)
        await asyncio.sleep(0.1)

        baseline_start = time.time()
        baseline_samples = []

        while time.time() - baseline_start < duration_seconds:
            try:
                cpu_percent = self.process.cpu_percent(interval=self.sampling_interval)
                memory_mb = self.process.memory_info().rss / (1024 * 1024)

                baseline_samples.append({"cpu_percent": cpu_percent, "memory_mb": memory_mb, "timestamp": time.time()})

                await asyncio.sleep(0.001)

            except (psutil.NoSuchProcess, psutil.AccessDenied):
                break

        if baseline_samples:
            cpu_values = [s["cpu_percent"] for s in baseline_samples]
            memory_values = [s["memory_mb"] for s in baseline_samples]

            self._baseline_cpu_samples = cpu_values
            self._baseline_memory_mb = sum(memory_values) / len(memory_values)

            if len(cpu_values) > 1:
                cpu_std = (
                    sum((x - sum(cpu_values) / len(cpu_values)) ** 2 for x in cpu_values) / (len(cpu_values) - 1)
                ) ** 0.5
                cpu_mean = sum(cpu_values) / len(cpu_values)
                cpu_cv = cpu_std / cpu_mean if cpu_mean > 0 else 0
                self._cpu_validation_samples = [cpu_cv]

            logger.debug(
                "Baseline established",
                cpu_avg_percent=f"{sum(cpu_values) / len(cpu_values):.1f}",
                memory_avg_mb=f"{self._baseline_memory_mb:.1f}",
            )

    def _get_baseline_cpu_percent(self) -> float:
        if not self._baseline_cpu_samples:
            return 0.0
        return sum(self._baseline_cpu_samples) / len(self._baseline_cpu_samples)

    def _get_cpu_measurement_accuracy(self) -> float | None:
        if not self._cpu_validation_samples:
            return None
        return 1.0 - min(self._cpu_validation_samples[0], 1.0)

    async def _monitor_loop(self) -> None:
        self.process.cpu_percent(interval=None)
        await asyncio.sleep(0.1)

        while self.monitoring:
            try:
                cpu_percent = self.process.cpu_percent(interval=self.sampling_interval)
                mem_info = self.process.memory_info()

                io_counters = self._get_io_counters()
                io_metrics = {}
                if io_counters and self._baseline_io:
                    io_metrics = {
                        "io_read_bytes": io_counters["read_bytes"] - self._baseline_io["read_bytes"],
                        "io_write_bytes": io_counters["write_bytes"] - self._baseline_io["write_bytes"],
                        "io_read_count": io_counters["read_count"] - self._baseline_io["read_count"],
                        "io_write_count": io_counters["write_count"] - self._baseline_io["write_count"],
                    }

                metric = ResourceMetrics(
                    timestamp=time.time(),
                    cpu_percent=cpu_percent,
                    memory_rss=mem_info.rss,
                    memory_vms=mem_info.vms,
                    num_threads=self.process.num_threads(),
                    open_files=self._get_open_files_count(),
                    **io_metrics,
                )

                self.metrics_buffer.append(metric)

            except (psutil.NoSuchProcess, psutil.AccessDenied):
                break

            await asyncio.sleep(0.001)

    async def start(self) -> None:
        self.monitoring = True
        self.metrics_buffer.clear()

        await self._establish_baseline(duration_seconds=0.5)

        self._baseline_io = self._get_io_counters()
        self.process.cpu_percent(interval=None)

        try:
            baseline_metric = ResourceMetrics(
                timestamp=time.time(),
                cpu_percent=0.0,
                memory_rss=self.process.memory_info().rss,
                memory_vms=self.process.memory_info().vms,
                num_threads=self.process.num_threads(),
                open_files=self._get_open_files_count(),
            )
            self.metrics_buffer.append(baseline_metric)
        except Exception as e:
            emergency_metric = ResourceMetrics(
                timestamp=time.time(),
                cpu_percent=0.0,
                memory_rss=1024 * 1024,
                memory_vms=1024 * 1024,
                num_threads=1,
                open_files=10,
            )
            self.metrics_buffer.append(emergency_metric)
            logger.warning("Failed to collect baseline metric", error=str(e))

        self._monitor_task = asyncio.create_task(self._monitor_loop())

    async def stop(self) -> PerformanceMetrics:
        self.monitoring = False

        if self._monitor_task:
            await self._monitor_task

        return self._calculate_metrics()

    def _calculate_metrics(self) -> PerformanceMetrics:
        if not self.metrics_buffer:
            try:
                mem_info = self.process.memory_info()
                emergency_sample = ResourceMetrics(
                    timestamp=time.time(),
                    cpu_percent=0.0,
                    memory_rss=mem_info.rss,
                    memory_vms=mem_info.vms,
                    num_threads=self.process.num_threads(),
                    open_files=self._get_open_files_count(),
                )
                self.metrics_buffer.append(emergency_sample)
            except Exception as e:
                logger.warning("Failed to create emergency sample", error=str(e))

            if not self.metrics_buffer:
                return PerformanceMetrics(
                    extraction_time=0,
                    peak_memory_mb=1.0,
                    avg_memory_mb=1.0,
                    peak_cpu_percent=0,
                    avg_cpu_percent=0,
                    samples=[],
                )

        start_time = self.metrics_buffer[0].timestamp
        end_time = self.metrics_buffer[-1].timestamp
        extraction_time = max(end_time - start_time, 0.001)

        memory_samples = [m.memory_rss / (1024 * 1024) for m in self.metrics_buffer]
        peak_memory_mb = max(memory_samples)
        avg_memory_mb = sum(memory_samples) / len(memory_samples)

        baseline_memory_mb = self._baseline_memory_mb
        peak_memory_mb = max(0, peak_memory_mb - baseline_memory_mb)
        avg_memory_mb = max(0, avg_memory_mb - baseline_memory_mb)

        cpu_samples = [m.cpu_percent for m in self.metrics_buffer]
        peak_cpu_percent = max(cpu_samples) if cpu_samples else 0
        avg_cpu_percent = sum(cpu_samples) / len(cpu_samples) if cpu_samples else 0

        baseline_cpu_percent = self._get_baseline_cpu_percent()
        peak_cpu_percent = max(0, peak_cpu_percent - baseline_cpu_percent)
        avg_cpu_percent = max(0, avg_cpu_percent - baseline_cpu_percent)

        total_io_read_mb = None
        total_io_write_mb = None
        if self.metrics_buffer and self.metrics_buffer[-1].io_read_bytes is not None:
            total_io_read_mb = self.metrics_buffer[-1].io_read_bytes / (1024 * 1024)
        if self.metrics_buffer and self.metrics_buffer[-1].io_write_bytes is not None:
            total_io_write_mb = self.metrics_buffer[-1].io_write_bytes / (1024 * 1024)

        return PerformanceMetrics(
            extraction_time=extraction_time,
            peak_memory_mb=peak_memory_mb,
            avg_memory_mb=avg_memory_mb,
            peak_cpu_percent=peak_cpu_percent,
            avg_cpu_percent=avg_cpu_percent,
            total_io_read_mb=total_io_read_mb,
            total_io_write_mb=total_io_write_mb,
            samples=self.metrics_buffer.copy(),
            baseline_cpu_percent=baseline_cpu_percent,
            baseline_memory_mb=baseline_memory_mb,
            cpu_measurement_accuracy=self._get_cpu_measurement_accuracy(),
        )


@contextmanager
def profile_performance(sampling_interval_ms: int = 50) -> Iterator[PerformanceMetrics]:  # noqa: ARG001
    gc.collect()

    process = psutil.Process()
    start_time = time.time()

    process.cpu_percent(interval=None)
    baseline_memory = process.memory_info().rss
    baseline_io = None

    with suppress(AttributeError, psutil.AccessDenied):
        baseline_io = process.io_counters()._asdict()

    samples = []

    def collect_sample() -> ResourceMetrics | None:
        try:
            cpu = process.cpu_percent(interval=None)
            mem_info = process.memory_info()

            io_metrics = {}
            if baseline_io:
                try:
                    current_io = process.io_counters()._asdict()
                    io_metrics = {
                        "io_read_bytes": current_io["read_bytes"] - baseline_io["read_bytes"],
                        "io_write_bytes": current_io["write_bytes"] - baseline_io["write_bytes"],
                        "io_read_count": current_io["read_count"] - baseline_io["read_count"],
                        "io_write_count": current_io["write_count"] - baseline_io["write_count"],
                    }
                except (AttributeError, psutil.AccessDenied):
                    pass

            try:
                open_files = len(process.open_files())
            except (psutil.AccessDenied, AttributeError):
                open_files = 0

            return ResourceMetrics(
                timestamp=time.time(),
                cpu_percent=cpu,
                memory_rss=mem_info.rss,
                memory_vms=mem_info.vms,
                num_threads=process.num_threads(),
                open_files=open_files,
                **io_metrics,
            )
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            return None

    metrics = PerformanceMetrics(
        extraction_time=0,
        peak_memory_mb=0,
        avg_memory_mb=0,
        peak_cpu_percent=0,
        avg_cpu_percent=0,
        samples=samples,
    )

    if baseline_sample := collect_sample():
        samples.append(baseline_sample)

    try:
        yield metrics

        if final_sample := collect_sample():
            samples.append(final_sample)

        if len(samples) < 3:
            for _ in range(3):
                if sample := collect_sample():
                    samples.append(sample)
                time.sleep(0.01)

    finally:
        end_time = time.time()

        if samples:
            memory_samples = [s.memory_rss / (1024 * 1024) for s in samples]
            cpu_samples = [s.cpu_percent for s in samples if s.cpu_percent > 0]

            metrics.extraction_time = end_time - start_time
            metrics.peak_memory_mb = max(memory_samples)
            metrics.avg_memory_mb = sum(memory_samples) / len(memory_samples)
            metrics.peak_cpu_percent = max(cpu_samples) if cpu_samples else 0
            metrics.avg_cpu_percent = sum(cpu_samples) / len(cpu_samples) if cpu_samples else 0

            if samples and samples[-1].io_read_bytes is not None:
                metrics.total_io_read_mb = samples[-1].io_read_bytes / (1024 * 1024)
            if samples and samples[-1].io_write_bytes is not None:
                metrics.total_io_write_mb = samples[-1].io_write_bytes / (1024 * 1024)
        else:
            try:
                final_memory = process.memory_info().rss
                metrics.extraction_time = end_time - start_time
                metrics.peak_memory_mb = final_memory / (1024 * 1024)
                metrics.avg_memory_mb = (baseline_memory + final_memory) / 2 / (1024 * 1024)
                fallback_sample = collect_sample()
                if fallback_sample:
                    samples.append(fallback_sample)
            except Exception:
                metrics.extraction_time = end_time - start_time
                metrics.peak_memory_mb = 1.0
                metrics.avg_memory_mb = 1.0


class AsyncPerformanceProfiler:
    def __init__(self, sampling_interval_ms: int = 50) -> None:
        self.monitor = EnhancedResourceMonitor(sampling_interval_ms)
        self.metrics: PerformanceMetrics | None = None

    async def __aenter__(self) -> PerformanceMetrics:
        gc.collect()

        await self.monitor.start()

        self.metrics = PerformanceMetrics(
            extraction_time=0,
            peak_memory_mb=0,
            avg_memory_mb=0,
            peak_cpu_percent=0,
            avg_cpu_percent=0,
            samples=self.monitor.metrics_buffer,
        )
        return self.metrics

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: types.TracebackType | None,
    ) -> None:
        result = await self.monitor.stop()

        if self.metrics:
            self.metrics.extraction_time = result.extraction_time
            self.metrics.peak_memory_mb = result.peak_memory_mb
            self.metrics.avg_memory_mb = result.avg_memory_mb
            self.metrics.peak_cpu_percent = result.peak_cpu_percent
            self.metrics.avg_cpu_percent = result.avg_cpu_percent
            self.metrics.total_io_read_mb = result.total_io_read_mb
            self.metrics.total_io_write_mb = result.total_io_write_mb
            self.metrics.startup_time = result.startup_time
            self.metrics.baseline_cpu_percent = result.baseline_cpu_percent
            self.metrics.baseline_memory_mb = result.baseline_memory_mb
            self.metrics.cpu_measurement_accuracy = result.cpu_measurement_accuracy
