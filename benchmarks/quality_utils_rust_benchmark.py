"""Benchmark script for Rust quality utils implementation."""

from __future__ import annotations

import json
import time
from pathlib import Path
from typing import Any

from kreuzberg._rust_bridge import (
    calculate_quality_score,
    clean_extracted_text,
    normalize_spaces,
)

TEST_FILES_DIR = Path(__file__).parent.parent / "tests" / "test_source_files"
BENCHMARK_DIR = Path(__file__).parent
BASELINE_FILE = BENCHMARK_DIR / "baselines" / "quality_utils_baseline.json"
RESULTS_DIR = BENCHMARK_DIR / "results"
RESULTS_DIR.mkdir(exist_ok=True)


def load_test_data() -> dict[str, str]:
    """Load test data from real files."""
    test_data = {}

    pdf_files = [
        TEST_FILES_DIR / "sample.pdf",
        TEST_FILES_DIR / "multipage_sample.pdf",
    ]

    for pdf_path in pdf_files:
        if pdf_path.exists():
            with pdf_path.open("rb") as f:
                test_data[pdf_path.name] = f.read(10240).decode("utf-8", errors="ignore")

    text_files = TEST_FILES_DIR.glob("*.txt")
    for txt_path in text_files:
        with txt_path.open(encoding="utf-8", errors="ignore") as f:
            test_data[txt_path.name] = f.read()

    test_data["small_text"] = "This is a small test text with some words."
    test_data["medium_text"] = " ".join(["This is a medium length text."] * 100)
    test_data["large_text"] = " ".join(["This is a large text document with many words."] * 1000)
    test_data["ocr_artifacts"] = "T h i s   h a s   s c a t t e r e d   c h a r s... ... ..."
    test_data["navigation"] = "Skip to main content\nHome > Products > Item\nPage 1 of 10"

    return test_data


def benchmark_function(func: Any, data: str, iterations: int = 100) -> dict[str, float]:
    """Benchmark a single function."""
    times = []
    for _ in range(iterations):
        start = time.perf_counter()
        func(data)
        end = time.perf_counter()
        times.append((end - start) * 1000)

    times.sort()
    return {
        "min_ms": times[0],
        "max_ms": times[-1],
        "avg_ms": sum(times) / len(times),
        "median_ms": times[len(times) // 2],
        "p95_ms": times[int(len(times) * 0.95)],
        "throughput_mb_per_sec": len(data) / (1024 * 1024) / (times[len(times) // 2] / 1000),
    }


def run_benchmarks() -> dict[str, Any]:
    """Run all benchmarks."""
    test_data = load_test_data()
    results: dict[str, Any] = {}

    functions = {
        "calculate_quality_score": calculate_quality_score,
        "clean_extracted_text": clean_extracted_text,
        "normalize_spaces": normalize_spaces,
    }

    for func_name, func in functions.items():
        results[func_name] = {}

        for data_name, data in test_data.items():
            iterations = 1000 if len(data) < 1000 else 100

            stats = benchmark_function(func, data, iterations)
            results[func_name][data_name] = stats

    return results


def compare_with_baseline(rust_results: dict[str, Any]) -> None:
    """Compare Rust results with Python baseline."""
    if not BASELINE_FILE.exists():
        return

    with BASELINE_FILE.open() as f:
        baseline = json.load(f)

    for func_name, func_data in rust_results.items():
        if func_name not in baseline:
            continue

        for data_name in func_data:
            if data_name not in baseline[func_name]:
                continue

            _py_median = baseline[func_name][data_name]["median_ms"]
            _rs_median = func_data[data_name]["median_ms"]
            _speedup = _py_median / _rs_median if _rs_median > 0 else float("inf")

            _py_throughput = baseline[func_name][data_name]["throughput_mb_per_sec"]
            _rs_throughput = func_data[data_name]["throughput_mb_per_sec"]


def main() -> None:
    """Main benchmark execution."""

    results = run_benchmarks()

    output_file = RESULTS_DIR / f"rust_benchmark_{int(time.time())}.json"
    with output_file.open("w") as f:
        json.dump(results, f, indent=2)

    compare_with_baseline(results)


if __name__ == "__main__":
    main()
