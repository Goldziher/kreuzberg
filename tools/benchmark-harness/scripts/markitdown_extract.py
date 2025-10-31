#!/usr/bin/env python3
"""Wrapper script for markitdown extraction."""

import json
import sys
import time
from pathlib import Path

try:
    from markitdown import MarkItDown
except ImportError as e:
    print(json.dumps({"error": f"markitdown not installed: {e}"}), file=sys.stderr)
    sys.exit(1)


def extract_with_markitdown(file_path: str) -> dict[str, object]:
    """Extract content using markitdown."""
    start = time.perf_counter()

    md = MarkItDown()
    result = md.convert(file_path)

    extraction_ms = (time.perf_counter() - start) * 1000

    return {
        "content": result.text_content,
        "metadata": {
            "title": getattr(result, "title", None),
        },
        "_extraction_time_ms": extraction_ms,
    }


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit(1)

    file_path = sys.argv[1]
    if not Path(file_path).exists():
        sys.exit(1)

    try:
        result = extract_with_markitdown(file_path)
        print(json.dumps(result))
    except Exception as e:
        print(json.dumps({"error": str(e)}), file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
