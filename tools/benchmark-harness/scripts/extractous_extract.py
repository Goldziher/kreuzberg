#!/usr/bin/env python3
"""Wrapper script for extractous extraction."""

import json
import sys
import time
from pathlib import Path

try:
    from extractous import Extractor
except ImportError as e:
    print(json.dumps({"error": f"extractous not installed: {e}"}), file=sys.stderr)
    sys.exit(1)


def extract_with_extractous(file_path: str) -> dict[str, object]:
    """Extract content using extractous."""
    start = time.perf_counter()

    extractor = Extractor()
    result = extractor.extract_file(file_path)

    extraction_ms = (time.perf_counter() - start) * 1000

    metadata = {}
    if hasattr(result, "metadata"):
        metadata = {k: v for k, v in result.metadata.items() if isinstance(v, (str, int, float, bool, type(None)))}

    return {
        "content": result.text if hasattr(result, "text") else str(result),
        "metadata": metadata,
        "_extraction_time_ms": extraction_ms,
    }


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit(1)

    file_path = sys.argv[1]
    if not Path(file_path).exists():
        sys.exit(1)

    try:
        result = extract_with_extractous(file_path)
        print(json.dumps(result))
    except Exception as e:
        print(json.dumps({"error": str(e)}), file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
