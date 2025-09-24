"""Baseline memory and performance benchmarks for image preprocessing."""

from __future__ import annotations

import gc
import time
from typing import Any

import psutil
from PIL import Image

from kreuzberg._types import ExtractionConfig
from kreuzberg._utils._image_preprocessing import normalize_image_dpi


def get_memory_mb() -> float:
    """Get current process memory usage in MB."""
    return float(psutil.Process().memory_info().rss / (1024 * 1024))


def benchmark_implementation(width: int, height: int, config: ExtractionConfig) -> dict[str, Any]:
    """Benchmark the image preprocessing implementation."""
    image = Image.new("RGB", (width, height), color="white")

    gc.collect()
    initial_memory = get_memory_mb()
    start_time = time.perf_counter()

    try:
        result_image, metadata = normalize_image_dpi(image, config)

        end_time = time.perf_counter()
        peak_memory = get_memory_mb()

        result_image.close()
        image.close()

        return {
            "success": True,
            "processing_time_ms": (end_time - start_time) * 1000,
            "memory_increase_mb": peak_memory - initial_memory,
            "final_dpi": metadata.final_dpi,
            "scale_factor": metadata.scale_factor,
            "auto_adjusted": metadata.auto_adjusted,
            "resample_method": getattr(metadata, "resample_method", "N/A"),
            "dimension_clamped": getattr(metadata, "dimension_clamped", False),
            "new_dimensions": getattr(metadata, "new_dimensions", None),
        }
    except Exception as e:
        return {
            "success": False,
            "error": str(e),
            "processing_time_ms": (time.perf_counter() - start_time) * 1000,
            "memory_increase_mb": get_memory_mb() - initial_memory,
        }


def main() -> None:
    """Run baseline benchmarks."""

    configs = [
        ExtractionConfig(target_dpi=150, max_image_dimension=2048, auto_adjust_dpi=False),
        ExtractionConfig(target_dpi=300, max_image_dimension=4096, auto_adjust_dpi=False),
        ExtractionConfig(target_dpi=300, max_image_dimension=4096, auto_adjust_dpi=True),
    ]

    sizes = [
        (1000, 1000, "1MP"),
        (2000, 1500, "3MP"),
        (3000, 2000, "6MP"),
        (4000, 3000, "12MP"),
    ]

    for width, height, _size_name in sizes:
        for config in configs:
            result = benchmark_implementation(width, height, config)

            "✓" if result["success"] else "✗"
            f"{result['processing_time_ms']:.1f}"
            f"{result['memory_increase_mb']:.1f}"

            result["final_dpi"] if result["success"] else f"ERROR: {result['error'][:20]}..."

            gc.collect()


if __name__ == "__main__":
    main()
