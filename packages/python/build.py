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


def build_cli_binary() -> None:
    """Build the kreuzberg-cli binary with all features and copy it to the package."""
    workspace_root = Path(__file__).resolve().parents[2]
    package_dir = Path(__file__).resolve().parent / "kreuzberg"

    # Check if cargo is available
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

        # Copy the binary to the package directory
        source_binary = workspace_root / "target" / "release" / "kreuzberg"
        dest_binary = package_dir / "kreuzberg-cli"

        if source_binary.exists():
            shutil.copy2(source_binary, dest_binary)
            dest_binary.chmod(0o755)  # Make it executable

    except subprocess.CalledProcessError:
        # Don't raise - allow wheel build to continue, Python fallback will kick in
        pass


def build_wheel(
    wheel_directory: str,
    config_settings: dict[str, Any] | None = None,
    metadata_directory: str | None = None,
) -> str:
    """Build a wheel, ensuring CLI is built first."""
    import maturin

    build_cli_binary()

    return maturin.build_wheel(wheel_directory, config_settings, metadata_directory)  # type: ignore


def build_sdist(
    sdist_directory: str,
    config_settings: dict[str, Any] | None = None,
) -> str:
    """Build an sdist."""
    import maturin

    build_cli_binary()

    return maturin.build_sdist(sdist_directory, config_settings)  # type: ignore


def build_editable(
    wheel_directory: str,
    config_settings: dict[str, Any] | None = None,
    metadata_directory: str | None = None,
) -> str:
    """Build an editable wheel."""
    import maturin

    build_cli_binary()

    return maturin.build_editable(wheel_directory, config_settings, metadata_directory)  # type: ignore
