"""Compare memory usage between original and optimized image preprocessing."""

from __future__ import annotations

import gc
import os
import tracemalloc
from typing import Any

from PIL import Image

from kreuzberg._types import ExtractionConfig
from kreuzberg._utils._image_preprocessing import normalize_image_dpi as normalize_original
from kreuzberg._utils._image_preprocessing_optimized import (
    cleanup_image_memory,
    get_image_memory_stats,
)
from kreuzberg._utils._image_preprocessing_optimized import (
    normalize_image_dpi_memory_optimized as normalize_optimized,
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
    img = Image.new(mode, (width, height), color="white")

    for y in range(0, height, 100):
        for x in range(0, width, 100):
            color = (x % 255, y % 255, (x + y) % 255) if mode == "RGB" else x % 255
            img.putpixel((x, y), color)

    return img


def measure_memory_usage(func, *args, **kwargs) -> tuple[Any, float, float, float]:
    """Measure memory usage of a function call."""
    gc.collect()

    tracemalloc.start()
    initial_memory = get_process_memory_mb()

    try:
        result = func(*args, **kwargs)

        _current, peak = tracemalloc.get_traced_memory()
        peak_mb = peak / (1024 * 1024)

        final_memory = get_process_memory_mb()
        memory_increase = final_memory - initial_memory

        return result, peak_mb, memory_increase, final_memory

    finally:
        tracemalloc.stop()


def compare_implementations() -> None:
    """Compare memory usage between original and optimized implementations."""

    configs = {
        "default": ExtractionConfig(target_dpi=300, max_image_dimension=4096, auto_adjust_dpi=False),
        "auto_adjust": ExtractionConfig(target_dpi=300, max_image_dimension=4096, auto_adjust_dpi=True),
        "high_dpi": ExtractionConfig(target_dpi=600, max_image_dimension=8192, auto_adjust_dpi=True),
    }

    test_sizes = [
        (1920, 1080, "medium"),
        (3000, 2000, "large"),
        (5000, 4000, "xlarge"),
        (6000, 4000, "xxlarge"),
    ]

    for width, height, _size_name in test_sizes:
        (width * height * 3) / (1024 * 1024)

        for config in configs.values():
            test_image_orig = create_test_image(width, height)
            gc.collect()
            get_process_memory_mb()

            try:
                result_orig, _peak_orig, increase_orig, _final_orig = measure_memory_usage(
                    normalize_original, test_image_orig, config
                )
                success_orig = True
            except Exception as e:
                success_orig = False
                str(e)
                increase_orig = 0

            del test_image_orig
            if success_orig and result_orig and hasattr(result_orig[0], "close"):
                result_orig[0].close()
            gc.collect()

            test_image_opt = create_test_image(width, height)
            gc.collect()

            try:
                result_opt, _peak_opt, increase_opt, _final_opt = measure_memory_usage(
                    normalize_optimized, test_image_opt, config
                )
                success_opt = True
            except Exception as e:
                success_opt = False
                str(e)
                increase_opt = 0

            if success_orig and success_opt and increase_orig > 0:
                ((increase_orig - increase_opt) / increase_orig) * 100
            else:
                pass

            if success_orig:
                pass
            else:
                pass

            if success_opt:
                pass
            else:
                pass

            del test_image_opt
            if success_opt and result_opt and hasattr(result_opt[0], "close"):
                result_opt[0].close()

            cleanup_image_memory()
            gc.collect()


def test_memory_limits() -> None:
    """Test memory limit enforcement in optimized version."""

    test_cases = [
        (10000, 8000, "huge"),
        (15000, 10000, "massive"),
        (20000, 15000, "extreme"),
    ]

    config = ExtractionConfig(target_dpi=300, max_image_dimension=8192, auto_adjust_dpi=True)

    for width, height, _size_name in test_cases:
        (width * height * 3) / (1024 * 1024)

        try:
            test_image = create_test_image(min(width, 2000), min(height, 2000))

            result, _peak_mb, _increase_mb, _final_mb = measure_memory_usage(normalize_optimized, test_image, config)

            if hasattr(result[0], "close"):
                result[0].close()
            del test_image

        except Exception:
            pass

        cleanup_image_memory()
        gc.collect()


def test_progressive_scaling() -> None:
    """Test progressive scaling for extreme scale factors."""

    test_image = create_test_image(2000, 1500)

    scale_tests = [
        (ExtractionConfig(target_dpi=150, max_image_dimension=4096, auto_adjust_dpi=False), "downscale_2x"),
        (ExtractionConfig(target_dpi=600, max_image_dimension=8192, auto_adjust_dpi=False), "upscale_2x"),
        (ExtractionConfig(target_dpi=900, max_image_dimension=12288, auto_adjust_dpi=False), "upscale_3x"),
    ]

    for config, _test_name in scale_tests:
        try:
            result, _peak_mb, _increase_mb, _final_mb = measure_memory_usage(normalize_optimized, test_image, config)

            if hasattr(result[0], "close"):
                result[0].close()

        except Exception:
            pass

        cleanup_image_memory()
        gc.collect()

    del test_image


def main() -> None:
    """Run memory comparison benchmarks."""

    get_image_memory_stats()

    compare_implementations()
    test_memory_limits()
    test_progressive_scaling()

    cleanup_image_memory()
    gc.collect()

    get_image_memory_stats()


if __name__ == "__main__":
    main()
