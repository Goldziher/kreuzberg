"""Baseline memory benchmark for image preprocessing."""

from __future__ import annotations

import contextlib
import gc
import os
import tracemalloc
from typing import Any

from PIL import Image

from kreuzberg._types import ExtractionConfig
from kreuzberg._utils._image_preprocessing import (
    estimate_processing_time,
    get_dpi_adjustment_heuristics,
    normalize_image_dpi,
)


def get_process_memory_mb() -> float:
    """Get current process memory usage in MB."""
    try:
        import psutil

        process = psutil.Process(os.getpid())
        return process.memory_info().rss / (1024 * 1024)
    except ImportError:
        return 0.0


def create_test_image(width: int, height: int, mode: str = "RGB") -> Image.Image:
    """Create a test image of specified dimensions."""
    # Create a simple gradient pattern
    img = Image.new(mode, (width, height), color="white")

    # Add some pattern to make it more realistic
    for y in range(height):
        for x in range(0, width, 50):  # Every 50 pixels
            color = (x % 255, y % 255, (x + y) % 255) if mode == "RGB" else x % 255
            img.putpixel((x, y), color)

    return img


def measure_memory_usage(func, *args, **kwargs) -> tuple[Any, float, float]:
    """Measure memory usage of a function call."""
    # Force garbage collection before measurement
    gc.collect()

    # Start memory tracing
    tracemalloc.start()
    initial_memory = get_process_memory_mb()

    try:
        result = func(*args, **kwargs)

        # Get peak memory usage
        _current, peak = tracemalloc.get_traced_memory()
        peak_mb = peak / (1024 * 1024)

        # Get final memory
        final_memory = get_process_memory_mb()
        memory_increase = final_memory - initial_memory

        return result, peak_mb, memory_increase

    finally:
        tracemalloc.stop()
        gc.collect()


def benchmark_normalize_image_dpi() -> None:
    """Benchmark memory usage of normalize_image_dpi with different image sizes."""

    # Test configurations
    configs = {
        "default": ExtractionConfig(target_dpi=300, max_image_dimension=4096, auto_adjust_dpi=False),
        "auto_adjust": ExtractionConfig(target_dpi=300, max_image_dimension=4096, auto_adjust_dpi=True),
        "high_dpi": ExtractionConfig(target_dpi=600, max_image_dimension=8192, auto_adjust_dpi=True),
    }

    # Test image sizes (width, height)
    test_sizes = [
        (800, 600, "small"),  # ~0.5 MP
        (1920, 1080, "medium"),  # ~2.1 MP
        (3000, 2000, "large"),  # ~6 MP
        (5000, 4000, "xlarge"),  # ~20 MP
        (8000, 6000, "xxlarge"),  # ~48 MP
    ]

    for width, height, _size_name in test_sizes:
        (width * height * 3) / (1024 * 1024)

        # Test with original image
        test_image = create_test_image(width, height)
        len(test_image.tobytes()) / (1024 * 1024)

        for config in configs.values():
            try:
                result, _peak_mb, _memory_increase = measure_memory_usage(normalize_image_dpi, test_image, config)

                normalized_image, _metadata = result
                len(normalized_image.tobytes()) / (1024 * 1024)

                # Clean up
                del normalized_image

            except Exception:
                pass

            gc.collect()

        del test_image


def benchmark_image_operations() -> None:
    """Benchmark memory usage of individual image operations."""

    # Create a large test image
    width, height = 4000, 3000
    test_image = create_test_image(width, height)
    len(test_image.tobytes()) / (1024 * 1024)

    operations = [
        ("Image resize (2x)", lambda img: img.resize((width * 2, height * 2), Image.Resampling.BICUBIC)),
        ("Image resize (0.5x)", lambda img: img.resize((width // 2, height // 2), Image.Resampling.LANCZOS)),
        ("Image copy", lambda img: img.copy()),
        ("Image convert RGB", lambda img: img.convert("RGB")),
        ("Image tobytes", lambda img: img.tobytes()),
    ]

    for _op_name, operation in operations:
        try:
            result, _peak_mb, _memory_increase = measure_memory_usage(operation, test_image)

            # Clean up result
            del result
            gc.collect()

        except Exception:
            pass


def benchmark_heuristics_functions() -> None:
    """Benchmark memory usage of heuristic functions."""

    test_cases = [
        (800, 600, "small"),
        (3000, 2000, "large"),
        (8000, 6000, "xxlarge"),
    ]

    for width, height, _size_name in test_cases:
        # Test get_dpi_adjustment_heuristics
        with contextlib.suppress(Exception):
            result, peak_mb, memory_increase = measure_memory_usage(
                get_dpi_adjustment_heuristics, width, height, 150, 300, 4096
            )

        # Test estimate_processing_time
        with contextlib.suppress(Exception):
            _result, _peak_mb, _memory_increase = measure_memory_usage(
                estimate_processing_time, width, height, "tesseract"
            )


def analyze_memory_patterns() -> None:
    """Analyze memory usage patterns and identify optimization opportunities."""

    # Test progressive image size impact
    sizes = [(1000, 1000), (2000, 2000), (3000, 3000), (4000, 4000)]
    config = ExtractionConfig(target_dpi=300, max_image_dimension=4096, auto_adjust_dpi=True)

    for width, height in sizes:
        estimated_mb = (width * height * 3) / (1024 * 1024)

        test_image = create_test_image(width, height)
        try:
            result, peak_mb, _memory_increase = measure_memory_usage(normalize_image_dpi, test_image, config)

            estimated_mb / peak_mb if peak_mb > 0 else 0

            del result
        except Exception:
            pass

        del test_image
        gc.collect()


def main() -> None:
    """Run all memory baseline benchmarks."""

    benchmark_normalize_image_dpi()
    benchmark_image_operations()
    benchmark_heuristics_functions()
    analyze_memory_patterns()


if __name__ == "__main__":
    main()
