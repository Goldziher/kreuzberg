#!/usr/bin/env python3
"""Extractous extraction wrapper for benchmark harness."""

import sys

from extractous import Extractor


def main() -> None:
    if len(sys.argv) != 2:
        print("Usage: extractous_extract.py <file_path>", file=sys.stderr)
        sys.exit(1)

    file_path = sys.argv[1]

    try:
        extractor = Extractor()
        result = extractor.extract_file(file_path)

        # Print extracted text
        print(result, end="")
    except Exception as e:
        print(f"Error extracting with Extractous: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
