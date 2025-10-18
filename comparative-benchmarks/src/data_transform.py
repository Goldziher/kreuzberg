from __future__ import annotations

from typing import TYPE_CHECKING

import msgspec
import polars as pl

from .types import BenchmarkResult, BenchmarkSummary

if TYPE_CHECKING:
    from pathlib import Path


def results_to_dataframe(results: list[BenchmarkResult]) -> pl.DataFrame:
    if not results:
        return pl.DataFrame()

    sanitized: list[dict[str, object]] = []
    for result in results:
        record = msgspec.structs.asdict(result)

        # Drop heavy or nested fields that break DataFrame inference
        record.pop("extracted_metadata", None)
        record.pop("quality_metrics", None)
        record.pop("extracted_text", None)

        # Normalize Enum values so Polars sees consistent string columns
        record["framework"] = record["framework"].value
        record["category"] = record["category"].value
        record["file_type"] = record["file_type"].value
        record["status"] = record["status"].value

        sanitized.append(record)

    schema = {
        "file_path": pl.String,
        "file_size": pl.Int64,
        "file_type": pl.String,
        "category": pl.String,
        "framework": pl.String,
        "iteration": pl.Int64,
        "extraction_time": pl.Float64,
        "startup_time": pl.Float64,
        "peak_memory_mb": pl.Float64,
        "avg_memory_mb": pl.Float64,
        "peak_cpu_percent": pl.Float64,
        "avg_cpu_percent": pl.Float64,
        "total_io_mb": pl.Float64,
        "status": pl.String,
        "character_count": pl.Int64,
        "word_count": pl.Int64,
        "error_type": pl.String,
        "error_message": pl.String,
        "overall_quality_score": pl.Float64,
        "attempts": pl.Int64,
        "timestamp": pl.Float64,
        "platform": pl.String,
        "python_version": pl.String,
    }

    df = pl.DataFrame(sanitized, schema=schema, strict=False)

    if "framework" in df.columns:
        df = df.with_columns(pl.col("framework").cast(pl.String))

    if "file_type" in df.columns:
        df = df.with_columns(pl.col("file_type").cast(pl.String))

    if "category" in df.columns:
        df = df.with_columns(pl.col("category").cast(pl.String))

    if "status" in df.columns:
        df = df.with_columns(pl.col("status").cast(pl.String))

    return df


def summaries_to_dataframe(summaries: list[BenchmarkSummary]) -> pl.DataFrame:
    if not summaries:
        return pl.DataFrame()

    data = [msgspec.structs.asdict(summary) for summary in summaries]

    df = pl.DataFrame(data)

    if "framework" in df.columns:
        df = df.with_columns(pl.col("framework").cast(pl.String))

    if "category" in df.columns:
        df = df.with_columns(pl.col("category").cast(pl.String))

    return df


def load_results_from_json(file_path: Path) -> pl.DataFrame:
    with file_path.open("rb") as f:
        results = msgspec.json.decode(f.read(), type=list[BenchmarkResult])

    return results_to_dataframe(results)


def aggregate_by_framework(df: pl.DataFrame) -> pl.DataFrame:
    return (
        df.group_by("framework")
        .agg(
            pl.col("extraction_time").count().alias("total_files"),
            pl.col("status")
            .filter(pl.col("status") == "success")
            .count()
            .alias("successful_files"),
            pl.col("status")
            .filter(pl.col("status") == "failed")
            .count()
            .alias("failed_files"),
            pl.col("status")
            .filter(pl.col("status") == "timeout")
            .count()
            .alias("timeout_files"),
            pl.col("extraction_time").mean().alias("avg_extraction_time"),
            pl.col("extraction_time").median().alias("median_extraction_time"),
            pl.col("extraction_time").quantile(0.95).alias("p95_extraction_time"),
            pl.col("extraction_time").quantile(0.99).alias("p99_extraction_time"),
            pl.col("extraction_time").std().alias("std_extraction_time"),
            pl.col("extraction_time").min().alias("min_extraction_time"),
            pl.col("extraction_time").max().alias("max_extraction_time"),
            pl.col("peak_memory_mb").mean().alias("avg_peak_memory_mb"),
            pl.col("peak_memory_mb").max().alias("peak_memory_mb"),
            pl.col("avg_cpu_percent").mean().alias("avg_cpu_percent"),
            pl.col("file_size").sum().alias("total_bytes_processed"),
        )
        .with_columns(
            (pl.col("successful_files") / pl.col("total_files")).alias("success_rate"),
            (pl.col("total_files") / pl.col("avg_extraction_time").sum()).alias(
                "files_per_second"
            ),
            (
                pl.col("total_bytes_processed")
                / 1024
                / 1024
                / pl.col("avg_extraction_time").sum()
            ).alias("mb_per_second"),
        )
    )


def aggregate_by_framework_and_format(df: pl.DataFrame) -> pl.DataFrame:
    return (
        df.group_by(["framework", "file_type"])
        .agg(
            pl.col("extraction_time").count().alias("total_files"),
            pl.col("status")
            .filter(pl.col("status") == "SUCCESS")
            .count()
            .alias("successful_files"),
            pl.col("extraction_time").mean().alias("avg_extraction_time"),
            pl.col("extraction_time").median().alias("median_extraction_time"),
            pl.col("peak_memory_mb").mean().alias("avg_peak_memory_mb"),
        )
        .with_columns(
            (pl.col("successful_files") / pl.col("total_files")).alias("success_rate")
        )
    )


def aggregate_by_framework_and_category(df: pl.DataFrame) -> pl.DataFrame:
    return (
        df.group_by(["framework", "category"])
        .agg(
            pl.col("extraction_time").count().alias("total_files"),
            pl.col("status")
            .filter(pl.col("status") == "SUCCESS")
            .count()
            .alias("successful_files"),
            pl.col("extraction_time").mean().alias("avg_extraction_time"),
            pl.col("extraction_time").median().alias("median_extraction_time"),
            pl.col("peak_memory_mb").mean().alias("avg_peak_memory_mb"),
        )
        .with_columns(
            (pl.col("successful_files") / pl.col("total_files")).alias("success_rate")
        )
    )


def calculate_percentiles(
    df: pl.DataFrame, column: str, percentiles: list[float]
) -> dict[str, float | None]:
    result = {}
    for p in percentiles:
        percentile_name = f"p{int(p * 100)}_{column}"
        value = df.get_column(column).quantile(p)
        result[percentile_name] = value

    return result


def filter_successful_results(df: pl.DataFrame) -> pl.DataFrame:
    return df.filter(pl.col("status") == "success")


def add_derived_metrics(df: pl.DataFrame) -> pl.DataFrame:
    return df.with_columns(
        (pl.col("file_size") / 1024 / 1024).alias("file_size_mb"),
        (pl.col("character_count") / pl.col("extraction_time")).alias(
            "chars_per_second"
        ),
        (pl.col("word_count") / pl.col("extraction_time")).alias("words_per_second"),
        (pl.col("peak_memory_mb") / (pl.col("file_size") / 1024 / 1024)).alias(
            "memory_per_mb"
        ),
    )


def export_to_csv(df: pl.DataFrame, output_path: Path) -> Path:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    df.write_csv(str(output_path))
    return output_path


def export_to_json(df: pl.DataFrame, output_path: Path) -> Path:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    df.write_json(str(output_path))
    return output_path


def export_to_parquet(df: pl.DataFrame, output_path: Path) -> Path:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    df.write_parquet(str(output_path))
    return output_path
