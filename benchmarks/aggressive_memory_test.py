"""Test aggressive memory optimization approach."""

from __future__ import annotations

import gc
import os
from typing import Any

from PIL import Image

from kreuzberg._types import ExtractionConfig
from kreuzberg._utils._image_preprocessing import normalize_image_dpi as normalize_original
from kreuzberg._utils._image_preprocessing_v2 import (
    cleanup_aggressive_memory,
)
from kreuzberg._utils._image_preprocessing_v2 import (
    normalize_image_dpi_aggressive as normalize_aggressive,
)


def get_process_memory_mb() -> float:
    """Get current process memory usage in MB."""
    try:
        import psutil

        process = psutil.Process(os.getpid())
        return process.memory_info().rss / (1024 * 1024)
    except ImportError:
        return 0.0


def create_test_image(width: int, height: int) -> Image.Image:
    """Create a test image."""
    return Image.new("RGB", (width, height), color="white")


def test_memory_usage(func, image, config) -> dict[str, Any]:
    """Test memory usage of a function."""
    initial_memory = get_process_memory_mb()
    gc.collect()

    try:
        result = func(image, config)

        after_memory = get_process_memory_mb()
        memory_increase = after_memory - initial_memory

        # Clean up result
        if hasattr(result[0], "close"):
            result[0].close()

        return {
            "success": True,
            "memory_increase": memory_increase,
            "final_memory": after_memory,
            "metadata": result[1] if len(result) > 1 else {},
            "error": None,
        }

    except Exception as e:
        return {
            "success": False,
            "memory_increase": get_process_memory_mb() - initial_memory,
            "final_memory": get_process_memory_mb(),
            "metadata": {},
            "error": str(e),
        }


def main() -> None:
    """Compare original vs aggressive optimization."""

    config = ExtractionConfig(target_dpi=300, max_image_dimension=4096, auto_adjust_dpi=False)

    # Test cases with expected memory issues
    test_cases = [
        (1000, 1000, "small"),
        (2000, 1500, "medium"),
        (3000, 2000, "large"),
        (4000, 3000, "xlarge"),
        (5000, 4000, "xxlarge"),
    ]

    for width, height, _size_name in test_cases:
        (width * height * 3) / (1024 * 1024)

        # Test original version
        image_orig = create_test_image(width, height)
        result_orig = test_memory_usage(normalize_original, image_orig, config)

        "✓" if result_orig["success"] else "✗"
        result_orig["error"][:25] + "..." if result_orig["error"] and len(result_orig["error"]) > 25 else (
            result_orig["error"] or ""
        )
        getattr(result_orig["metadata"], "resample_method", "N/A")
        getattr(result_orig["metadata"], "final_dpi", "N/A")

        del image_orig
        gc.collect()

        # Test aggressive version
        image_agg = create_test_image(width, height)
        result_agg = test_memory_usage(normalize_aggressive, image_agg, config)

        "✓" if result_agg["success"] else "✗"
        result_agg["error"][:25] + "..." if result_agg["error"] and len(result_agg["error"]) > 25 else (
            result_agg["error"] or ""
        )
        getattr(result_agg["metadata"], "resample_method", "N/A")
        getattr(result_agg["metadata"], "final_dpi", "N/A")

        # Calculate improvement
        if result_orig["success"] and result_agg["success"]:
            improvement = result_orig["memory_increase"] - result_agg["memory_increase"]
            (improvement / result_orig["memory_increase"]) * 100 if result_orig["memory_increase"] != 0 else 0

        del image_agg
        cleanup_aggressive_memory()
        gc.collect()

    # Test with auto-adjust DPI to see smart DPI calculation

    auto_config = ExtractionConfig(target_dpi=300, max_image_dimension=4096, auto_adjust_dpi=True)

    for width, height, _size_name in test_cases[:3]:  # Test first 3 sizes
        image = create_test_image(width, height)
        result = test_memory_usage(normalize_aggressive, image, auto_config)

        if result["success"]:
            getattr(result["metadata"], "final_dpi", "N/A")
        else:
            pass

        del image
        cleanup_aggressive_memory()
        gc.collect()


if __name__ == "__main__":
    main()
