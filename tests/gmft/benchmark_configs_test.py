"""Benchmark script to test and tune GMFT default configurations."""

from __future__ import annotations

import json
import time
from pathlib import Path
from typing import Any

from kreuzberg._gmft import extract_tables_sync
from kreuzberg._types import GMFTConfig
from kreuzberg.exceptions import MissingDependencyError


def benchmark_config(pdf_path: Path, config: GMFTConfig, config_name: str) -> dict[str, Any]:
    """Benchmark a specific configuration."""

    start_time = time.time()
    results = {
        "config_name": config_name,
        "detection_threshold": config.detection_threshold,
        "structure_threshold": config.structure_threshold,
        "tables_found": 0,
        "total_cells": 0,
        "avg_confidence": 0.0,
        "extraction_time": 0.0,
        "error": None,
    }

    try:
        tables = extract_tables_sync(pdf_path, config=config)
        extraction_time = time.time() - start_time

        results["tables_found"] = len(tables)
        results["extraction_time"] = extraction_time

        # Analyze extracted tables
        total_cells = 0
        for table in tables:
            df = table["df"]
            if df is not None:
                total_cells += df.shape[0] * df.shape[1]

        results["total_cells"] = total_cells

    except MissingDependencyError:
        results["error"] = "Missing dependencies"
    except Exception as e:
        results["error"] = str(e)

    return results


def main() -> None:
    """Run benchmarks with different configurations."""

    # Test PDFs
    test_pdfs = [
        Path("tests/test_source_files/gmft/tiny.pdf"),
        Path("tests/test_source_files/pdfs_with_tables/medium.pdf"),
    ]

    # Configuration variants to test
    configs = {
        "default": GMFTConfig(),
        "conservative": GMFTConfig(
            detection_threshold=0.8,  # Higher threshold for fewer false positives
            structure_threshold=0.6,
        ),
        "balanced": GMFTConfig(
            detection_threshold=0.7,  # Microsoft's recommended default
            structure_threshold=0.5,
        ),
        "aggressive": GMFTConfig(
            detection_threshold=0.5,  # Lower threshold to catch more tables
            structure_threshold=0.3,
        ),
        "v1.1-pub": GMFTConfig(
            structure_model="microsoft/table-transformer-structure-recognition-v1.1-pub",
            detection_threshold=0.7,
            structure_threshold=0.5,
        ),
        "v1.1-fin": GMFTConfig(
            structure_model="microsoft/table-transformer-structure-recognition-v1.1-fin",
            detection_threshold=0.7,
            structure_threshold=0.5,
        ),
    }

    all_results = []

    for pdf_path in test_pdfs:
        if not pdf_path.exists():
            continue

        for config_name, config in configs.items():
            result = benchmark_config(pdf_path, config, config_name)
            result["pdf"] = pdf_path.name
            all_results.append(result)

    # Summary

    # Group by configuration
    config_stats = {}
    for result in all_results:
        if result["error"]:
            continue

        config_name = result["config_name"]
        if config_name not in config_stats:
            config_stats[config_name] = {"total_tables": 0, "total_cells": 0, "total_time": 0.0, "pdf_count": 0}

        config_stats[config_name]["total_tables"] += result["tables_found"]
        config_stats[config_name]["total_cells"] += result["total_cells"]
        config_stats[config_name]["total_time"] += result["extraction_time"]
        config_stats[config_name]["pdf_count"] += 1

    for stats in config_stats.values():
        if stats["pdf_count"] > 0:
            stats["total_time"] / stats["pdf_count"]

    # Save results
    output_path = Path("tests/gmft/benchmark_results.json")
    output_path.parent.mkdir(exist_ok=True)

    with output_path.open("w") as f:
        json.dump(all_results, f, indent=2)

    # Recommendations

    if config_stats:
        # Find best performing config (most tables found)
        max(config_stats.items(), key=lambda x: x[1]["total_tables"])

        # Find fastest config
        min(config_stats.items(), key=lambda x: x[1]["total_time"] / max(x[1]["pdf_count"], 1))

        # Recommend balanced approach
        if "balanced" in config_stats:
            pass


if __name__ == "__main__":
    main()
