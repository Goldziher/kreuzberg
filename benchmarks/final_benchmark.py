"""Final comprehensive benchmark comparing all improvements."""

import asyncio
import json
import time
from pathlib import Path

from kreuzberg import ExtractionConfig, extract_file
from kreuzberg._utils._cache import (
    clear_all_caches,
    get_document_cache,
    get_mime_cache,
    get_ocr_cache,
    get_table_cache,
)


async def run_final_benchmark() -> dict[str, object] | None:
    """Run comprehensive benchmark of all caching improvements."""
    test_files_dir = Path("tests/test_source_files")
    pdf_files = list(test_files_dir.glob("*.pdf"))

    if not pdf_files:
        return None

    single_file = pdf_files[0]

    full_config = ExtractionConfig(
        force_ocr=True, ocr_backend="tesseract", extract_tables=True, chunk_content=True
    )

    clear_all_caches()

    start_time = time.time()
    try:
        result_baseline = await extract_file(single_file, config=full_config)
        baseline_duration = time.time() - start_time

        baseline_success = True
    except Exception:
        baseline_duration = time.time() - start_time
        baseline_success = False
        result_baseline = None

    start_time = time.time()
    try:
        result_cached = await extract_file(single_file, config=full_config)
        cached_duration = time.time() - start_time

        total_speedup = (
            baseline_duration / cached_duration if cached_duration > 0 else float("inf")
        )
        content_match = (
            result_baseline.content == result_cached.content
            if result_baseline and result_cached
            else False
        )

        cached_success = True
    except Exception:
        cached_duration = time.time() - start_time
        cached_success = False
        total_speedup = 1
        content_match = False

    run_times = []
    for _i in range(10):
        start_time = time.time()
        try:
            await extract_file(single_file, config=full_config)
            duration = time.time() - start_time
            run_times.append(duration)
        except Exception:
            run_times.append(float("inf"))

    valid_times = [t for t in run_times if t != float("inf")]
    if valid_times:
        avg_time = sum(valid_times) / len(valid_times)
        min(valid_times)
        max(valid_times)
        (sum((t - avg_time) ** 2 for t in valid_times) / len(valid_times)) ** 0.5

    caches = {
        "MIME": get_mime_cache(),
        "OCR": get_ocr_cache(),
        "Tables": get_table_cache(),
        "Documents": get_document_cache(),
    }

    total_size = 0
    total_items = 0

    for cache in caches.values():
        stats = cache.get_stats()  # type: ignore[attr-defined]
        total_size += stats["total_cache_size_mb"]
        total_items += stats["cached_results"]

        stats["cached_results"] / max(stats["total_cache_size_mb"], 0.001)

    if baseline_success and cached_success:
        baseline_duration - cached_duration

        components = {
            "MIME Detection": 0.001,
            "OCR Processing": baseline_duration * 0.7,
            "Table Extraction": baseline_duration * 0.25,
            "Document Processing": baseline_duration * 0.05,
        }

        for time_est in components.values():
            (time_est / baseline_duration) * 100

    cache_root = Path(".kreuzberg")
    if cache_root.exists():
        len(list(cache_root.rglob("*.msgpack")))
        sum(f.stat().st_size for f in cache_root.rglob("*") if f.is_file())

    return {
        "baseline_duration": baseline_duration,
        "cached_duration": cached_duration,
        "total_speedup": total_speedup,
        "avg_cached_time": avg_time if valid_times else None,
        "cache_size_mb": total_size,
        "cache_items": total_items,
        "reliability_rate": len(valid_times) / 10,
        "content_accuracy": content_match,
        "baseline_success": baseline_success,
        "cached_success": cached_success,
    }


if __name__ == "__main__":
    try:
        results = asyncio.run(run_final_benchmark())

        if results is not None:
            final_results_file = Path("final_benchmark_results.json")
            with final_results_file.open("w") as f:
                json.dump(results, f, indent=2, default=str)

            if results["baseline_success"] and results["cached_success"]:
                pass

    except Exception:
        import traceback

        traceback.print_exc()
