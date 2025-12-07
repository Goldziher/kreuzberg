"""Print the installed kreuzberg Python package version."""

from __future__ import annotations

import sys


def main() -> int:
    try:
        import kreuzberg  # type: ignore
    except Exception as exc:  # pragma: no cover - runtime helper
        print(f"Failed to import kreuzberg: {exc}", file=sys.stderr)
        return 1

    print(f"Kreuzberg version: {getattr(kreuzberg, '__version__', 'unknown')}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
