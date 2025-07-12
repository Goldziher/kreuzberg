"""Command-line interface for Kreuzberg benchmarks."""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import typer
from rich.console import Console
from rich.table import Table

from .benchmarks import KreuzbergBenchmarks
from .models import FlameGraphConfig
from .runner import BenchmarkRunner

app = typer.Typer(help="Kreuzberg Performance Benchmarking Suite")
console = Console()


def _generate_quality_report(data: dict[str, Any], console: Console) -> None:
    """Generate metadata quality report from benchmark results."""
    console.print("\n[bold]METADATA QUALITY REPORT[/bold]")
    console.print("=" * 80)

    # Collect results with extraction quality
    quality_results = [
        r
        for r in data["results"]
        if r.get("extraction_quality")
        and r.get("extraction_quality", {}).get("metadata_quality")
    ]

    if not quality_results:
        console.print("[yellow]No extraction quality data available[/yellow]")
        return

    # Group by backend
    backend_stats = {}
    for result in quality_results:
        metadata = result.get("metadata", {})
        backend = metadata.get("backend", "unknown")

        if backend not in backend_stats:
            backend_stats[backend] = {
                "results": [],
                "file_types": set(),
                "total_metadata_fields": 0,
                "unique_fields": set(),
                "avg_completeness": 0,
                "avg_richness": 0,
            }

        quality = result["extraction_quality"]["metadata_quality"]
        backend_stats[backend]["results"].append(quality)
        backend_stats[backend]["file_types"].add(metadata.get("file_type", "unknown"))
        backend_stats[backend]["total_metadata_fields"] += quality["metadata_count"]
        backend_stats[backend]["unique_fields"].update(quality["metadata_fields"])

    # Calculate averages
    for backend, stats in backend_stats.items():
        results = stats["results"]
        if results:
            stats["avg_completeness"] = sum(
                r["metadata_completeness"] for r in results
            ) / len(results)
            stats["avg_richness"] = sum(r["metadata_richness"] for r in results) / len(
                results
            )
            stats["avg_metadata_count"] = stats["total_metadata_fields"] / len(results)

    # Create summary table
    table = Table(title="Backend Metadata Quality Comparison")
    table.add_column("Backend", style="bold")
    table.add_column("Files Tested", justify="right")
    table.add_column("Avg Fields", justify="right")
    table.add_column("Unique Fields", justify="right")
    table.add_column("Completeness %", justify="right")
    table.add_column("Richness Score", justify="right")

    for backend in sorted(backend_stats.keys()):
        stats = backend_stats[backend]
        table.add_row(
            backend,
            str(len(stats["results"])),
            f"{stats['avg_metadata_count']:.1f}",
            str(len(stats["unique_fields"])),
            f"{stats['avg_completeness']:.1f}%",
            f"{stats['avg_richness']:.2f}",
        )

    console.print(table)

    # File type breakdown
    console.print("\n[bold]Metadata Quality by File Type and Backend:[/bold]")

    file_type_stats = {}
    for result in quality_results:
        metadata = result.get("metadata", {})
        file_type = metadata.get("file_type", "unknown")
        backend = metadata.get("backend", "unknown")
        quality = result["extraction_quality"]["metadata_quality"]

        if file_type not in file_type_stats:
            file_type_stats[file_type] = {}

        if backend not in file_type_stats[file_type]:
            file_type_stats[file_type][backend] = []

        file_type_stats[file_type][backend].append(quality)

    for file_type in sorted(file_type_stats.keys()):
        console.print(f"\n{file_type.upper()}:")
        for backend in sorted(file_type_stats[file_type].keys()):
            qualities = file_type_stats[file_type][backend]
            avg_count = sum(q["metadata_count"] for q in qualities) / len(qualities)
            has_title = (
                sum(1 for q in qualities if q["has_title"]) / len(qualities) * 100
            )
            has_author = (
                sum(1 for q in qualities if q["has_author"]) / len(qualities) * 100
            )

            console.print(
                f"  {backend}: {avg_count:.1f} fields, "
                f"{has_title:.0f}% with title, {has_author:.0f}% with author"
            )

    # Top metadata fields
    console.print("\n[bold]Most Common Metadata Fields:[/bold]")
    all_fields = {}
    for backend, stats in backend_stats.items():
        for field in stats["unique_fields"]:
            if field not in all_fields:
                all_fields[field] = set()
            all_fields[field].add(backend)

    # Sort by number of backends that have the field
    sorted_fields = sorted(all_fields.items(), key=lambda x: len(x[1]), reverse=True)[
        :20
    ]

    for field, backends in sorted_fields:
        console.print(f"  {field}: {', '.join(sorted(backends))}")


@app.command()
def run(
    output_dir: Path = typer.Option(
        Path("results"),
        "--output-dir",
        "-o",
        help="Directory to save benchmark results",
    ),  # noqa: B008
    test_files_dir: Path | None = typer.Option(
        None,
        "--test-files-dir",
        "-t",
        help="Directory containing test files for benchmarking",
    ),
    include_flame: bool = typer.Option(
        False, "--flame", "-f", help="Generate flame graphs for performance profiling"
    ),
    suite_name: str = typer.Option(
        "kreuzberg_sync_vs_async",
        "--suite-name",
        "-s",
        help="Name of the benchmark suite",
    ),
    sync_only: bool = typer.Option(
        False, "--sync-only", help="Run only synchronous benchmarks"
    ),
    async_only: bool = typer.Option(
        False, "--async-only", help="Run only asynchronous benchmarks"
    ),
    comparison_only: bool = typer.Option(
        False,
        "--comparison-only",
        help="Run only direct sync vs async comparison benchmarks",
    ),
    include_stress: bool = typer.Option(
        False, "--stress", help="Include stress test benchmarks"
    ),
    backend_comparison: bool = typer.Option(
        False, "--backend-comparison", help="Run backend comparison benchmarks"
    ),
) -> None:
    """Run Kreuzberg performance benchmarks."""
    console.print("[bold blue]Kreuzberg Performance Benchmarks[/bold blue]")
    console.print(f"Suite: {suite_name}")

    benchmarks = KreuzbergBenchmarks(test_files_dir)

    if not benchmarks.test_files:
        console.print("[red]Error:[/red] No test files found for benchmarking")
        console.print(f"Looking in: {benchmarks.test_files_dir}")
        console.print("Please ensure test files exist or specify --test-files-dir")
        raise typer.Exit(1)

    console.print(f"Found {len(benchmarks.test_files)} test files")

    flame_config = FlameGraphConfig(enabled=include_flame) if include_flame else None

    runner = BenchmarkRunner(console=console, flame_config=flame_config)

    sync_benchmarks = []
    async_benchmarks = []

    if backend_comparison:
        # Backend comparison mode
        backend_benchmarks = benchmarks.get_backend_benchmarks()
        sync_benchmarks = backend_benchmarks
        async_benchmarks = []
    elif comparison_only:
        comparison_benchmarks = benchmarks.get_comparison_benchmarks()

        sync_benchmarks = [
            (name, func, meta)
            for name, func, meta in comparison_benchmarks
            if meta.get("type") == "sync"
        ]
        async_benchmarks = [
            (name, func, meta)
            for name, func, meta in comparison_benchmarks
            if meta.get("type") == "async"
        ]
    else:
        if not async_only:
            sync_benchmarks = benchmarks.get_sync_benchmarks()
            if include_stress:
                sync_benchmarks.extend(benchmarks.get_stress_benchmarks())

        if not sync_only:
            async_benchmarks = benchmarks.get_async_benchmarks()
            if include_stress:
                async_benchmarks.extend(benchmarks.get_stress_benchmarks())

    if not sync_benchmarks and not async_benchmarks:
        console.print("[red]Error:[/red] No benchmarks to run")
        raise typer.Exit(1)

    console.print(
        f"Running {len(sync_benchmarks)} sync + {len(async_benchmarks)} async benchmarks"
    )

    try:
        suite = runner.run_benchmark_suite(
            suite_name=suite_name,
            benchmarks=sync_benchmarks,  # type: ignore[arg-type]
            async_benchmarks=async_benchmarks if async_benchmarks else None,  # type: ignore[arg-type]
        )

        runner.print_summary(suite)

        output_dir.mkdir(parents=True, exist_ok=True)
        timestamp = datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%S")
        output_file = output_dir / f"{suite_name}_{timestamp}.json"
        runner.save_results(suite, output_file)

        latest_file = output_dir / "latest.json"
        runner.save_results(suite, latest_file)

        console.print("\n[green]Benchmark completed successfully![/green]")
        console.print(f"Results saved to: {output_file}")

    except KeyboardInterrupt:
        console.print("\n[yellow]Benchmark interrupted by user[/yellow]")
        raise typer.Exit(1)
    except Exception as e:
        console.print(f"\n[red]Benchmark failed:[/red] {e}")
        raise typer.Exit(1)


@app.command()
def compare(
    result1: Path = typer.Argument(..., help="First benchmark result file"),
    result2: Path = typer.Argument(..., help="Second benchmark result file"),
    output: Path | None = typer.Option(
        None, "--output", "-o", help="Save comparison to file"
    ),
) -> None:
    """Compare two benchmark results."""
    console.print("[bold blue]Benchmark Comparison[/bold blue]")

    try:
        with open(result1) as f:
            data1 = json.load(f)
        with open(result2) as f:
            data2 = json.load(f)
    except Exception as e:
        console.print(f"[red]Error loading files:[/red] {e}")
        raise typer.Exit(1)

    console.print("Comparing:")
    console.print(f"  {result1.name}: {data1['name']} ({data1['timestamp']})")
    console.print(f"  {result2.name}: {data2['name']} ({data2['timestamp']})")

    rate1 = data1["summary"]["success_rate_percent"]
    rate2 = data2["summary"]["success_rate_percent"]
    console.print(
        f"\nSuccess Rate: {rate1:.1f}% vs {rate2:.1f}% ({rate2 - rate1:+.1f}%)"
    )

    duration1 = data1["summary"]["total_duration_seconds"]
    duration2 = data2["summary"]["total_duration_seconds"]
    console.print(
        f"Total Duration: {duration1:.3f}s vs {duration2:.3f}s ({duration2 - duration1:+.3f}s)"
    )

    if output:
        comparison_data = {
            "comparison_timestamp": datetime.now(timezone.utc).isoformat(),
            "file1": {"path": str(result1), "data": data1},
            "file2": {"path": str(result2), "data": data2},
            "summary": {
                "success_rate_diff": rate2 - rate1,
                "duration_diff": duration2 - duration1,
            },
        }

        with open(output, "w") as f:
            json.dump(comparison_data, f, indent=2)
        console.print(f"\nComparison saved to: {output}")


@app.command()
def analyze(
    result_file: Path = typer.Argument(..., help="Benchmark result file to analyze"),
    quality_report: bool = typer.Option(
        False, "--quality", "-q", help="Generate metadata quality report"
    ),
) -> None:
    """Analyze benchmark results and generate insights."""
    console.print("[bold blue]Benchmark Analysis[/bold blue]")

    try:
        with open(result_file) as f:
            data = json.load(f)
    except Exception as e:
        console.print(f"[red]Error loading file:[/red] {e}")
        raise typer.Exit(1)

    console.print(f"Analyzing: {data['name']} ({data['timestamp']})")

    successful_results = [
        r for r in data["results"] if r["success"] and r["performance"]
    ]

    if not successful_results:
        console.print("[red]No successful results to analyze[/red]")
        return

    durations = [r["performance"]["duration_seconds"] for r in successful_results]
    memory_peaks = [r["performance"]["memory_peak_mb"] for r in successful_results]

    console.print("\n[bold]Performance Analysis:[/bold]")
    console.print(f"  Successful benchmarks: {len(successful_results)}")
    console.print(f"  Duration range: {min(durations):.3f}s - {max(durations):.3f}s")
    console.print(
        f"  Memory range: {min(memory_peaks):.1f}MB - {max(memory_peaks):.1f}MB"
    )

    sync_results = [r for r in successful_results if "sync" in r["name"]]
    async_results = [r for r in successful_results if "async" in r["name"]]

    if sync_results and async_results:
        sync_avg = sum(
            r["performance"]["duration_seconds"] for r in sync_results
        ) / len(sync_results)
        async_avg = sum(
            r["performance"]["duration_seconds"] for r in async_results
        ) / len(async_results)

        console.print("\n[bold]Sync vs Async:[/bold]")
        console.print(
            f"  Sync average: {sync_avg:.3f}s ({len(sync_results)} benchmarks)"
        )
        console.print(
            f"  Async average: {async_avg:.3f}s ({len(async_results)} benchmarks)"
        )
        console.print(f"  Performance difference: {async_avg - sync_avg:+.3f}s")

    if quality_report:
        _generate_quality_report(data, console)


if __name__ == "__main__":
    app()
