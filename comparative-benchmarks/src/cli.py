from __future__ import annotations

import asyncio
import sys
from functools import cache
from pathlib import Path

import click
from rich.console import Console

from .aggregate import ResultAggregator
from .benchmark import BenchmarkRunner
from .data_transform import (
    aggregate_by_framework,
    aggregate_by_framework_and_format,
    export_to_csv,
    export_to_json,
    export_to_parquet,
    load_results_from_json,
)
from .docs_generator import (
    DocConfig,
    generate_detailed_results_page,
    generate_framework_comparison_page,
    generate_index_page,
    generate_methodology_page,
)
from .logger import get_logger
from .types import BenchmarkConfig, DocumentCategory, Framework
from .visualization import (
    create_interactive_dashboard,
    create_memory_usage_chart,
    create_per_format_heatmap,
    create_performance_comparison_chart,
    create_throughput_chart,
    create_time_distribution_chart,
)

logger = get_logger(__name__)


@cache
def get_console() -> Console:
    return Console()


@click.group()
def cli() -> None:
    pass


@cli.command()
@click.option(
    "--iterations",
    "-i",
    type=int,
    default=5,
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
def benchmark(
    iterations: int, timeout: int, framework: str | None, output: Path
) -> None:
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

    if framework:
        output_dir = output.parent
    else:
        output_dir = Path("results")
    output_dir.mkdir(parents=True, exist_ok=True)

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

    runner = BenchmarkRunner(config)
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
    except Exception as e:  # noqa: BLE001
        console.print(f"[red]Benchmark failed: {e}[/red]")
        logger.error("Benchmark failed", error=str(e))
        sys.exit(1)


@cli.command()
@click.option(
    "--input",
    "-i",
    "input_file",
    type=click.Path(exists=True, path_type=Path),
    required=True,
    help="Input JSON file with benchmark results",
)
@click.option(
    "--output-dir",
    "-o",
    type=click.Path(path_type=Path),
    default=Path("charts"),
    help="Output directory for charts",
)
def visualize(input_file: Path, output_dir: Path) -> None:
    console = get_console()
    console.print("[bold blue]Generating Visualizations[/bold blue]")
    console.print(f"Input: {input_file}")
    console.print(f"Output: {output_dir}\n")

    output_dir.mkdir(parents=True, exist_ok=True)

    console.print("📊 Loading benchmark results...")
    df = load_results_from_json(input_file)

    console.print("📈 Creating performance comparison chart...")
    summary_df = aggregate_by_framework(df)
    create_performance_comparison_chart(
        summary_df, output_dir / "performance_comparison.html"
    )

    console.print("💾 Creating memory usage chart...")
    create_memory_usage_chart(summary_df, output_dir / "memory_usage.html")

    console.print("⚡ Creating throughput chart...")
    create_throughput_chart(summary_df, output_dir / "throughput.html")

    console.print("📦 Creating time distribution chart...")
    create_time_distribution_chart(df, output_dir / "time_distribution.html")

    console.print("🎯 Creating interactive dashboard...")
    create_interactive_dashboard(summary_df, output_dir / "dashboard.html")

    console.print("🔥 Creating per-format heatmap...")
    format_df = aggregate_by_framework_and_format(df)
    create_per_format_heatmap(format_df, output_dir / "format_heatmap.html")

    console.print(f"\n[green]✨ Generated 6 visualizations in {output_dir}[/green]")


@cli.command()
@click.option(
    "--input",
    "-i",
    "input_file",
    type=click.Path(exists=True, path_type=Path),
    required=True,
    help="Input JSON file with benchmark results",
)
@click.option(
    "--output-dir",
    "-o",
    type=click.Path(path_type=Path),
    default=Path("docs/benchmarks"),
    help="Output directory for documentation",
)
@click.option(
    "--charts-dir",
    "-c",
    type=click.Path(path_type=Path),
    default=None,
    help="Directory containing generated charts",
)
def generate_docs(input_file: Path, output_dir: Path, charts_dir: Path | None) -> None:
    console = get_console()
    console.print("[bold blue]Generating Documentation[/bold blue]")
    console.print(f"Input: {input_file}")
    console.print(f"Output: {output_dir}\n")

    output_dir.mkdir(parents=True, exist_ok=True)
    (output_dir / "data").mkdir(exist_ok=True)
    (output_dir / "charts").mkdir(exist_ok=True)

    console.print("📊 Loading benchmark results...")
    df = load_results_from_json(input_file)

    console.print("📈 Aggregating data...")
    summary_df = aggregate_by_framework(df)
    format_df = aggregate_by_framework_and_format(df)

    console.print("💾 Exporting data files...")
    export_to_csv(summary_df, output_dir / "data" / "latest.csv")
    export_to_json(summary_df, output_dir / "data" / "latest.json")
    export_to_parquet(summary_df, output_dir / "data" / "latest.parquet")

    config: DocConfig = {"charts_dir": charts_dir} if charts_dir else {}

    console.print("📝 Generating index page...")
    generate_index_page(summary_df, output_dir / "index.md", config)

    console.print("📋 Generating detailed results...")
    generate_detailed_results_page(
        summary_df, format_df, output_dir / "latest-results.md", config
    )

    console.print("⚖️  Generating framework comparison...")
    generate_framework_comparison_page(
        summary_df, output_dir / "framework-comparison.md", config
    )

    console.print("🔬 Generating methodology...")
    generate_methodology_page(output_dir / "methodology.md")

    console.print(f"\n[green]✨ Generated documentation in {output_dir}[/green]")


def main() -> None:
    cli()


if __name__ == "__main__":
    main()
