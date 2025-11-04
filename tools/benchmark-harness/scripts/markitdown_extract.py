#!/usr/bin/env python3
"""MarkItDown extraction wrapper for benchmark harness."""

from __future__ import annotations

import json
import sys
import time

from markitdown import MarkItDown


def main() -> None:
    if len(sys.argv) != 2:
        print("Usage: markitdown_extract.py <file_path>", file=sys.stderr)
        sys.exit(1)

    file_path = sys.argv[1]

    try:
        start = time.perf_counter()
        md = MarkItDown()
        result = md.convert(file_path)
        duration_ms = (time.perf_counter() - start) * 1000.0

        payload = {
            "content": result.text_content or "",
            "metadata": {"framework": "markitdown"},
            "_extraction_time_ms": duration_ms,
        }
        print(json.dumps(payload), end="")
    except Exception as e:
        print(f"Error extracting with MarkItDown: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
