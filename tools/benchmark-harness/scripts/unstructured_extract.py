#!/usr/bin/env python3
"""Unstructured extraction wrapper for benchmark harness."""

import sys

from unstructured.partition.auto import partition


def main() -> None:
    if len(sys.argv) != 2:
        print("Usage: unstructured_extract.py <file_path>", file=sys.stderr)
        sys.exit(1)

    file_path = sys.argv[1]

    try:
        elements = partition(filename=file_path)

        # Extract and print text from all elements
        text = "\n\n".join([str(el) for el in elements])
        print(text, end="")
    except Exception as e:
        print(f"Error extracting with Unstructured: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
