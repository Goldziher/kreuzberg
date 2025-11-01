#!/usr/bin/env python3
"""MarkItDown extraction wrapper for benchmark harness."""

import sys

from markitdown import MarkItDown


def main() -> None:
    if len(sys.argv) != 2:
        print("Usage: markitdown_extract.py <file_path>", file=sys.stderr)
        sys.exit(1)

    file_path = sys.argv[1]

    try:
        md = MarkItDown()
        result = md.convert(file_path)

        # Print extracted markdown
        print(result.text_content, end="")
    except Exception as e:
        print(f"Error extracting with MarkItDown: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
