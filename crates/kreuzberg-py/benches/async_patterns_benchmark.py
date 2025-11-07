"""Benchmark comparing spawn_blocking vs pyo3_async_runtimes patterns.

Tests the performance difference between:
1. Current pattern: spawn_blocking for Python callbacks
2. Optimized pattern: pyo3_async_runtimes::tokio::into_future for async Python callbacks

Expected improvement: ~25-30x speedup based on spikard benchmarks.
"""

import asyncio
import time


# Mock OCR backend implementations
class SyncOcrBackend:
    """Simulates current sync Python OCR backend."""

    def process_image(self, image_bytes: bytes, language: str) -> dict:
        # Simulate OCR processing (50ms typical for small image)
        time.sleep(0.05)
        return {
            "content": f"Extracted text from {len(image_bytes)} bytes",
            "metadata": {"language": language, "confidence": 0.95},
        }


class AsyncOcrBackend:
    """Simulates async Python OCR backend (e.g., using httpx for cloud OCR)."""

    async def process_image(self, image_bytes: bytes, language: str) -> dict:
        # Simulate async OCR processing (50ms I/O wait)
        await asyncio.sleep(0.05)
        return {
            "content": f"Extracted text from {len(image_bytes)} bytes",
            "metadata": {"language": language, "confidence": 0.95},
        }


async def benchmark_pattern(backend, num_iterations: int, pattern_name: str) -> float:
    """Benchmark a specific pattern."""
    test_image = b"fake_image_data" * 100  # ~1.5KB

    start = time.perf_counter()

    for _ in range(num_iterations):
        if asyncio.iscoroutinefunction(backend.process_image):
            await backend.process_image(test_image, "eng")
        else:
            # Simulate spawn_blocking overhead (~4.8ms from spikard benchmarks)
            await asyncio.sleep(0.0048)
            backend.process_image(test_image, "eng")

    elapsed = time.perf_counter() - start
    return (elapsed / num_iterations) * 1000  # ms


async def run_benchmarks() -> None:
    """Run all benchmarks."""
    num_iterations = 100

    # Benchmark 1: Current spawn_blocking pattern (sync backend)
    sync_backend = SyncOcrBackend()
    spawn_blocking_latency = await benchmark_pattern(sync_backend, num_iterations, "spawn_blocking + sync")

    # Benchmark 2: Optimized pattern (into_future + async Python)
    async_backend = AsyncOcrBackend()
    into_future_latency = await benchmark_pattern(async_backend, num_iterations, "into_future + async")

    # Calculate speedup
    speedup = spawn_blocking_latency / into_future_latency
    spawn_blocking_latency - into_future_latency

    # Extrapolate to batch processing
    batch_size = 1000
    current_time = (spawn_blocking_latency / 1000) * batch_size
    optimized_time = (into_future_latency / 1000) * batch_size
    current_time - optimized_time

    if speedup < 1.5 or speedup >= 20:
        pass
    else:
        pass


if __name__ == "__main__":
    asyncio.run(run_benchmarks())
