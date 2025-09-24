#!/usr/bin/env python
"""Baseline benchmark for token reduction system.

This benchmark establishes the current performance characteristics
of the Python token reduction implementation, focusing on actual
token reduction performance and statistics.
"""

import gc
import json
import re
import time
from pathlib import Path
from typing import Any

import psutil

from kreuzberg._token_reduction._reducer import reduce_tokens
from kreuzberg._types import TokenReductionConfig


def get_memory_usage() -> float:
    """Get current memory usage in MB."""
    process = psutil.Process()
    return float(process.memory_info().rss / 1024 / 1024)


def count_tokens(text: str) -> int:
    """Count tokens (words) in text."""
    return len(re.findall(r"\S+", text))


def count_unique_tokens(text: str) -> int:
    """Count unique tokens in text."""
    tokens = re.findall(r"\S+", text.lower())
    return len(set(tokens))


def analyze_text(text: str) -> dict[str, Any]:
    """Analyze text characteristics."""
    tokens = re.findall(r"\S+", text)
    chars = len(text)
    lines = text.count("\n") + 1

    # Count different types of content
    punctuation_count = len(re.findall(r'[.,!?;:\'"()\\[\\]{}]', text))
    number_count = len(re.findall(r"\d+", text))

    return {
        "chars": chars,
        "tokens": len(tokens),
        "unique_tokens": len({t.lower() for t in tokens}),
        "lines": lines,
        "punctuation": punctuation_count,
        "numbers": number_count,
        "avg_token_length": sum(len(t) for t in tokens) / len(tokens) if tokens else 0,
    }


def measure_reduction(
    text: str,
    config: TokenReductionConfig,
    iterations: int = 100,
) -> dict[str, Any]:
    """Measure token reduction performance and statistics."""
    # Analyze original text
    original_stats = analyze_text(text)

    # Warmup
    for _ in range(5):
        _ = reduce_tokens(text, config=config)

    # Measure performance
    gc.collect()
    start_memory = get_memory_usage()

    times = []
    for _ in range(iterations):
        start = time.perf_counter()
        result = reduce_tokens(text, config=config)
        end = time.perf_counter()
        times.append((end - start) * 1000)  # Convert to ms

    end_memory = get_memory_usage()
    memory_used = end_memory - start_memory

    # Analyze reduced text
    reduced_stats = analyze_text(result)

    # Calculate statistics
    avg_time = sum(times) / len(times)
    min_time = min(times)
    max_time = max(times)

    # Calculate reductions
    char_reduction = 100 * (1 - reduced_stats["chars"] / original_stats["chars"]) if original_stats["chars"] > 0 else 0
    token_reduction = (
        100 * (1 - reduced_stats["tokens"] / original_stats["tokens"]) if original_stats["tokens"] > 0 else 0
    )
    unique_token_reduction = (
        100 * (1 - reduced_stats["unique_tokens"] / original_stats["unique_tokens"])
        if original_stats["unique_tokens"] > 0
        else 0
    )

    # Tokens per second
    tokens_per_sec = (original_stats["tokens"] / (avg_time / 1000)) if avg_time > 0 else 0

    return {
        "timing": {
            "avg_ms": avg_time,
            "min_ms": min_time,
            "max_ms": max_time,
            "memory_mb": memory_used,
        },
        "original": original_stats,
        "reduced": reduced_stats,
        "reduction": {
            "char_reduction_pct": char_reduction,
            "token_reduction_pct": token_reduction,
            "unique_token_reduction_pct": unique_token_reduction,
            "tokens_removed": original_stats["tokens"] - reduced_stats["tokens"],
            "chars_removed": original_stats["chars"] - reduced_stats["chars"],
        },
        "performance": {
            "tokens_per_sec": tokens_per_sec,
            "mb_per_sec": (len(text) / 1024 / 1024) / (avg_time / 1000) if avg_time > 0 else 0,
            "ms_per_kb": avg_time / (len(text) / 1024) if len(text) > 0 else 0,
        },
    }


def create_test_corpus() -> dict[str, str]:
    """Create comprehensive test corpus."""

    # Technical documentation with lots of stopwords
    technical = (
        """
    The system implementation requires that the components should be integrated
    with the existing infrastructure. This is important because it will ensure
    that all the modules are working properly and that they can communicate
    with each other effectively. The architecture has been designed to be
    scalable and maintainable, which means that it can handle increasing
    loads and that it will be easy to update in the future.

    Furthermore, the system must be able to process the data efficiently
    and return the results quickly. This is achieved through the use of
    optimized algorithms and data structures that have been specifically
    chosen for this purpose. The performance has been tested extensively
    and the results show that it meets all the requirements.
    """
        * 50
    )

    # Natural language with heavy stopword usage
    natural = (
        """
    It was the best of times, it was the worst of times, it was the age
    of wisdom, it was the age of foolishness, it was the epoch of belief,
    it was the epoch of incredulity, it was the season of Light, it was
    the season of Darkness, it was the spring of hope, it was the winter
    of despair, we had everything before us, we had nothing before us,
    we were all going direct to Heaven, we were all going direct the other
    way - in short, the period was so far like the present period, that
    some of its noisiest authorities insisted on its being received, for
    good or for evil, in the superlative degree of comparison only.
    """
        * 40
    )

    # Mixed content with code
    mixed = (
        """
    def process_data(input_data):
        # This function processes the input data and returns the results
        if not input_data:
            return None

        # The processing algorithm is implemented here
        result = []
        for item in input_data:
            # Check if the item is valid and should be processed
            if item and item.is_valid():
                processed = transform(item)
                result.append(processed)

        return result

    The above code demonstrates how the data processing works. It is
    important to note that the function will return None if the input
    is empty or invalid. This is a defensive programming practice that
    helps prevent errors in the system.
    """
        * 60
    )

    # List and bullet points
    lists = (
        """
    The main features of the system include:
    - High performance processing that can handle large volumes of data
    - Scalable architecture that grows with your needs
    - Robust error handling that ensures reliability
    - Comprehensive logging that provides visibility into operations

    In order to use the system effectively, you should:
    1. First, ensure that all prerequisites are installed
    2. Then, configure the system according to your requirements
    3. Next, run the initial setup and verification
    4. Finally, deploy the system to production

    The benefits that you will see include:
    * Improved efficiency in data processing
    * Reduced operational costs
    * Better reliability and uptime
    * Enhanced user experience
    """
        * 50
    )

    # Academic/formal text
    academic = (
        """
    Nevertheless, notwithstanding the aforementioned considerations, it is
    imperative that we acknowledge the fundamental implications of these
    findings. The theoretical framework, which has been extensively
    validated through empirical research, suggests that there exists a
    significant correlation between the observed phenomena and the
    underlying mechanisms that govern the system's behavior.

    Moreover, it should be noted that the methodology employed in this
    investigation adheres to the established protocols and standards that
    have been recognized by the scientific community. The results, therefore,
    can be considered reliable and reproducible, provided that the same
    experimental conditions are maintained.
    """
        * 45
    )

    return {
        "technical": technical,
        "natural": natural,
        "mixed": mixed,
        "lists": lists,
        "academic": academic,
    }


def run_comprehensive_benchmark() -> dict[str, Any]:
    """Run comprehensive benchmark with detailed statistics."""

    corpus = create_test_corpus()
    all_results = {}

    # Test moderate mode (actual reduction)

    for name, text in corpus.items():
        config = TokenReductionConfig(mode="moderate")
        stats = measure_reduction(text, config, iterations=50 if len(text) < 50000 else 20)

        all_results[f"{name}_moderate"] = stats

    # Test light mode

    for name, text in corpus.items():
        config = TokenReductionConfig(mode="light")
        stats = measure_reduction(text, config, iterations=50 if len(text) < 50000 else 20)

        all_results[f"{name}_light"] = stats

    # Test with language-specific stopwords

    languages = ["en", "es", "fr", "de"]
    test_text = corpus["natural"]

    for lang in languages:
        config = TokenReductionConfig(mode="moderate", language_hint=lang)
        stats = measure_reduction(test_text, config, iterations=20)

        all_results[f"natural_{lang}"] = stats

    # Test with markdown preservation

    markdown_text = (
        """
    # Header One
    ## Header Two

    This is **bold** and this is *italic* text.

    - Item one
    - Item two

    `code block` and inline code.

    [Link text](http://example.com)
    """
        * 100
    )

    for preserve in [True, False]:
        config = TokenReductionConfig(mode="moderate", preserve_markdown=preserve)
        stats = measure_reduction(markdown_text, config, iterations=50)

        all_results[f"markdown_preserve_{preserve}"] = stats

    # Summary

    # Average performance by mode
    moderate_stats = [v for k, v in all_results.items() if "moderate" in k and "natural" not in k]
    light_stats = [v for k, v in all_results.items() if "light" in k]

    if moderate_stats:
        sum(s["reduction"]["token_reduction_pct"] for s in moderate_stats) / len(moderate_stats)
        sum(s["timing"]["avg_ms"] for s in moderate_stats) / len(moderate_stats)
        sum(s["performance"]["tokens_per_sec"] for s in moderate_stats) / len(moderate_stats)

    if light_stats:
        sum(s["reduction"]["token_reduction_pct"] for s in light_stats) / len(light_stats)
        sum(s["timing"]["avg_ms"] for s in light_stats) / len(light_stats)
        sum(s["performance"]["tokens_per_sec"] for s in light_stats) / len(light_stats)

    # Best/worst reduction
    sorted_by_reduction = sorted(
        [(k, v["reduction"]["token_reduction_pct"]) for k, v in all_results.items()], key=lambda x: x[1], reverse=True
    )

    for _name, _pct in sorted_by_reduction[:3]:
        pass

    # Save detailed results
    results_path = Path("benchmarks/token_reduction_baseline_results.json")
    with results_path.open("w") as f:
        json.dump(all_results, f, indent=2)

    return all_results


if __name__ == "__main__":
    results = run_comprehensive_benchmark()
