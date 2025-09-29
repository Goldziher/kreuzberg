from __future__ import annotations

import asyncio
import sys
from functools import cache
from pathlib import Path

import click
from rich.console import Console

from .aggregate import ResultAggregator
from .benchmark import ComprehensiveBenchmarkRunner
from .logger import get_logger
from .types import BenchmarkConfig, DocumentCategory, Framework

logger = get_logger(__name__)


@cache
def get_console() -> Console:
    """Get or create the console instance lazily."""
    return Console()


@click.command()
@click.option(
    "--iterations",
    "-i",
    type=int,
    default=3,
    help="Number of benchmark iterations",
)
@click.option(
    "--timeout",
    "-t",
    type=int,
    default=300,
    help="Timeout in seconds for each extraction",
)
@click.option(
    "--framework",
    "-f",
    type=click.Choice([f.value for f in Framework]),
    help="Specific framework to benchmark (if not specified, runs all frameworks)",
)
@click.option(
    "--output",
    "-o",
    type=click.Path(dir_okay=False, path_type=Path),
    default="results/aggregated.json",
    help="Output file for aggregated results",
)
def main(iterations: int, timeout: int, framework: str | None, output: Path) -> None:
    """Run benchmarks for all frameworks."""
    console = get_console()
    console.print("[bold]Starting Benchmark Suite[/bold]")
    console.print(f"  Iterations: {iterations}")
    console.print(f"  Timeout: {timeout}s")
    if framework:
        console.print(f"  Framework: {framework}")
    else:
        console.print("  Frameworks: all")
    console.print(f"  Output: {output}")
    console.print()

    frameworks = [Framework(framework)] if framework else list(Framework)

    categories = list(DocumentCategory)

    output_dir = Path("results")
    output_dir.mkdir(exist_ok=True)

    config = BenchmarkConfig(
        frameworks=frameworks,
        categories=categories,
        file_types=None,
        iterations=iterations,
        warmup_runs=1,
        timeout_seconds=timeout,
        output_dir=output_dir,
        continue_on_error=True,
        max_run_duration_minutes=360,
        save_extracted_text=False,
        enable_quality_assessment=False,
    )

    console.print("[cyan]Running benchmarks...[/cyan]")

    runner = ComprehensiveBenchmarkRunner(config)
    runner.use_subprocess_isolation = True

    try:
        results = asyncio.run(runner.run_benchmark_suite())
        console.print(f"[green]✓ Completed {len(results)} benchmarks[/green]")

        console.print("[cyan]Aggregating results...[/cyan]")
        aggregator = ResultAggregator()
        aggregated = aggregator.aggregate_results([output_dir])

        output.parent.mkdir(parents=True, exist_ok=True)
        aggregator.save_results(aggregated, output.parent, output.name)
        console.print(f"[green]✓ Results saved to {output}[/green]")

    except KeyboardInterrupt:
        console.print("[yellow]Benchmark interrupted by user[/yellow]")
        sys.exit(1)
    except Exception as e:
        console.print(f"[red]Benchmark failed: {e}[/red]")
        logger.error("Benchmark failed", error=str(e))
        sys.exit(1)


if __name__ == "__main__":
    main()
