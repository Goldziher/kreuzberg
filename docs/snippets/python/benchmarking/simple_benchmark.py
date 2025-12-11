```python
import time
import asyncio
from pathlib import Path
from kreuzberg import Kreuzberg, ExtractionConfig

async def benchmark_extractions():
    config = ExtractionConfig(use_cache=False)
    kreuzberg = Kreuzberg(config)
    file_path = "document.pdf"
    num_runs = 10

    start = time.perf_counter()
    for _ in range(num_runs):
        result = kreuzberg.extract_file(file_path)
    sync_duration = time.perf_counter() - start
    avg_sync = sync_duration / num_runs

    print(f"Sync extraction ({num_runs} runs):")
    print(f"  - Total time: {sync_duration:.3f}s")
    print(f"  - Average: {avg_sync:.3f}s per extraction")

    start = time.perf_counter()
    tasks = [kreuzberg.extract_file_async(file_path) for _ in range(num_runs)]
    await asyncio.gather(*tasks)
    async_duration = time.perf_counter() - start

    print(f"\nAsync extraction ({num_runs} parallel runs):")
    print(f"  - Total time: {async_duration:.3f}s")
    print(f"  - Average: {async_duration / num_runs:.3f}s per extraction")
    print(f"  - Speedup: {sync_duration / async_duration:.1f}x")

    cache_config = ExtractionConfig(use_cache=True)
    kreuzberg_cached = Kreuzberg(cache_config)

    print("\nFirst extraction (populates cache)...")
    start = time.perf_counter()
    result1 = await kreuzberg_cached.extract_file_async(file_path)
    first_duration = time.perf_counter() - start
    print(f"  - Time: {first_duration:.3f}s")

    print("Second extraction (from cache)...")
    start = time.perf_counter()
    result2 = await kreuzberg_cached.extract_file_async(file_path)
    cached_duration = time.perf_counter() - start
    print(f"  - Time: {cached_duration:.3f}s")
    print(f"  - Cache speedup: {first_duration / cached_duration:.1f}x")

if __name__ == "__main__":
    asyncio.run(benchmark_extractions())
```
