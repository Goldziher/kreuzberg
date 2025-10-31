#!/usr/bin/env python3
"""Profile PyO3 bridge for memory leaks and overhead.

Tests:
1. Memory overhead of Rust↔Python conversions
2. Memory leaks in long-running scenarios
3. Performance comparison: PyO3 bridge vs pure Rust
4. Reference counting and GIL handling
"""

import gc
import json
import time
import tracemalloc
from collections.abc import Callable
from pathlib import Path
from typing import Any

import psutil

# Import both Python and Rust implementations
from kreuzberg import extract_file_sync as py_extract_file
from kreuzberg import extract_file_sync_impl as rust_extract_file


def get_memory_mb() -> float:
    """Get current process memory in MB."""
    process = psutil.Process()
    return process.memory_info().rss / 1024 / 1024


def profile_extraction(
    func: Callable[[str], Any], file_path: Path, iterations: int = 100, label: str = ""
) -> dict[str, Any]:
    """Profile a single extraction function."""
    # Warm up
    _ = func(str(file_path))
    gc.collect()
    time.sleep(0.1)

    # Start profiling
    tracemalloc.start()
    start_mem = get_memory_mb()
    start_time = time.time()

    mem_samples = []
    for i in range(iterations):
        _ = func(str(file_path))
        if i % 10 == 0:
            mem_samples.append(get_memory_mb())

    end_time = time.time()
    end_mem = get_memory_mb()

    current, peak = tracemalloc.get_traced_memory()
    tracemalloc.stop()

    # Force GC to check for leaks
    gc.collect()
    time.sleep(0.2)
    gc_mem = get_memory_mb()

    duration = end_time - start_time
    avg_time = duration / iterations

    return {
        "label": label,
        "iterations": iterations,
        "total_duration": duration,
        "avg_time_ms": avg_time * 1000,
        "start_mem_mb": start_mem,
        "end_mem_mb": end_mem,
        "peak_mem_mb": max(mem_samples) if mem_samples else end_mem,
        "delta_mem_mb": end_mem - start_mem,
        "after_gc_mem_mb": gc_mem,
        "leak_mb": gc_mem - start_mem,
        "tracemalloc_current_mb": current / 1024 / 1024,
        "tracemalloc_peak_mb": peak / 1024 / 1024,
    }


def compare_implementations(file_path: Path, iterations: int = 100) -> dict[str, Any]:
    """Compare Python wrapper vs direct Rust implementation."""
    # Test Python wrapper
    py_results = profile_extraction(py_extract_file, file_path, iterations, label="Python Wrapper")

    # Give system time to stabilize
    time.sleep(1)
    gc.collect()

    # Test direct Rust
    rust_results = profile_extraction(rust_extract_file, file_path, iterations, label="Direct Rust")

    # Calculate overhead
    overhead_time = (py_results["avg_time_ms"] - rust_results["avg_time_ms"]) / rust_results["avg_time_ms"] * 100
    overhead_mem = py_results["leak_mb"] - rust_results["leak_mb"]

    return {
        "python": py_results,
        "rust": rust_results,
        "overhead_time_pct": overhead_time,
        "overhead_mem_mb": overhead_mem,
    }


def stress_test(file_path: Path, duration_sec: int = 60) -> None:
    """Long-running stress test to detect memory leaks."""
    get_memory_mb()
    start_time = time.time()
    iterations = 0
    mem_samples = []

    while time.time() - start_time < duration_sec:
        _ = py_extract_file(str(file_path))
        iterations += 1

        if iterations % 50 == 0:
            current_mem = get_memory_mb()
            elapsed = time.time() - start_time
            iterations / elapsed
            mem_samples.append(current_mem)

    end_time = time.time()
    get_memory_mb()

    gc.collect()
    time.sleep(0.5)
    get_memory_mb()

    duration = end_time - start_time
    iterations / duration

    if mem_samples:
        pass

    # Check for continuous growth (leak indicator)
    if len(mem_samples) >= 3:
        growth = mem_samples[-1] - mem_samples[0]
        if growth > 10:  # More than 10MB growth
            pass
        else:
            pass


def main() -> None:
    """Run all profiling tests."""
    # Find test documents
    test_dir = Path("test_documents")

    # Test files of different sizes
    test_files = [
        test_dir / "documents" / "fake.docx",  # Small
        test_dir / "documents" / "lorem_ipsum.docx",  # Medium
        test_dir / "pdf" / "fake_memo.pdf",  # Small PDF
    ]

    # Filter existing files
    test_files = [f for f in test_files if f.exists()]

    if not test_files:
        return

    # Test each file
    for test_file in test_files:
        # Quick comparison
        results = compare_implementations(test_file, iterations=50)

        # Save results
        output_file = Path(f"results/memory_profile/pyo3_bridge_{test_file.stem}.json")
        output_file.parent.mkdir(parents=True, exist_ok=True)
        output_file.write_text(json.dumps(results, indent=2))

    # Stress test on smallest file
    if test_files:
        stress_test(test_files[0], duration_sec=30)


if __name__ == "__main__":
    main()
