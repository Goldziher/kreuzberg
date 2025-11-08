"""Unstructured extraction wrapper for benchmark harness."""

from __future__ import annotations

import json
import sys
import time

from unstructured.partition.auto import partition


def main() -> None:
    if len(sys.argv) != 2:
        print("Usage: unstructured_extract.py <file_path>", file=sys.stderr)
        sys.exit(1)

    file_path = sys.argv[1]

    try:
        start = time.perf_counter()
        elements = partition(filename=file_path)
        duration_ms = (time.perf_counter() - start) * 1000.0

        text = "\n\n".join(str(el) for el in elements)
        payload = {
            "content": text,
            "metadata": {"framework": "unstructured"},
            "_extraction_time_ms": duration_ms,
        }
        print(json.dumps(payload), end="")
    except Exception as e:
        print(f"Error extracting with Unstructured: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
