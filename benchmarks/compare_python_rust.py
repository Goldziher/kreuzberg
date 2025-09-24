"""Compare Python vs Rust implementations."""

from __future__ import annotations

import time
from typing import Any

# Rust implementations
from kreuzberg._rust_bridge import (
    calculate_quality_score,
    clean_extracted_text,
    normalize_spaces,
)

# Python implementations
from kreuzberg._utils._quality import (
    calculate_quality_score as calculate_quality_score_py,
)
from kreuzberg._utils._quality import (
    clean_extracted_text as clean_extracted_text_py,
)
from kreuzberg._utils._quality import (
    normalize_spaces as normalize_spaces_py,
)


def benchmark_function(func: Any, data: Any, iterations: int = 1000) -> float:
    """Benchmark a single function and return average time in ms."""
    start = time.perf_counter()
    for _ in range(iterations):
        func(data) if not isinstance(data, dict) else func(**data)
    end = time.perf_counter()
    return ((end - start) / iterations) * 1000  # Convert to ms


def main() -> None:
    # Test data
    test_texts = {
        "small": "This is a small test text with some words.",
        "medium": " ".join(["This is a medium length text."] * 100),
        "large": " ".join(["This is a large text document with many words."] * 1000),
        "ocr_artifacts": "T h i s   h a s   s c a t t e r e d   c h a r s... ... ...",
    }

    for text in test_texts.values():
        # Calculate quality score
        py_time = benchmark_function(calculate_quality_score_py, text)
        rs_time = benchmark_function(calculate_quality_score, text)
        py_time / rs_time if rs_time > 0 else float("inf")

    for text in test_texts.values():
        # Clean extracted text
        py_time = benchmark_function(clean_extracted_text_py, text)
        rs_time = benchmark_function(clean_extracted_text, text)
        py_time / rs_time if rs_time > 0 else float("inf")

    for text in test_texts.values():
        # Normalize spaces
        py_time = benchmark_function(normalize_spaces_py, text)
        rs_time = benchmark_function(normalize_spaces, text)
        py_time / rs_time if rs_time > 0 else float("inf")


if __name__ == "__main__":
    main()
