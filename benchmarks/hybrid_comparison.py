"""Performance comparison between Kreuzberg, Extractous, and Hybrid backends."""
# type: ignore

from __future__ import annotations

import time
from pathlib import Path
from typing import Any

try:
    from kreuzberg import extract_file_sync
    from kreuzberg._backends import ExtractionStrategy, HybridBackend

    HAS_KREUZBERG = True
except ImportError:
    HAS_KREUZBERG = False

# Test files for different scenarios (using relative paths from current directory)
TEST_FILES = {
    "xlsx_small": "../tests/test_source_files/excel.xlsx",
    "xlsx_large": "../tests/test_source_files/excel-multi-sheet.xlsx",
    "json_small": "../tests/test_source_files/json/books.json",
    "json_large": "../tests/test_source_files/json/large-dataset.json",
    "eml_simple": "../tests/test_source_files/email/sample-email.eml",
    "eml_complex": "../tests/test_source_files/email/large-newsletter.eml",
    "yaml_config": "../tests/test_source_files/yaml/sample-config.yaml",
    "pdf_contract": "../tests/test_source_files/sample-contract.pdf",
    "docx_document": "../tests/test_source_files/document.docx",
}


def benchmark_backend(
    backend_name: str, test_files: dict[str, str], iterations: int = 3
) -> dict[str, Any]:
    """Benchmark a specific backend.

    Args:
        backend_name: Name of the backend to test
        test_files: Dictionary of test file descriptions to paths
        iterations: Number of iterations per file

    Returns:
        Dictionary containing benchmark results
    """
    results = {
        "backend": backend_name,
        "iterations": iterations,
        "files": {},
        "summary": {},
        "errors": [],
    }

    total_time = 0
    total_files = 0
    successful_extractions = 0

    for file_desc, file_path in test_files.items():
        if not Path(file_path).exists():
            results["errors"].append(f"File not found: {file_path}")
            continue

        file_results = {
            "path": file_path,
            "times": [],
            "success": False,
            "avg_time": 0,
            "throughput": 0,
            "text_length": 0,
            "backend_used": None,
            "error": None,
        }

        file_size = Path(file_path).stat().st_size / (1024 * 1024)  # MB

        for i in range(iterations):
            try:
                start_time = time.perf_counter()

                if backend_name == "kreuzberg":
                    result = extract_file_sync(file_path)
                elif backend_name == "hybrid_speed":
                    backend = HybridBackend(strategy=ExtractionStrategy.SPEED)
                    result = backend.extract(file_path)
                elif backend_name == "hybrid_rich_metadata":
                    backend = HybridBackend(strategy=ExtractionStrategy.RICH_METADATA)
                    result = backend.extract(file_path)
                elif backend_name == "hybrid_balanced":
                    backend = HybridBackend(strategy=ExtractionStrategy.BALANCED)
                    result = backend.extract(file_path)
                else:
                    raise ValueError(f"Unknown backend: {backend_name}")

                end_time = time.perf_counter()
                extraction_time = (end_time - start_time) * 1000  # ms

                file_results["times"].append(extraction_time)
                file_results["success"] = True
                file_results["text_length"] = len(result.content)
                file_results["backend_used"] = result.metadata.get(
                    "primary_backend", backend_name
                )

                total_time += extraction_time
                successful_extractions += 1

            except Exception as e:
                file_results["error"] = str(e)
                results["errors"].append(f"{file_desc}: {e}")

        if file_results["times"]:
            file_results["avg_time"] = sum(file_results["times"]) / len(
                file_results["times"]
            )
            file_results["throughput"] = (
                file_size / (file_results["avg_time"] / 1000)
                if file_results["avg_time"] > 0
                else 0
            )

        results["files"][file_desc] = file_results
        total_files += iterations

    # Calculate summary statistics
    results["summary"] = {
        "total_files": total_files,
        "successful_extractions": successful_extractions,
        "success_rate": (successful_extractions / total_files * 100)
        if total_files > 0
        else 0,
        "avg_extraction_time": total_time / successful_extractions
        if successful_extractions > 0
        else 0,
        "total_time": total_time,
    }

    return results


def run_comprehensive_benchmark() -> dict[str, Any]:
    """Run comprehensive benchmark comparing all backends."""
    if not HAS_KREUZBERG:
        print("Kreuzberg not available for benchmarking")
        return {}

    print("🚀 Starting Hybrid Backend Performance Benchmark")
    print("=" * 60)

    # Filter test files to only existing ones
    existing_files = {k: v for k, v in TEST_FILES.items() if Path(v).exists()}
    print(f"Testing with {len(existing_files)} files...")

    backends_to_test = [
        "kreuzberg",
        "hybrid_speed",
        "hybrid_rich_metadata",
        "hybrid_balanced",
    ]

    all_results = {}

    for backend in backends_to_test:
        print(f"\n📊 Benchmarking {backend}...")
        try:
            results = benchmark_backend(backend, existing_files, iterations=3)
            all_results[backend] = results

            # Print summary
            summary = results["summary"]
            print(f"  Success rate: {summary['success_rate']:.1f}%")
            print(f"  Avg time: {summary['avg_extraction_time']:.1f}ms")
            print(f"  Total time: {summary['total_time']:.1f}ms")
            if results["errors"]:
                print(f"  Errors: {len(results['errors'])}")

        except Exception as e:
            print(f"  ❌ Failed: {e}")
            all_results[backend] = {"error": str(e)}

    return all_results


def print_comparison_report(results: dict[str, Any]) -> None:
    """Print a detailed comparison report."""
    print("\n" + "=" * 80)
    print("📈 HYBRID BACKEND PERFORMANCE COMPARISON REPORT")
    print("=" * 80)

    # Overall performance comparison
    print("\n🏆 Overall Performance Summary:")
    print("-" * 50)
    print(f"{'Backend':<20} {'Success Rate':<12} {'Avg Time':<12} {'Total Time':<12}")
    print("-" * 50)

    for backend, data in results.items():
        if "error" in data:
            print(f"{backend:<20} ERROR: {data['error']}")
            continue

        summary = data["summary"]
        print(
            f"{backend:<20} {summary['success_rate']:>8.1f}%   {summary['avg_extraction_time']:>8.1f}ms   {summary['total_time']:>8.1f}ms"
        )

    # Per-file-type analysis
    print("\n📁 Per-File-Type Performance:")
    print("-" * 80)

    # Get all file types tested
    file_types = set()
    for backend_data in results.values():
        if "files" in backend_data:
            file_types.update(backend_data["files"].keys())

    for file_type in sorted(file_types):
        print(f"\n{file_type.upper()}:")
        print(
            f"{'Backend':<20} {'Time (ms)':<12} {'Throughput':<15} {'Backend Used':<15}"
        )
        print("-" * 65)

        for backend, data in results.items():
            if "error" in data or file_type not in data.get("files", {}):
                continue

            file_data = data["files"][file_type]
            if file_data["success"]:
                throughput = f"{file_data['throughput']:.2f} MB/s"
                backend_used = file_data.get("backend_used", "unknown")
                print(
                    f"{backend:<20} {file_data['avg_time']:>8.1f}    {throughput:<15} {backend_used:<15}"
                )

    # Backend routing analysis
    print("\n🔀 Backend Routing Analysis:")
    print("-" * 50)

    routing_stats = {}
    for backend, data in results.items():
        if "error" in data or not backend.startswith("hybrid"):
            continue

        for file_type, file_data in data.get("files", {}).items():
            if file_data["success"]:
                backend_used = file_data.get("backend_used", "unknown")
                key = f"{backend}::{file_type}"
                routing_stats[key] = backend_used

    for strategy in ["hybrid_speed", "hybrid_rich_metadata", "hybrid_balanced"]:
        print(f"\n{strategy.replace('hybrid_', '').upper()} Strategy:")
        for key, backend_used in routing_stats.items():
            if key.startswith(strategy):
                file_type = key.split("::")[-1]
                print(f"  {file_type:<20} → {backend_used}")


def save_results(
    results: dict[str, Any], output_file: str = "hybrid_benchmark_results.json"
) -> None:
    """Save benchmark results to JSON file."""
    import json

    # Make results JSON serializable
    serializable_results = {}
    for backend, data in results.items():
        if "error" not in data:
            serializable_results[backend] = {
                "backend": data["backend"],
                "summary": data["summary"],
                "files": {
                    k: {**v, "times": v["times"]} for k, v in data["files"].items()
                },
                "errors": data["errors"],
            }
        else:
            serializable_results[backend] = data

    with open(output_file, "w") as f:
        json.dump(serializable_results, f, indent=2)

    print(f"\n💾 Results saved to {output_file}")


if __name__ == "__main__":
    # Run the benchmark
    results = run_comprehensive_benchmark()

    if results:
        print_comparison_report(results)
        save_results(results)

        print("\n✅ Benchmark completed successfully!")
        print(
            f"   Tested {len([r for r in results.values() if 'error' not in r])} backends"
        )
        print(f"   Total test files: {len(TEST_FILES)}")
    else:
        print("❌ Benchmark failed to run")
