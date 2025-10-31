#!/usr/bin/env python3
"""Wrapper script for unstructured extraction."""

import json
import sys
from pathlib import Path

try:
    from unstructured.partition.auto import partition
except ImportError as e:
    print(json.dumps({"error": f"unstructured not installed: {e}"}), file=sys.stderr)
    sys.exit(1)


def extract_with_unstructured(file_path: str) -> dict[str, object]:
    """Extract content using unstructured."""
    elements = partition(filename=file_path)

    content_parts = []
    metadata_items = []

    for element in elements:
        content_parts.append(str(element))
        if hasattr(element, "metadata") and element.metadata:
            metadata_items.append(element.metadata.to_dict())

    return {
        "content": "\n\n".join(content_parts),
        "metadata": {
            "num_elements": len(elements),
            "element_types": list({type(e).__name__ for e in elements}),
        },
    }


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit(1)

    file_path = sys.argv[1]
    if not Path(file_path).exists():
        sys.exit(1)

    try:
        result = extract_with_unstructured(file_path)
        print(json.dumps(result))
    except Exception as e:
        print(json.dumps({"error": str(e)}), file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
