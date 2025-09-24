"""Simple memory comparison test."""

from __future__ import annotations

import gc
import os
from typing import Any

from PIL import Image

from kreuzberg._types import ExtractionConfig
from kreuzberg._utils._image_preprocessing import normalize_image_dpi as normalize_original
from kreuzberg._utils._image_preprocessing_optimized import (
    cleanup_image_memory,
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


def create_test_image(width: int, height: int) -> Image.Image:
    """Create a test image."""
    return Image.new("RGB", (width, height), color="white")


def test_single_case(width: int, height: int, implementation: str, config: ExtractionConfig) -> dict[str, Any]:
    """Test a single case and return memory stats."""
    initial_memory = get_process_memory_mb()
    gc.collect()

    try:
        test_image = create_test_image(width, height)

        if implementation == "original":
            result = normalize_original(test_image, config)
        else:
            result = normalize_optimized(test_image, config)

        final_memory = get_process_memory_mb()
        memory_increase = final_memory - initial_memory

        # Cleanup
        if hasattr(result[0], "close"):
            result[0].close()
        del test_image, result

        if implementation == "optimized":
            cleanup_image_memory()

        gc.collect()

        return {"success": True, "memory_increase": memory_increase, "final_memory": final_memory, "error": None}

    except Exception as e:
        return {"success": False, "memory_increase": 0, "final_memory": get_process_memory_mb(), "error": str(e)}


def main() -> None:
    """Run simple memory comparison."""

    # Simple config
    config = ExtractionConfig(target_dpi=300, max_image_dimension=4096, auto_adjust_dpi=False)

    # Test sizes
    test_cases = [
        (2000, 1500, "medium"),
        (4000, 3000, "large"),
        (6000, 4000, "xlarge"),
    ]

    for width, height, _size_name in test_cases:
        (width * height * 3) / (1024 * 1024)

        # Test original
        result_orig = test_single_case(width, height, "original", config)
        "✓" if result_orig["success"] else "✗"
        result_orig["error"][:25] + "..." if result_orig["error"] and len(result_orig["error"]) > 25 else (
            result_orig["error"] or ""
        )

        # Test optimized
        result_opt = test_single_case(width, height, "optimized", config)
        "✓" if result_opt["success"] else "✗"
        result_opt["error"][:25] + "..." if result_opt["error"] and len(result_opt["error"]) > 25 else (
            result_opt["error"] or ""
        )

        # Calculate improvement
        if result_orig["success"] and result_opt["success"]:
            result_orig["memory_increase"] - result_opt["memory_increase"]


if __name__ == "__main__":
    main()
