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


_FEATURE_SUBCOMMANDS: dict[str, str] = {"serve": "all", "mcp": "all"}


def _iter_dev_cli_candidates(workspace_root: Path) -> list[Path]:
    suffixes = [".exe"] if sys.platform == "win32" else [""]
    candidate_dirs = ("target/release", "target/debug")
    candidate_names = ("kreuzberg-cli", "kreuzberg")

    candidates: list[Path] = []
    for directory in candidate_dirs:
        for name in candidate_names:
            for suffix in suffixes:
                candidate = workspace_root / directory / f"{name}{suffix}"
                if candidate.exists():
                    candidates.append(candidate)
    return candidates


def _binary_supports_subcommand(binary: Path, subcommand: str) -> bool:
    try:
        probe = subprocess.run(
            [str(binary), subcommand, "--help"],
            capture_output=True,
            text=True,
            check=False,
            timeout=10,
        )

        if probe.returncode == 0:
            return True

        stderr = probe.stderr.lower()
        return subcommand not in stderr or "unrecognized subcommand" not in stderr
    except subprocess.TimeoutExpired:
        return False


def _build_cli_with_features(workspace_root: Path, feature: str) -> bool:
    """Build CLI with specified features. Returns True if successful."""
    cargo = shutil.which("cargo")
    if cargo is None:
        return False

    try:
        result = subprocess.run(
            [cargo, "build", "-p", "kreuzberg-cli", "--features", feature],
            cwd=workspace_root,
            check=False,
            capture_output=True,
            timeout=300,  # 5 minute timeout for builds
        )
        return result.returncode == 0
    except (subprocess.TimeoutExpired, OSError):
        return False


def _discover_dev_cli_binary(requested_subcommand: str | None) -> str | None:
    """Return the path to a locally built CLI binary if available."""
    workspace_root = Path(__file__).resolve().parents[3]
    candidates = _iter_dev_cli_candidates(workspace_root)

    if requested_subcommand is None:
        if candidates:
            return str(candidates[0])
        return None

    for candidate in candidates:
        if _binary_supports_subcommand(candidate, requested_subcommand):
            return str(candidate)

    # Don't try to build in non-development scenarios
    if not (workspace_root / "Cargo.toml").exists():
        return None

    feature = _FEATURE_SUBCOMMANDS.get(requested_subcommand)
    if feature is None:
        return None

    if not _build_cli_with_features(workspace_root, feature):
        return None

    for candidate in _iter_dev_cli_candidates(workspace_root):
        if _binary_supports_subcommand(candidate, requested_subcommand):
            return str(candidate)

    return None


def _find_packaged_cli_binary() -> str | None:
    """Look for the CLI binary in common installation paths before building one."""
    # First check if it's in the package directory (for wheel installations)
    package_dir = Path(__file__).parent
    for name in ("kreuzberg-cli", "kreuzberg", "kreuzberg-cli.exe", "kreuzberg.exe"):
        candidate = package_dir / name
        if candidate.exists() and candidate.is_file():
            return str(candidate)

    # Then check venv bin directory
    script_dir = Path(sys.executable).parent
    for name in ("kreuzberg-cli", "kreuzberg"):
        candidate = script_dir / name
        if candidate.exists():
            try:
                with candidate.open("rb") as f:
                    header = f.read(2)
                    if header == b"#!":
                        continue
            except OSError:
                continue
            return str(candidate)
    return None


def main(argv: Sequence[str] | None = None) -> int:
    """Execute the Rust CLI with the provided arguments."""
    args = list(argv[1:] if argv is not None else sys.argv[1:])

    requested_subcommand: str | None = None
    if args:
        first = args[0]
        if not first.startswith("-"):
            requested_subcommand = first

    cli_path = shutil.which("kreuzberg-cli")

    if cli_path is None:
        cli_path = _find_packaged_cli_binary()

    if cli_path is None:
        cli_path = _discover_dev_cli_binary(requested_subcommand)

    if cli_path is None:
        sys.stderr.write(
            "The embedded Kreuzberg CLI binary could not be located. "
            "This indicates a packaging issue with the wheel; please open an issue at "
            "https://github.com/kreuzberg-dev/kreuzberg/issues so we can investigate.\n",
        )
        return 1

    completed = subprocess.run([cli_path, *args], check=False)
    return completed.returncode


if __name__ == "__main__":
    raise SystemExit(main())
