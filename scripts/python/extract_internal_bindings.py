#!/usr/bin/env python3
"""
Extract compiled _internal_bindings artifacts from a wheel into the package directory.

Args:
    1: Wheel path
    2: Target package directory (e.g., packages/python/kreuzberg)
"""

from __future__ import annotations

import shutil
import sys
import zipfile
from pathlib import Path


def main() -> int:
    if len(sys.argv) != 3:
        print("Usage: extract_internal_bindings.py <wheel-path> <target-dir>", file=sys.stderr)
        return 1

    wheel_path = Path(sys.argv[1]).expanduser().resolve()
    target_dir = Path(sys.argv[2]).expanduser().resolve()
    target_dir.mkdir(parents=True, exist_ok=True)

    if not wheel_path.is_file():
        print(f"Wheel not found: {wheel_path}", file=sys.stderr)
        return 1

    copied = False
    with zipfile.ZipFile(wheel_path) as zf:
        for member in zf.namelist():
            if not member.startswith("kreuzberg/_internal_bindings"):
                continue
            if member.endswith(("/", ".py", ".pyi")):
                continue
            destination = target_dir / Path(member).name
            with zf.open(member) as src, destination.open("wb") as dst:
                shutil.copyfileobj(src, dst)
            copied = True

    if not copied:
        print(f"No compiled bindings found inside {wheel_path}", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
