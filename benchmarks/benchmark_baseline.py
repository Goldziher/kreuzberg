"""Baseline performance benchmark before implementing multi-layer caching."""

import asyncio
import time
from pathlib import Path

from kreuzberg import ExtractionConfig, batch_extract_file, extract_file_sync
from kreuzberg._utils._document_cache import clear_document_cache, get_document_cache


async def run_baseline_benchmark() -> dict[str, object] | None:
    """Run comprehensive baseline benchmark."""
    test_files_dir = Path("tests/test_source_files")
    test_files = list(test_files_dir.glob("*.pdf"))

    if not test_files:
        return None

    single_file = test_files[0]
    mixed_files = test_files[:3] if len(test_files) >= 3 else [single_file] * 3

    results = {}

    clear_document_cache()

    start_time = time.time()
    result = extract_file_sync(single_file)
    cold_duration = time.time() - start_time

    results["single_file_cold"] = {
        "duration": cold_duration,
        "content_length": len(result.content),
        "success": not result.metadata.get("error"),
    }

    start_time = time.time()
    extract_file_sync(single_file)
    cached_duration = time.time() - start_time

    speedup = cold_duration / cached_duration if cached_duration > 0 else float("inf")

    results["single_file_cached"] = {
        "duration": cached_duration,
        "speedup": speedup,
        "cache_hit": cached_duration < 0.1,
    }

    clear_document_cache()

    same_files = [single_file] * 10
    start_time = time.time()
    same_results = await batch_extract_file(same_files)
    same_duration = time.time() - start_time

    same_successes = sum(1 for r in same_results if not r.metadata.get("error"))
    same_failure_rate = ((len(same_results) - same_successes) / len(same_results)) * 100

    results["same_file_batch"] = {
        "duration": same_duration,
        "avg_per_file": same_duration / len(same_files),
        "success_rate": 100 - same_failure_rate,
        "throughput": len(same_files) / same_duration,
    }

    clear_document_cache()

    start_time = time.time()
    mixed_results = await batch_extract_file(mixed_files)
    mixed_duration = time.time() - start_time

    mixed_successes = sum(1 for r in mixed_results if not r.metadata.get("error"))
    mixed_failure_rate = (
        (len(mixed_results) - mixed_successes) / len(mixed_results)
    ) * 100

    results["mixed_files_batch"] = {
        "duration": mixed_duration,
        "avg_per_file": mixed_duration / len(mixed_files),
        "success_rate": 100 - mixed_failure_rate,
        "throughput": len(mixed_files) / mixed_duration,
    }

    clear_document_cache()

    ocr_config = ExtractionConfig(force_ocr=True, ocr_backend="tesseract")
    ocr_files = [single_file] * 3

    start_time = time.time()
    ocr_results = await batch_extract_file(ocr_files, ocr_config)
    ocr_duration = time.time() - start_time

    ocr_successes = sum(1 for r in ocr_results if not r.metadata.get("error"))
    ocr_failure_rate = ((len(ocr_results) - ocr_successes) / len(ocr_results)) * 100

    results["ocr_workload"] = {
        "duration": ocr_duration,
        "avg_per_file": ocr_duration / len(ocr_files),
        "success_rate": 100 - ocr_failure_rate,
        "throughput": len(ocr_files) / ocr_duration,
    }

    cache = get_document_cache()
    cache_stats = cache.get_stats()

    results["cache_stats"] = cache_stats

    return results  # type: ignore[return-value]


if __name__ == "__main__":
    baseline_results = asyncio.run(run_baseline_benchmark())

    import json

    baseline_file = Path("baseline_results.json")
    with baseline_file.open("w") as f:
        json.dump(baseline_results, f, indent=2, default=str)
