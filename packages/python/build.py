"""Custom build hook for building the Kreuzberg CLI with all features.

This hook ensures that the kreuzberg-cli binary is built with the 'all' feature
(which includes 'api' and 'mcp') before the wheel is built. This is required for
the serve and mcp commands to be available in the Python package.
"""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path
from typing import Any

try:
    import maturin
except ImportError as exc:  # pragma: no cover - build-time dependency check
    raise ImportError(
        "The maturin build backend is required to package kreuzberg. "
        "Install it via `pip install maturin` and re-run the build."
    ) from exc


def build_cli_binary() -> None:
    """Build the kreuzberg-cli binary with all features and copy it to the package."""
    workspace_root = Path(__file__).resolve().parents[2]
    package_dir = Path(__file__).resolve().parent / "kreuzberg"

    cargo = shutil.which("cargo")
    if cargo is None:
        return

    try:
        subprocess.run(
            [cargo, "build", "-p", "kreuzberg-cli", "--release", "--features", "all"],
            cwd=workspace_root,
            check=True,
            capture_output=True,
        )

        source_binary = workspace_root / "target" / "release" / "kreuzberg"
        dest_binary = package_dir / "kreuzberg-cli"

        if source_binary.exists():
            shutil.copy2(source_binary, dest_binary)
            dest_binary.chmod(0o755)

    except subprocess.CalledProcessError:
        pass


def build_wheel(
    wheel_directory: str,
    config_settings: dict[str, Any] | None = None,
    metadata_directory: str | None = None,
) -> str:
    """Build a wheel, ensuring CLI is built first."""
    build_cli_binary()

    return maturin.build_wheel(wheel_directory, config_settings, metadata_directory)  # type: ignore


def build_sdist(
    sdist_directory: str,
    config_settings: dict[str, Any] | None = None,
) -> str:
    """Build an sdist."""
    build_cli_binary()

    return maturin.build_sdist(sdist_directory, config_settings)  # type: ignore


def build_editable(
    wheel_directory: str,
    config_settings: dict[str, Any] | None = None,
    metadata_directory: str | None = None,
) -> str:
    """Build an editable wheel."""
    build_cli_binary()

    return maturin.build_editable(wheel_directory, config_settings, metadata_directory)  # type: ignore
