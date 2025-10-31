#!/usr/bin/env python3
"""Docling extraction wrapper for benchmark harness."""

import sys

from docling.document_converter import DocumentConverter


def main() -> None:
    if len(sys.argv) != 2:
        print("Usage: docling_extract.py <file_path>", file=sys.stderr)
        sys.exit(1)

    file_path = sys.argv[1]

    try:
        converter = DocumentConverter()
        result = converter.convert(file_path)

        # Extract markdown text
        print(result.document.export_to_markdown(), end="")
    except Exception as e:
        print(f"Error extracting with Docling: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
