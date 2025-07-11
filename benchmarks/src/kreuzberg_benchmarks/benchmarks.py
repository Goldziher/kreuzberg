"""Core benchmark implementations comparing sync vs async performance."""

from __future__ import annotations

import asyncio
from pathlib import Path
from typing import Any, Callable

from kreuzberg import (
    ExtractionConfig,
    batch_extract_file,
    extract_file,
    extract_file_sync,
)
from kreuzberg._backend_router import extract_file_with_backend


class KreuzbergBenchmarks:
    """Benchmark suite for Kreuzberg sync vs async performance comparison."""

    def __init__(self, test_files_dir: Path | None = None) -> None:
        self.test_files_dir = test_files_dir or Path("../tests/test_source_files")
        self.test_files = self._discover_test_files()

    def _discover_test_files(self) -> list[Path]:
        """Discover available test files for benchmarking."""
        if not self.test_files_dir.exists():
            return []

        extensions = [
            ".pdf",
            ".docx",
            ".xlsx",
            ".pptx",
            ".html",
            ".md",
            ".png",
            ".jpg",
            ".jpeg",
            ".eml",
            ".json",
            ".yaml",
            ".yml",
            ".toml",
        ]
        test_files: list[Path] = []

        for ext in extensions:
            test_files.extend(self.test_files_dir.glob(f"*{ext}"))

        return sorted(test_files)[:10]

    def get_sync_benchmarks(
        self,
    ) -> list[tuple[str, Callable[[], Any], dict[str, Any]]]:
        """Get list of synchronous benchmarks."""
        benchmarks: list[tuple[str, Callable[[], Any], dict[str, Any]]] = []

        for test_file in self.test_files:
            if not test_file.exists():
                continue

            file_type = test_file.suffix[1:]
            base_name = test_file.stem

            benchmarks.append(
                (
                    f"sync_{file_type}_{base_name}_default",
                    (lambda f=test_file: extract_file_sync(f)),  # type: ignore[misc]
                    {
                        "file_type": file_type,
                        "file_name": str(test_file.name),
                        "config": "default",
                    },
                )
            )

            if file_type in ["pdf", "png", "jpg", "jpeg"]:
                benchmarks.append(
                    (
                        f"sync_{file_type}_{base_name}_force_ocr",
                        (
                            lambda f=test_file: extract_file_sync(  # type: ignore[misc]
                                f, config=ExtractionConfig(force_ocr=True)
                            )
                        ),
                        {
                            "file_type": file_type,
                            "file_name": str(test_file.name),
                            "config": "force_ocr",
                        },
                    )
                )

        if len(self.test_files) >= 3:
            small_batch = self.test_files[:3]
            medium_batch = self.test_files[: min(5, len(self.test_files))]

            benchmarks.extend(
                [
                    (
                        "sync_batch_small",
                        (lambda: [extract_file_sync(f) for f in small_batch]),
                        {"batch_size": str(len(small_batch)), "config": "sequential"},
                    ),
                    (
                        "sync_batch_medium",
                        (lambda: [extract_file_sync(f) for f in medium_batch]),
                        {"batch_size": str(len(medium_batch)), "config": "sequential"},
                    ),
                ]
            )

        return benchmarks

    def get_async_benchmarks(
        self,
    ) -> list[tuple[str, Callable[[], Any], dict[str, Any]]]:
        """Get list of asynchronous benchmarks."""
        benchmarks: list[tuple[str, Callable[[], Any], dict[str, Any]]] = []

        for test_file in self.test_files:
            if not test_file.exists():
                continue

            file_type = test_file.suffix[1:]
            base_name = test_file.stem

            benchmarks.append(
                (
                    f"async_{file_type}_{base_name}_default",
                    (lambda f=test_file: extract_file(f)),  # type: ignore[misc]
                    {
                        "file_type": file_type,
                        "file_name": str(test_file.name),
                        "config": "default",
                    },
                )
            )

            if file_type in ["pdf", "png", "jpg", "jpeg"]:
                benchmarks.append(
                    (
                        f"async_{file_type}_{base_name}_force_ocr",
                        (
                            lambda f=test_file: extract_file(  # type: ignore[misc]
                                f, config=ExtractionConfig(force_ocr=True)
                            )
                        ),
                        {
                            "file_type": file_type,
                            "file_name": str(test_file.name),
                            "config": "force_ocr",
                        },
                    )
                )

        if len(self.test_files) >= 3:
            small_batch = self.test_files[:3]
            medium_batch = self.test_files[: min(5, len(self.test_files))]

            benchmarks.extend(
                [
                    (
                        "async_batch_small_concurrent",
                        (lambda: batch_extract_file(small_batch)),
                        {"batch_size": str(len(small_batch)), "config": "concurrent"},
                    ),
                    (
                        "async_batch_medium_concurrent",
                        (lambda: batch_extract_file(medium_batch)),
                        {"batch_size": str(len(medium_batch)), "config": "concurrent"},
                    ),
                    (
                        "async_batch_small_sequential",
                        (
                            lambda: asyncio.gather(
                                *[extract_file(f) for f in small_batch]
                            )
                        ),
                        {
                            "batch_size": str(len(small_batch)),
                            "config": "sequential_async",
                        },
                    ),
                ]
            )

        return benchmarks

    def get_comparison_benchmarks(
        self,
    ) -> list[tuple[str, Callable[[], Any], dict[str, Any]]]:
        """Get benchmarks specifically for sync vs async comparison."""
        if not self.test_files:
            return []

        test_file = self.test_files[0]
        benchmarks: list[tuple[str, Callable[[], Any], dict[str, Any]]] = []

        benchmarks.extend(
            [
                (
                    "comparison_sync_default",
                    lambda: extract_file_sync(test_file),
                    {
                        "type": "sync",
                        "operation": "single_file",
                        "file": str(test_file.name),
                    },
                ),
                (
                    "comparison_async_default",
                    lambda: extract_file(test_file),
                    {
                        "type": "async",
                        "operation": "single_file",
                        "file": str(test_file.name),
                    },
                ),
            ]
        )

        if len(self.test_files) >= 3:
            batch_files = self.test_files[:3]
            benchmarks.extend(
                [
                    (
                        "comparison_sync_batch",
                        lambda: [extract_file_sync(f) for f in batch_files],
                        {
                            "type": "sync",
                            "operation": "batch",
                            "batch_size": str(len(batch_files)),
                        },
                    ),
                    (
                        "comparison_async_batch",
                        lambda: batch_extract_file(batch_files),
                        {
                            "type": "async",
                            "operation": "batch",
                            "batch_size": str(len(batch_files)),
                        },
                    ),
                ]
            )

        return benchmarks

    def get_stress_benchmarks(
        self,
    ) -> list[tuple[str, Callable[[], Any], dict[str, Any]]]:
        """Get stress test benchmarks for performance limits."""
        if len(self.test_files) < 2:
            return []

        all_files = self.test_files * 2
        large_batch = all_files[: min(10, len(all_files))]

        return [
            (
                "stress_sync_large_batch",
                lambda: [extract_file_sync(f) for f in large_batch],
                {
                    "type": "stress",
                    "operation": "sync_batch",
                    "batch_size": len(large_batch),
                },
            ),
            (
                "stress_async_large_batch",
                lambda: batch_extract_file(large_batch),
                {
                    "type": "stress",
                    "operation": "async_batch",
                    "batch_size": len(large_batch),
                },
            ),
        ]

    def get_backend_benchmarks(
        self,
    ) -> list[tuple[str, Callable[[], Any], dict[str, Any]]]:
        """Get benchmarks for comparing different extraction backends."""
        benchmarks: list[tuple[str, Callable[[], Any], dict[str, Any]]] = []
        backends = ["kreuzberg", "extractous", "hybrid", "auto"]

        for test_file in self.test_files:
            if not test_file.exists():
                continue

            file_type = test_file.suffix[1:]
            base_name = test_file.stem

            for backend in backends:
                benchmarks.append(
                    (
                        f"backend_{backend}_{file_type}_{base_name}",
                        (
                            lambda f=test_file, b=backend: extract_file_with_backend(
                                f, config=ExtractionConfig(extraction_backend=b)
                            )
                        ),
                        {
                            "file_type": file_type,
                            "file_name": str(test_file.name),
                            "backend": backend,
                            "operation": "extract_with_backend",
                        },
                    )
                )

        return benchmarks
