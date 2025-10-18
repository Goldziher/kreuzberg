"""Proxy entry point that forwards to the Rust-based Kreuzberg CLI.

This keeps `python -m kreuzberg` and the `kreuzberg` console script working
without shipping an additional Python CLI implementation.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Sequence


def main(argv: Sequence[str] | None = None) -> int:
    """Execute the Rust CLI with the provided arguments."""
    args = list(argv[1:] if argv is not None else sys.argv[1:])

    cli_path = shutil.which("kreuzberg-cli")
    if cli_path is None:
        sys.stderr.write(
            "The embedded Kreuzberg CLI binary could not be located. "
            "This indicates a packaging issue with the wheel; please open an issue at "
            "https://github.com/Goldziher/kreuzberg/issues so we can investigate.\n",
        )
        return 1

    completed = subprocess.run([cli_path, *args], check=False)
    return completed.returncode


if __name__ == "__main__":
    raise SystemExit(main())
