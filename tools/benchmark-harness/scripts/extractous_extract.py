"""Extractous extraction wrapper for benchmark harness."""

from __future__ import annotations

import json
import sys
import time

from extractous import Extractor


def main() -> None:
    if len(sys.argv) != 2:
        print("Usage: extractous_extract.py <file_path>", file=sys.stderr)
        sys.exit(1)

    file_path = sys.argv[1]

    try:
        extractor = Extractor()
        start = time.perf_counter()
        text, metadata = extractor.extract_file_to_string(file_path)
        duration_ms = (time.perf_counter() - start) * 1000.0

        payload = {
            "content": text or "",
            "metadata": metadata or {"framework": "extractous"},
            "_extraction_time_ms": duration_ms,
        }
        print(json.dumps(payload), end="")
    except Exception as e:
        print(f"Error extracting with Extractous: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
