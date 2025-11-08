"""Kreuzberg Python extraction wrapper for benchmark harness.

Supports three modes:
- sync: extract_file_sync() - synchronous extraction
- async: extract_file() - asynchronous extraction
- batch: batch_extract_files_sync() - synchronous batch extraction
"""

from __future__ import annotations

import asyncio
import json
import sys
import time
from typing import Any

from kreuzberg import batch_extract_files_sync, extract_file, extract_file_sync


def extract_sync(file_path: str) -> dict[str, Any]:
    """Extract using synchronous API."""
    start = time.perf_counter()
    result = extract_file_sync(file_path)
    duration_ms = (time.perf_counter() - start) * 1000.0

    return {
        "content": result.content,
        "metadata": result.metadata or {},
        "_extraction_time_ms": duration_ms,
    }


async def extract_async(file_path: str) -> dict[str, Any]:
    """Extract using asynchronous API."""
    start = time.perf_counter()
    result = await extract_file(file_path)
    duration_ms = (time.perf_counter() - start) * 1000.0

    return {
        "content": result.content,
        "metadata": result.metadata or {},
        "_extraction_time_ms": duration_ms,
    }


def extract_batch_sync(file_paths: list[str]) -> list[dict[str, Any]]:
    """Extract multiple files using batch API."""
    start = time.perf_counter()
    results = batch_extract_files_sync(file_paths)  # type: ignore[arg-type]
    total_duration_ms = (time.perf_counter() - start) * 1000.0

    per_file_duration_ms = total_duration_ms / len(file_paths) if file_paths else 0

    return [
        {
            "content": result.content,
            "metadata": result.metadata or {},
            "_extraction_time_ms": per_file_duration_ms,
            "_batch_total_ms": total_duration_ms,
        }
        for result in results
    ]


def main() -> None:
    if len(sys.argv) < 3:
        print("Usage: kreuzberg_extract.py <mode> <file_path> [additional_files...]", file=sys.stderr)
        print("Modes: sync, async, batch", file=sys.stderr)
        sys.exit(1)

    mode = sys.argv[1]
    file_paths = sys.argv[2:]

    try:
        if mode == "sync":
            if len(file_paths) != 1:
                print("Error: sync mode requires exactly one file", file=sys.stderr)
                sys.exit(1)
            payload = extract_sync(file_paths[0])
            print(json.dumps(payload), end="")

        elif mode == "async":
            if len(file_paths) != 1:
                print("Error: async mode requires exactly one file", file=sys.stderr)
                sys.exit(1)
            payload = asyncio.run(extract_async(file_paths[0]))
            print(json.dumps(payload), end="")

        elif mode == "batch":
            if len(file_paths) < 1:
                print("Error: batch mode requires at least one file", file=sys.stderr)
                sys.exit(1)

            if len(file_paths) == 1:
                results = extract_batch_sync(file_paths)
                print(json.dumps(results[0]), end="")
            else:
                results = extract_batch_sync(file_paths)
                print(json.dumps(results), end="")

        else:
            print(f"Error: Unknown mode '{mode}'. Use sync, async, or batch", file=sys.stderr)
            sys.exit(1)

    except Exception as e:
        print(f"Error extracting with Kreuzberg: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
