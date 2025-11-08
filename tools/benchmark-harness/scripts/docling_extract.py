"""Docling extraction wrapper for benchmark harness.

Supports two modes:
- sync: convert() - synchronous single-file extraction
- batch: convert_all() - batch extraction for multiple files
"""

from __future__ import annotations

import json
import sys
import time
from typing import Any

from docling.document_converter import DocumentConverter


def extract_sync(file_path: str) -> dict[str, Any]:
    """Extract using synchronous single-file API."""
    start = time.perf_counter()
    converter = DocumentConverter()
    result = converter.convert(file_path)
    markdown = result.document.export_to_markdown()
    duration_ms = (time.perf_counter() - start) * 1000.0

    return {
        "content": markdown,
        "metadata": {"framework": "docling"},
        "_extraction_time_ms": duration_ms,
    }


def extract_batch(file_paths: list[str]) -> list[dict[str, Any]]:
    """Extract multiple files using batch API."""
    start = time.perf_counter()
    converter = DocumentConverter()
    results = converter.convert_all(file_paths, raises_on_error=False)
    total_duration_ms = (time.perf_counter() - start) * 1000.0

    per_file_duration_ms = total_duration_ms / len(file_paths) if file_paths else 0

    outputs = []
    for result in results:
        if result.status.name == "SUCCESS":
            markdown = result.document.export_to_markdown()
            outputs.append(
                {
                    "content": markdown,
                    "metadata": {"framework": "docling"},
                    "_extraction_time_ms": per_file_duration_ms,
                    "_batch_total_ms": total_duration_ms,
                }
            )
        else:
            outputs.append(
                {
                    "content": "",
                    "metadata": {
                        "framework": "docling",
                        "error": str(result.errors) if result.errors else "Unknown error",
                        "status": result.status.name,
                    },
                    "_extraction_time_ms": per_file_duration_ms,
                    "_batch_total_ms": total_duration_ms,
                }
            )

    return outputs


def main() -> None:
    if len(sys.argv) < 3:
        print("Usage: docling_extract.py <mode> <file_path> [additional_files...]", file=sys.stderr)
        print("Modes: sync, batch", file=sys.stderr)
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

        elif mode == "batch":
            if len(file_paths) < 1:
                print("Error: batch mode requires at least one file", file=sys.stderr)
                sys.exit(1)

            if len(file_paths) == 1:
                results = extract_batch(file_paths)
                print(json.dumps(results[0]), end="")
            else:
                results = extract_batch(file_paths)
                print(json.dumps(results), end="")

        else:
            print(f"Error: Unknown mode '{mode}'. Use sync or batch", file=sys.stderr)
            sys.exit(1)

    except Exception as e:
        print(f"Error extracting with Docling: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
