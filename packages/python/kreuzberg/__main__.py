"""Proxy entry point that forwards to the Rust-based Kreuzberg CLI.

This keeps `python -m kreuzberg` and the `kreuzberg` console script working
without shipping an additional Python CLI implementation.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Sequence


def main(argv: Sequence[str] | None = None) -> int:
    """Execute the Rust CLI with the provided arguments."""
    args = list(argv[1:] if argv is not None else sys.argv[1:])

    # Try to find the CLI binary on PATH first (production mode)
    cli_path = shutil.which("kreuzberg-cli")

    # In development mode, look for the binary in target/release
    if cli_path is None:
        # Try to find workspace root and look in target/release
        dev_binary = Path(__file__).parent.parent.parent.parent / "target" / "release" / "kreuzberg"
        if dev_binary.exists():
            cli_path = str(dev_binary)

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
