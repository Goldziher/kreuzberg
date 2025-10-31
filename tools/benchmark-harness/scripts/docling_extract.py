#!/usr/bin/env python3
"""Wrapper script for docling extraction."""

import json
import sys
from pathlib import Path

try:
    from docling.document_converter import DocumentConverter
except ImportError as e:
    print(json.dumps({"error": f"docling not installed: {e}"}), file=sys.stderr)
    sys.exit(1)


def extract_with_docling(file_path: str) -> dict[str, object]:
    """Extract content using docling."""
    converter = DocumentConverter()
    result = converter.convert(file_path)

    return {
        "content": result.document.export_to_markdown(),
        "metadata": {
            "num_pages": getattr(result.document, "num_pages", None),
            "title": getattr(result.document, "title", None),
        },
    }


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit(1)

    file_path = sys.argv[1]
    if not Path(file_path).exists():
        sys.exit(1)

    try:
        result = extract_with_docling(file_path)
        print(json.dumps(result))
    except Exception as e:
        print(json.dumps({"error": str(e)}), file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
