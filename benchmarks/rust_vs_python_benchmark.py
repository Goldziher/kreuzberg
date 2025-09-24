"""Benchmark comparing Rust vs Python image preprocessing implementations."""

from __future__ import annotations

import gc
import time
from typing import Any

import numpy as np
import psutil
from PIL import Image

from kreuzberg._internal_bindings import ExtractionConfig as RustConfig
from kreuzberg._internal_bindings import normalize_image_dpi_rust
from kreuzberg._types import ExtractionConfig as PythonConfig
from kreuzberg._utils._image_preprocessing import normalize_image_dpi as normalize_python


def get_memory_mb() -> float:
    """Get current process memory usage in MB."""
    return float(psutil.Process().memory_info().rss / (1024 * 1024))


def pil_to_numpy(image: Image.Image) -> np.ndarray:
    """Convert PIL image to numpy array."""
    return np.array(image)


def numpy_to_pil(array: np.ndarray) -> Image.Image:
    """Convert numpy array to PIL image."""
    return Image.fromarray(array)


def benchmark_python_implementation(image: Image.Image, config: PythonConfig) -> dict[str, Any]:
    """Benchmark the Python implementation."""
    gc.collect()
    initial_memory = get_memory_mb()
    start_time = time.perf_counter()

    try:
        _result_image, metadata = normalize_python(image, config)
        end_time = time.perf_counter()
        peak_memory = get_memory_mb()

        return {
            "success": True,
            "processing_time_ms": (end_time - start_time) * 1000,
            "memory_increase_mb": peak_memory - initial_memory,
            "final_dpi": metadata.final_dpi,
            "scale_factor": metadata.scale_factor,
            "implementation": "Python",
        }
    except Exception as e:
        return {
            "success": False,
            "error": str(e),
            "processing_time_ms": (time.perf_counter() - start_time) * 1000,
            "memory_increase_mb": get_memory_mb() - initial_memory,
            "implementation": "Python",
        }


def benchmark_rust_implementation(image: Image.Image, config: RustConfig) -> dict[str, Any]:
    """Benchmark the Rust implementation."""
    # Convert PIL to numpy for Rust
    image_array = pil_to_numpy(image)

    gc.collect()
    initial_memory = get_memory_mb()
    start_time = time.perf_counter()

    try:
        _result_array, metadata = normalize_image_dpi_rust(image_array, config)
        end_time = time.perf_counter()
        peak_memory = get_memory_mb()

        return {
            "success": True,
            "processing_time_ms": (end_time - start_time) * 1000,
            "memory_increase_mb": peak_memory - initial_memory,
            "final_dpi": metadata.final_dpi,
            "scale_factor": metadata.scale_factor,
            "implementation": "Rust",
        }
    except Exception as e:
        return {
            "success": False,
            "error": str(e),
            "processing_time_ms": (time.perf_counter() - start_time) * 1000,
            "memory_increase_mb": get_memory_mb() - initial_memory,
            "implementation": "Rust",
        }


def run_comparison(runs: int = 5) -> None:
    """Run comprehensive comparison between implementations."""

    # Test configurations
    python_config = PythonConfig(target_dpi=300, max_image_dimension=4096, auto_adjust_dpi=True)
    rust_config = RustConfig(target_dpi=300, max_image_dimension=4096, auto_adjust_dpi=True)

    # Test image sizes
    sizes = [
        (1000, 1000, "1MP"),
        (2000, 1500, "3MP"),
        (3000, 2000, "6MP"),
        (4000, 3000, "12MP"),
        (5000, 4000, "20MP"),
    ]

    results = []

    for width, height, size_name in sizes:
        # Create test image
        test_image = Image.new("RGB", (width, height), color="white")

        # Benchmark Python
        python_times = []
        python_memories = []
        for _ in range(runs):
            result = benchmark_python_implementation(test_image.copy(), python_config)
            if result["success"]:
                python_times.append(result["processing_time_ms"])
                python_memories.append(result["memory_increase_mb"])
            gc.collect()

        # Benchmark Rust
        rust_times = []
        rust_memories = []
        for _ in range(runs):
            result = benchmark_rust_implementation(test_image.copy(), rust_config)
            if result["success"]:
                rust_times.append(result["processing_time_ms"])
                rust_memories.append(result["memory_increase_mb"])
            gc.collect()

        if python_times and rust_times:
            avg_python_time = sum(python_times) / len(python_times)
            avg_rust_time = sum(rust_times) / len(rust_times)
            avg_python_memory = sum(python_memories) / len(python_memories)
            avg_rust_memory = sum(rust_memories) / len(rust_memories)

            speedup = avg_python_time / avg_rust_time if avg_rust_time > 0 else 0
            memory_reduction = avg_python_memory - avg_rust_memory

            results.append(
                {
                    "size": size_name,
                    "python_time_ms": avg_python_time,
                    "rust_time_ms": avg_rust_time,
                    "python_memory_mb": avg_python_memory,
                    "rust_memory_mb": avg_rust_memory,
                    "speedup": speedup,
                    "memory_reduction": memory_reduction,
                }
            )

    # Calculate overall statistics
    if results:
        sum(r["speedup"] for r in results) / len(results)
        sum(r["memory_reduction"] for r in results) / len(results)
        max(r["speedup"] for r in results)
        min(r["speedup"] for r in results)


if __name__ == "__main__":
    run_comparison()
