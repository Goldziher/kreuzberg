"""Performance benchmark focused on speed measurement."""

from __future__ import annotations

import gc
import statistics
import time
from typing import Any

import psutil
from PIL import Image

from kreuzberg._types import ExtractionConfig
from kreuzberg._utils._image_preprocessing import normalize_image_dpi


def measure_performance(width: int, height: int, config: ExtractionConfig, runs: int = 5) -> dict[str, Any]:
    """Measure performance with multiple runs for statistical accuracy."""
    times = []
    memory_peaks = []

    for _ in range(runs):
        image = Image.new("RGB", (width, height), color="white")

        gc.collect()
        initial_memory = psutil.Process().memory_info().rss / (1024 * 1024)

        start_time = time.perf_counter()
        try:
            result_image, _metadata = normalize_image_dpi(image, config)
            end_time = time.perf_counter()

            peak_memory = psutil.Process().memory_info().rss / (1024 * 1024)

            times.append((end_time - start_time) * 1000)
            memory_peaks.append(peak_memory - initial_memory)

            result_image.close()
        except Exception:
            times.append(float("inf"))
            memory_peaks.append(0)
        finally:
            image.close()
            gc.collect()

    valid_times = [t for t in times if t != float("inf")]

    if not valid_times:
        return {"success": False, "error": "All runs failed"}

    return {
        "success": True,
        "mean_time_ms": statistics.mean(valid_times),
        "median_time_ms": statistics.median(valid_times),
        "std_time_ms": statistics.stdev(valid_times) if len(valid_times) > 1 else 0,
        "min_time_ms": min(valid_times),
        "max_time_ms": max(valid_times),
        "mean_memory_mb": statistics.mean(memory_peaks),
        "success_rate": len(valid_times) / runs,
        "throughput_mp_per_sec": (width * height / 1_000_000) / (statistics.mean(valid_times) / 1000),
    }


def main() -> None:
    """Run performance benchmarks."""

    config = ExtractionConfig(target_dpi=300, max_image_dimension=4096, auto_adjust_dpi=True)

    sizes = [
        (1000, 1000, "1MP"),
        (1414, 1414, "2MP"),
        (1732, 1732, "3MP"),
        (2000, 2000, "4MP"),
        (2236, 2236, "5MP"),
        (2449, 2449, "6MP"),
        (2646, 2646, "7MP"),
        (2828, 2828, "8MP"),
    ]

    for width, height, _size_name in sizes:
        result = measure_performance(width, height, config)

        if result["success"]:
            pass
        else:
            pass


if __name__ == "__main__":
    main()
