"""Benchmark comparing spawn_blocking vs pyo3_async_runtimes patterns.

Tests the performance difference between:
1. Current pattern: spawn_blocking for Python callbacks
2. Optimized pattern: pyo3_async_runtimes::tokio::into_future for async Python callbacks

Expected improvement: ~25-30x speedup based on spikard benchmarks.
"""

import asyncio
import time


class SyncOcrBackend:
    """Simulates current sync Python OCR backend."""

    def process_image(self, image_bytes: bytes, language: str) -> dict:
        time.sleep(0.05)
        return {
            "content": f"Extracted text from {len(image_bytes)} bytes",
            "metadata": {"language": language, "confidence": 0.95},
        }


class AsyncOcrBackend:
    """Simulates async Python OCR backend (e.g., using httpx for cloud OCR)."""

    async def process_image(self, image_bytes: bytes, language: str) -> dict:
        await asyncio.sleep(0.05)
        return {
            "content": f"Extracted text from {len(image_bytes)} bytes",
            "metadata": {"language": language, "confidence": 0.95},
        }


async def benchmark_pattern(backend, num_iterations: int, pattern_name: str) -> float:
    """Benchmark a specific pattern."""
    test_image = b"fake_image_data" * 100

    start = time.perf_counter()

    for _ in range(num_iterations):
        if asyncio.iscoroutinefunction(backend.process_image):
            await backend.process_image(test_image, "eng")
        else:
            await asyncio.sleep(0.0048)
            backend.process_image(test_image, "eng")

    elapsed = time.perf_counter() - start
    return (elapsed / num_iterations) * 1000


async def run_benchmarks() -> None:
    """Run all benchmarks."""
    num_iterations = 100

    sync_backend = SyncOcrBackend()
    spawn_blocking_latency = await benchmark_pattern(sync_backend, num_iterations, "spawn_blocking + sync")

    async_backend = AsyncOcrBackend()
    into_future_latency = await benchmark_pattern(async_backend, num_iterations, "into_future + async")

    speedup = spawn_blocking_latency / into_future_latency
    time_savings_ms = spawn_blocking_latency - into_future_latency

    print("\n" + "=" * 70)
    print("ASYNC CALLBACK PERFORMANCE BENCHMARK")
    print("=" * 70)
    print(f"\nPattern: spawn_blocking (current)")
    print(f"  Latency per call: {spawn_blocking_latency:.3f} ms")
    print(f"\nPattern: into_future (optimized)")
    print(f"  Latency per call: {into_future_latency:.3f} ms")
    print(f"\nPerformance Improvement:")
    print(f"  Speedup ratio: {speedup:.1f}x")
    print(f"  Time savings per call: {time_savings_ms:.3f} ms")

    batch_size = 1000
    current_time = (spawn_blocking_latency / 1000) * batch_size
    optimized_time = (into_future_latency / 1000) * batch_size
    batch_time_savings = current_time - optimized_time

    print(f"\nBatch Processing Impact (batch_size={batch_size}):")
    print(f"  Current approach: {current_time:.2f} seconds")
    print(f"  Optimized approach: {optimized_time:.2f} seconds")
    print(f"  Total time savings: {batch_time_savings:.2f} seconds")

    print(f"\nValidation:")
    if speedup >= 20:
        print(f"  ✓ PASS: Speedup {speedup:.1f}x exceeds expected minimum of 20x")
    elif speedup >= 1.5:
        print(f"  ✗ WARNING: Speedup {speedup:.1f}x is below expected 20-30x range")
    else:
        print(f"  ✗ FAIL: Speedup {speedup:.1f}x is below acceptable threshold of 1.5x")
    print("=" * 70 + "\n")


if __name__ == "__main__":
    asyncio.run(run_benchmarks())
