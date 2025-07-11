"""Comprehensive benchmarking module for kreuzberg."""

from __future__ import annotations

import json
import statistics
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from kreuzberg._backend_router import extract_file_with_backend
from kreuzberg._types import ExtractionConfig, ExtractionResult


@dataclass
class BenchmarkResult:
    """Result of a single benchmark run."""

    backend: str
    file_path: Path
    file_type: str
    success: bool
    error: str | None = None

    # Performance metrics
    extraction_time: float = 0.0
    text_length: int = 0
    throughput: float = 0.0  # chars/second

    # Quality metrics
    metadata_count: int = 0
    metadata_fields: list[str] = field(default_factory=list)
    has_ocr: bool = False
    mime_type: str | None = None

    # Raw result for detailed analysis
    raw_result: ExtractionResult | None = None


@dataclass
class FileTypeBenchmark:
    """Aggregated benchmark results for a file type."""

    file_type: str
    backends: dict[str, list[BenchmarkResult]] = field(default_factory=dict)

    def add_result(self, result: BenchmarkResult) -> None:
        """Add a benchmark result."""
        if result.backend not in self.backends:
            self.backends[result.backend] = []
        self.backends[result.backend].append(result)

    def get_summary(self) -> dict[str, Any]:
        """Get summary statistics for this file type."""
        summary: dict[str, Any] = {"file_type": self.file_type, "backends": {}}

        for backend, results in self.backends.items():
            successful = [r for r in results if r.success]
            if not successful:
                summary["backends"][backend] = {
                    "success_rate": 0.0,
                    "error": results[0].error if results else "No results",
                }
                continue

            extraction_times = [r.extraction_time for r in successful]
            throughputs = [r.throughput for r in successful if r.throughput > 0]
            metadata_counts = [r.metadata_count for r in successful]

            summary["backends"][backend] = {
                "success_rate": len(successful) / len(results) * 100,
                "avg_extraction_time": statistics.mean(extraction_times),
                "std_extraction_time": statistics.stdev(extraction_times) if len(extraction_times) > 1 else 0,
                "avg_throughput": statistics.mean(throughputs) if throughputs else 0,
                "avg_metadata_fields": statistics.mean(metadata_counts),
                "total_runs": len(results),
                "successful_runs": len(successful),
                "has_ocr": any(r.has_ocr for r in successful),
                "unique_metadata_fields": sorted({field for r in successful for field in r.metadata_fields})[
                    :10
                ],  # Top 10 fields
            }

        return summary


class BenchmarkRunner:
    """Run comprehensive benchmarks across backends."""

    def __init__(self, iterations: int = 3, backends: list[str] | None = None) -> None:
        """Initialize benchmark runner.

        Args:
            iterations: Number of times to run each benchmark
            backends: List of backends to test (default: all available)
        """
        self.iterations = iterations
        self.backends = backends or ["kreuzberg", "extractous", "hybrid", "auto"]
        self.results: dict[str, FileTypeBenchmark] = {}

    def run_single_benchmark(self, file_path: Path, backend: str, file_type: str) -> BenchmarkResult:
        """Run a single benchmark test."""
        start_time = time.perf_counter()

        try:
            config = ExtractionConfig(extraction_backend=backend)  # type: ignore[arg-type]
            result = extract_file_with_backend(file_path, config=config)

            extraction_time = time.perf_counter() - start_time
            text_length = len(result.content)
            throughput = text_length / extraction_time if extraction_time > 0 else 0

            # Check for OCR (heuristic: images with text content)
            has_ocr = (file_type == "image" and text_length > 0) or (
                result.metadata and any("ocr" in str(v).lower() for v in result.metadata.values())
            )

            return BenchmarkResult(
                backend=backend,
                file_path=file_path,
                file_type=file_type,
                success=True,
                extraction_time=extraction_time,
                text_length=text_length,
                throughput=throughput,
                metadata_count=len(result.metadata) if result.metadata else 0,
                metadata_fields=list(result.metadata.keys()) if result.metadata else [],
                has_ocr=bool(has_ocr),
                mime_type=result.mime_type,
                raw_result=result,
            )

        except Exception as e:
            extraction_time = time.perf_counter() - start_time
            return BenchmarkResult(
                backend=backend,
                file_path=file_path,
                file_type=file_type,
                success=False,
                error=f"{type(e).__name__}: {e!s}",
                extraction_time=extraction_time,
            )

    def benchmark_file(self, file_path: Path, file_type: str) -> None:
        """Benchmark a single file across all backends."""
        if file_type not in self.results:
            self.results[file_type] = FileTypeBenchmark(file_type)

        for backend in self.backends:
            for _i in range(self.iterations):
                result = self.run_single_benchmark(file_path, backend, file_type)
                self.results[file_type].add_result(result)

    def benchmark_directory(self, directory: Path, file_patterns: dict[str, str] | None = None) -> None:
        """Benchmark all files in a directory.

        Args:
            directory: Directory containing test files
            file_patterns: Dict mapping file types to glob patterns
        """
        if file_patterns is None:
            file_patterns = {
                "pdf": "*.pdf",
                "docx": "*.docx",
                "xlsx": "*.xlsx",
                "image": "*.{jpg,jpeg,png}",
                "html": "*.{html,htm}",
                "json": "*.json",
                "yaml": "*.{yaml,yml}",
                "email": "*.eml",
                "text": "*.txt",
            }

        for file_type, pattern in file_patterns.items():
            for file_path in directory.glob(pattern):
                if file_path.is_file():
                    self.benchmark_file(file_path, file_type)

    def get_results(self) -> dict[str, Any]:
        """Get comprehensive benchmark results."""
        return {
            "summary": self._get_summary(),
            "detailed": {file_type: benchmark.get_summary() for file_type, benchmark in self.results.items()},
            "recommendations": self._get_recommendations(),
        }

    def _get_summary(self) -> dict[str, Any]:
        """Get overall summary across all file types."""
        backend_stats: dict[str, Any] = {}

        for backend in self.backends:
            all_results = []
            for file_type_bench in self.results.values():
                if backend in file_type_bench.backends:
                    all_results.extend(file_type_bench.backends[backend])

            if not all_results:
                continue

            successful = [r for r in all_results if r.success]
            if successful:
                backend_stats[backend] = {
                    "overall_success_rate": len(successful) / len(all_results) * 100,
                    "avg_extraction_time": statistics.mean(r.extraction_time for r in successful),
                    "avg_throughput": statistics.mean(r.throughput for r in successful if r.throughput > 0),
                    "total_metadata_fields": sum(r.metadata_count for r in successful),
                    "formats_with_metadata": len({r.file_type for r in successful if r.metadata_count > 0}),
                    "formats_with_ocr": len({r.file_type for r in successful if r.has_ocr}),
                }
            else:
                backend_stats[backend] = {
                    "overall_success_rate": 0.0,
                    "avg_extraction_time": 0.0,
                    "avg_throughput": 0.0,
                    "total_metadata_fields": 0,
                    "formats_with_metadata": 0,
                    "formats_with_ocr": 0,
                    "error": "No successful extractions",
                }

        return backend_stats

    def _get_recommendations(self) -> dict[str, dict[str, Any]]:
        """Get recommendations for optimal backend per file type."""
        recommendations = {}

        for file_type, benchmark in self.results.items():
            scores = {}

            for backend, results in benchmark.backends.items():
                successful = [r for r in results if r.success]
                if not successful:
                    continue

                # Calculate weighted score
                success_rate = len(successful) / len(results)
                avg_speed = statistics.mean(r.extraction_time for r in successful)
                avg_metadata = statistics.mean(r.metadata_count for r in successful)
                has_ocr = any(r.has_ocr for r in successful)

                # Normalize scores (lower time is better)
                speed_score = 1.0 / (avg_speed + 0.001)  # Avoid division by zero
                metadata_score = avg_metadata / 10.0  # Normalize to 0-1 range
                ocr_score = 1.0 if has_ocr else 0.0

                # Weighted combination
                if file_type == "image":
                    # For images, OCR is most important
                    total_score = success_rate * 0.3 + speed_score * 0.1 + metadata_score * 0.2 + ocr_score * 0.4
                else:
                    # For other formats, balance speed and metadata
                    total_score = success_rate * 0.3 + speed_score * 0.3 + metadata_score * 0.4

                scores[backend] = {
                    "score": total_score,
                    "success_rate": success_rate * 100,
                    "avg_time": avg_speed,
                    "avg_metadata_fields": avg_metadata,
                    "has_ocr": has_ocr,
                }

            if scores:
                best_backend = max(scores.items(), key=lambda x: x[1]["score"])
                recommendations[file_type] = {
                    "recommended_backend": best_backend[0],
                    "reason": self._get_recommendation_reason(file_type, best_backend[0], scores),
                    "scores": scores,
                }

        return recommendations

    def _get_recommendation_reason(self, file_type: str, backend: str, scores: dict[str, dict[str, Any]]) -> str:
        """Generate human-readable recommendation reason."""
        backend_score = scores[backend]

        reasons = []

        perfect_success_rate = 100
        high_success_rate = 90
        if backend_score["success_rate"] == perfect_success_rate:
            reasons.append("100% success rate")
        elif backend_score["success_rate"] >= high_success_rate:
            reasons.append(f"{backend_score['success_rate']:.0f}% success rate")

        # Compare to other backends
        other_backends = [b for b in scores if b != backend]
        if other_backends:
            avg_time_others = statistics.mean(scores[b]["avg_time"] for b in other_backends)
            if backend_score["avg_time"] < avg_time_others * 0.5:
                reasons.append(f"{avg_time_others / backend_score['avg_time']:.1f}x faster")

            avg_metadata_others = statistics.mean(scores[b]["avg_metadata_fields"] for b in other_backends)
            if backend_score["avg_metadata_fields"] > avg_metadata_others * 1.5:
                reasons.append(f"{backend_score['avg_metadata_fields']:.0f} metadata fields")

        if file_type == "image" and backend_score["has_ocr"]:
            reasons.append("includes OCR")

        return ", ".join(reasons) if reasons else "best overall performance"

    def save_results(self, output_path: Path) -> None:
        """Save benchmark results to JSON file."""
        results = self.get_results()
        with output_path.open("w") as f:
            json.dump(results, f, indent=2, default=str)

    def print_summary(self) -> None:
        """Print a formatted summary of results."""
        results = self.get_results()

        # Overall performance
        for stats in results["summary"].values():
            if stats.get("overall_success_rate", 0) > 0:
                if stats.get("formats_with_ocr", 0) > 0:
                    pass
            else:
                pass

        # Recommendations

        for _file_type, rec in sorted(results["recommendations"].items()):
            # Show comparison
            for _backend, _score in sorted(rec["scores"].items(), key=lambda x: x[1]["score"], reverse=True):
                pass


def create_benchmark_test_set(output_dir: Path) -> dict[str, list[Path]]:
    """Create a standard test set for benchmarking.

    Returns dict mapping file types to lists of test files.
    """
    test_files = {
        "pdf": [
            "tests/test_source_files/sample-contract.pdf",
            "tests/test_source_files/test-article.pdf",
            "tests/test_source_files/searchable.pdf",
        ],
        "docx": [
            "tests/test_source_files/document.docx",
        ],
        "xlsx": [
            "tests/test_source_files/excel.xlsx",
            "tests/test_source_files/excel-multi-sheet.xlsx",
        ],
        "image": [
            "tests/test_source_files/ocr-image.jpg",
        ],
        "json": [
            "tests/test_source_files/json/books.json",
        ],
        "yaml": [
            "tests/test_source_files/yaml/sample-config.yaml",
        ],
        "email": [
            "tests/test_source_files/email/sample-email.eml",
        ],
        "html": [
            "tests/test_source_files/html.html",
        ],
    }

    # Convert to Path objects and filter existing files
    result = {}
    for file_type, paths in test_files.items():
        existing = [Path(p) for p in paths if Path(p).exists()]
        if existing:
            result[file_type] = existing

    return result
