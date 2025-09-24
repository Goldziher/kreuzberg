"""Benchmark suite for quality utils (text processing) operations.

This benchmark establishes baselines for Python implementation before Rust migration.
Measures performance of:
- calculate_quality_score
- clean_extracted_text
- normalize_spaces
- safe_decode
"""

from __future__ import annotations

import gc
import json
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import TYPE_CHECKING, Any

import click
import numpy as np
from rich.console import Console
from rich.progress import BarColumn, Progress, SpinnerColumn, TextColumn, TimeElapsedColumn
from rich.table import Table

from kreuzberg._utils._quality import calculate_quality_score, clean_extracted_text
from kreuzberg._utils._string import normalize_spaces, safe_decode
from kreuzberg.extraction import extract_file_sync

if TYPE_CHECKING:
    from collections.abc import Callable

console = Console()


# Load real test data from existing files
def load_test_data() -> dict[str, Any]:
    """Load test data from actual test files."""
    test_data = {}

    # Small clean text
    test_data["small_clean"] = {
        "size": "1KB",
        "text": "This is a simple test document with clear structure. " * 30,
        "description": "Small clean text",
    }

    # Small text with OCR artifacts
    test_data["small_ocr_artifacts"] = {
        "size": "2KB",
        "text": "T h i s   i s   s c a t t e r e d   t e x t... " * 50 + "test123abc456def789" * 10,
        "description": "Small text with OCR artifacts",
    }

    # Extract text from small PDF
    try:
        small_pdf = extract_file_sync("tests/test_source_files/searchable.pdf")
        test_data["pdf_searchable"] = {
            "size": "18KB",
            "text": small_pdf.content,
            "description": "Extracted text from searchable PDF",
        }
    except Exception as e:
        console.print(f"[yellow]Warning: Could not extract searchable.pdf: {e}[/yellow]")

    # Extract text from test article PDF
    try:
        article = extract_file_sync("tests/test_source_files/test-article.pdf")
        test_data["pdf_article"] = {
            "size": "278KB",
            "text": article.content,
            "description": "Extracted text from test article PDF",
        }
    except Exception as e:
        console.print(f"[yellow]Warning: Could not extract test-article.pdf: {e}[/yellow]")

    # Extract text from large PDF
    try:
        large_pdf = extract_file_sync("tests/test_source_files/sharable-web-guide.pdf")
        test_data["pdf_large"] = {
            "size": "3.8MB",
            "text": large_pdf.content[:500000],  # Limit to 500KB of text
            "description": "Extracted text from large PDF (truncated)",
        }
    except Exception as e:
        console.print(f"[yellow]Warning: Could not extract sharable-web-guide.pdf: {e}[/yellow]")

    # Medium mixed content with various issues
    test_data["medium_mixed"] = {
        "size": "50KB",
        "text": (
            "This is a normal paragraph with good structure.\n\n" * 100
            + "function test() { return true; }" * 50
            + "<script>alert('test')</script>" * 30
            + "Skip to main content > Home > Page 1 of 10" * 20
            + "T h i s   h a s   s c a t t e r e d   c h a r s" * 40
        ),
        "description": "Medium mixed content (normal + code + navigation + OCR)",
    }

    # Large messy text
    test_data["large_messy"] = {
        "size": "500KB",
        "text": (
            "Normal text here. " * 2000
            + "   \t\t\t   " * 5000  # Excessive whitespace
            + "..." * 2000  # Repeated punctuation
            + "test123abc456" * 1000  # Malformed words
            + "\n\n\n\n\n" * 1000  # Multiple newlines
            + "א ב ג ד ה ו ז ח" * 500  # Hebrew characters  # noqa: RUF001
            + "\u0400\u0401\u0402" * 500  # Cyrillic that might be mojibake
        ),
        "description": "Large messy text with various issues",
    }

    return test_data


# Load test data
TEST_DATA = load_test_data()

# Add some binary data for encoding tests
ENCODING_TEST_DATA = {
    "utf8": b"Hello World",
    "hebrew": "שלום עולם".encode("windows-1255"),
    "arabic": "مرحبا بالعالم".encode("windows-1256"),
    "cyrillic": "Привет мир".encode("cp1251"),
    "mixed_invalid": b"\xff\xfe" + b"Test" + b"\x00\x01\x02",
}


def benchmark_function(func: Callable[..., Any], *args: Any, iterations: int = 100, **kwargs: Any) -> dict[str, Any]:
    """Benchmark a single function with multiple iterations."""
    times = []

    # Warmup
    for _ in range(min(10, iterations // 10)):
        func(*args, **kwargs)

    gc.collect()

    for _ in range(iterations):
        start = time.perf_counter()
        func(*args, **kwargs)
        end = time.perf_counter()
        times.append(end - start)

    times_ms = [t * 1000 for t in times]

    # Calculate throughput based on text size if available
    text_size_bytes = len(args[0]) if args else 0
    text_size_mb = text_size_bytes / (1024 * 1024)

    return {
        "mean_ms": np.mean(times_ms),
        "median_ms": np.median(times_ms),
        "std_ms": np.std(times_ms),
        "min_ms": np.min(times_ms),
        "max_ms": np.max(times_ms),
        "p95_ms": np.percentile(times_ms, 95),
        "p99_ms": np.percentile(times_ms, 99),
        "iterations": iterations,
        "total_time_s": sum(times),
        "ops_per_sec": iterations / sum(times) if sum(times) > 0 else 0,
        "mb_per_sec": text_size_mb / (np.mean(times_ms) / 1000) if text_size_mb > 0 else 0,
        "text_size_bytes": text_size_bytes,
    }


def run_quality_benchmarks(progress: Progress) -> dict[str, Any]:
    """Run benchmarks for quality utils functions."""
    results: dict[str, Any] = {}

    # Benchmark calculate_quality_score
    task = progress.add_task("[cyan]Benchmarking calculate_quality_score...", total=len(TEST_DATA))
    results["calculate_quality_score"] = {}

    for name, data in TEST_DATA.items():
        metadata = {"title": "Test Doc", "author": "Test Author"}
        stats = benchmark_function(
            calculate_quality_score, data["text"], metadata=metadata, iterations=100 if "huge" not in name else 10
        )
        results["calculate_quality_score"][name] = {
            **stats,
            "data_size": data["size"],
            "description": data["description"],
        }
        progress.update(task, advance=1)

    # Benchmark clean_extracted_text
    task = progress.add_task("[cyan]Benchmarking clean_extracted_text...", total=len(TEST_DATA))
    results["clean_extracted_text"] = {}

    for name, data in TEST_DATA.items():
        stats = benchmark_function(clean_extracted_text, data["text"], iterations=100 if "huge" not in name else 10)
        results["clean_extracted_text"][name] = {**stats, "data_size": data["size"], "description": data["description"]}
        progress.update(task, advance=1)

    # Benchmark normalize_spaces
    task = progress.add_task("[cyan]Benchmarking normalize_spaces...", total=len(TEST_DATA))
    results["normalize_spaces"] = {}

    for name, data in TEST_DATA.items():
        stats = benchmark_function(normalize_spaces, data["text"], iterations=100 if "huge" not in name else 10)
        results["normalize_spaces"][name] = {**stats, "data_size": data["size"], "description": data["description"]}
        progress.update(task, advance=1)

    # Benchmark safe_decode
    task = progress.add_task("[cyan]Benchmarking safe_decode...", total=len(ENCODING_TEST_DATA))
    results["safe_decode"] = {}

    for name, data in ENCODING_TEST_DATA.items():
        stats = benchmark_function(safe_decode, data, iterations=1000)
        results["safe_decode"][name] = {
            **stats,
            "data_size": f"{len(data)} bytes",
            "description": f"Decoding {name} encoded data",
        }
        progress.update(task, advance=1)

    return results


def display_results(results: dict[str, Any]) -> None:
    """Display benchmark results in a nice table format."""
    for func_name, func_results in results.items():
        console.print(f"\n[bold cyan]═══ {func_name} ═══[/bold cyan]")

        table = Table(show_header=True, header_style="bold magenta")
        table.add_column("Test Case", style="cyan")
        table.add_column("Size", justify="right")
        table.add_column("Mean (ms)", justify="right", style="green")
        table.add_column("Median (ms)", justify="right")
        table.add_column("P95 (ms)", justify="right")
        table.add_column("Ops/sec", justify="right", style="yellow")
        table.add_column("MB/sec", justify="right", style="blue")

        for test_name, stats in func_results.items():
            mb_per_sec = stats.get("mb_per_sec", 0)
            table.add_row(
                test_name,
                stats["data_size"],
                f"{stats['mean_ms']:.3f}",
                f"{stats['median_ms']:.3f}",
                f"{stats['p95_ms']:.3f}",
                f"{stats['ops_per_sec']:.1f}",
                f"{mb_per_sec:.1f}" if mb_per_sec > 0 else "-",
            )

        console.print(table)


def save_results(results: dict[str, Any], output_path: Path) -> None:
    """Save benchmark results to JSON file."""
    output = {
        "timestamp": datetime.now(tz=timezone.utc).isoformat(),
        "type": "quality_utils_benchmark",
        "implementation": "python",
        "results": results,
        "system_info": {
            "python_version": __import__("sys").version,
            "platform": __import__("platform").platform(),
            "cpu_count": __import__("os").cpu_count(),
        },
    }

    output_path.parent.mkdir(parents=True, exist_ok=True)
    with output_path.open("w") as f:
        json.dump(output, f, indent=2)

    console.print(f"\n[green]✓[/green] Results saved to {output_path}")


@click.command()
@click.option(
    "--output",
    "-o",
    type=click.Path(path_type=Path),
    default="benchmarks/results/quality_utils_baseline.json",
    help="Output file for benchmark results",
)
@click.option("--iterations", "-i", type=int, default=100, help="Number of iterations per benchmark")
@click.option(
    "--compare", "-c", type=click.Path(exists=True, path_type=Path), help="Compare with previous benchmark results"
)
def main(output: Path, iterations: int, compare: Path | None) -> None:  # noqa: ARG001
    """Run quality utils benchmarks and save results."""
    console.print("[bold cyan]Quality Utils Benchmark Suite[/bold cyan]")
    console.print("=" * 50)

    with Progress(
        SpinnerColumn(),
        TextColumn("[progress.description]{task.description}"),
        BarColumn(),
        TimeElapsedColumn(),
        console=console,
    ) as progress:
        results = run_quality_benchmarks(progress)

    display_results(results)
    save_results(results, output)

    if compare:
        console.print(f"\n[yellow]Comparing with {compare}...[/yellow]")
        with compare.open() as f:
            previous = json.load(f)

        if previous.get("type") == "quality_utils_benchmark":
            console.print("\n[bold]Performance Comparison[/bold]")
            table = Table(show_header=True, header_style="bold magenta")
            table.add_column("Function", style="cyan")
            table.add_column("Test Case")
            table.add_column("Previous (ms)", justify="right")
            table.add_column("Current (ms)", justify="right")
            table.add_column("Change", justify="right")

            for func_name in results:
                if func_name in previous.get("results", {}):
                    for test_name in results[func_name]:
                        if test_name in previous["results"][func_name]:
                            prev_mean = previous["results"][func_name][test_name]["mean_ms"]
                            curr_mean = results[func_name][test_name]["mean_ms"]
                            change = ((curr_mean - prev_mean) / prev_mean) * 100

                            change_str = f"{change:+.1f}%"
                            if change < -10:
                                change_str = f"[green]{change_str}[/green]"
                            elif change > 10:
                                change_str = f"[red]{change_str}[/red]"

                            table.add_row(func_name, test_name, f"{prev_mean:.3f}", f"{curr_mean:.3f}", change_str)

            console.print(table)


if __name__ == "__main__":
    main()
