"""Set up dynamic library search paths for bundled native libraries.

This module must be imported before _internal_bindings to ensure pdfium
and other native libraries can be found at runtime without requiring users
to manually set DYLD_LIBRARY_PATH (macOS), LD_LIBRARY_PATH (Linux), or
PATH (Windows).

Additionally, on macOS, this module fixes the library install names if needed
using install_name_tool, ensuring @loader_path is used for relative references.
"""

from __future__ import annotations

import contextlib
import os
import platform
import subprocess
import sys
from pathlib import Path


def setup_library_paths() -> None:
    """Add package directory to dynamic library search path.

    This ensures bundled native libraries (pdfium, etc.) can be found
    at runtime across all platforms.
    """
    # Get the directory containing this module (kreuzberg package directory)
    package_dir = Path(__file__).parent.resolve()

    # Platform-specific setup
    system = platform.system()

    if system == "Darwin":
        # macOS: Fix library install names first, then set paths
        _fix_macos_install_names(package_dir)
        _setup_macos_paths(package_dir)
    elif system == "Linux":
        # Linux: Set LD_LIBRARY_PATH
        _setup_linux_paths(package_dir)
    elif system == "Windows":
        # Windows: Add to PATH and DLL search path
        _setup_windows_paths(package_dir)


def _fix_macos_install_names(package_dir: Path) -> None:
    """Fix library install names on macOS to use @loader_path.

    This ensures the Python extension can find libpdfium.dylib in the same
    directory without requiring DYLD_LIBRARY_PATH to be set.
    """
    so_file = package_dir / "_internal_bindings.abi3.so"
    pdfium_lib = package_dir / "libpdfium.dylib"

    # Only fix if both files exist
    if not so_file.exists() or not pdfium_lib.exists():
        return

    # Check if fix is needed by examining current install name
    try:
        result = subprocess.run(
            ["otool", "-L", str(so_file)],  # noqa: S607
            capture_output=True,
            text=True,
            check=True,
            timeout=5,
        )

        # If library reference is already correct, skip
        if "@loader_path/libpdfium.dylib" in result.stdout:
            return

        # If library reference is wrong (./libpdfium.dylib), fix it
        if "./libpdfium.dylib" in result.stdout:
            # install_name_tool might fail if not installed or no write permissions
            # Fall back to setting DYLD_LIBRARY_PATH
            with contextlib.suppress(subprocess.CalledProcessError, subprocess.TimeoutExpired, FileNotFoundError):
                subprocess.run(
                    [  # noqa: S607
                        "install_name_tool",
                        "-change",
                        "./libpdfium.dylib",
                        "@loader_path/libpdfium.dylib",
                        str(so_file),
                    ],
                    check=True,
                    timeout=5,
                    capture_output=True,
                )
    # otool might not be available - continue with path-based approach
    except (subprocess.CalledProcessError, subprocess.TimeoutExpired, FileNotFoundError):
        pass


def _setup_macos_paths(package_dir: Path) -> None:
    """Set up library paths for macOS."""
    # Add to DYLD_LIBRARY_PATH
    current_path = os.environ.get("DYLD_LIBRARY_PATH", "")
    package_str = str(package_dir)

    if package_str not in current_path:
        if current_path:
            os.environ["DYLD_LIBRARY_PATH"] = f"{package_str}:{current_path}"
        else:
            os.environ["DYLD_LIBRARY_PATH"] = package_str

    # Also set DYLD_FALLBACK_LIBRARY_PATH as a backup
    current_fallback = os.environ.get("DYLD_FALLBACK_LIBRARY_PATH", "")
    if package_str not in current_fallback:
        if current_fallback:
            os.environ["DYLD_FALLBACK_LIBRARY_PATH"] = f"{package_str}:{current_fallback}"
        else:
            # Default fallback path on macOS
            os.environ["DYLD_FALLBACK_LIBRARY_PATH"] = f"{package_str}:/usr/local/lib:/usr/lib"


def _setup_linux_paths(package_dir: Path) -> None:
    """Set up library paths for Linux."""
    # Add to LD_LIBRARY_PATH
    current_path = os.environ.get("LD_LIBRARY_PATH", "")
    package_str = str(package_dir)

    if package_str not in current_path:
        if current_path:
            os.environ["LD_LIBRARY_PATH"] = f"{package_str}:{current_path}"
        else:
            os.environ["LD_LIBRARY_PATH"] = package_str

    # Try to use ctypes to add search path (Python 3.8+)
    try:
        import ctypes  # noqa: PLC0415
        import ctypes.util  # noqa: PLC0415

        # Try to pre-load libpdfium.so
        # Library load might fail, but LD_LIBRARY_PATH is set so pdfium-render should still find it
        lib_path = package_dir / "libpdfium.so"
        if lib_path.exists():
            with contextlib.suppress(OSError):
                ctypes.CDLL(str(lib_path))
    except (ImportError, AttributeError):
        # ctypes not available or CDLL doesn't work
        pass


def _setup_windows_paths(package_dir: Path) -> None:
    """Set up library paths for Windows."""
    package_str = str(package_dir)

    # Add to PATH
    current_path = os.environ.get("PATH", "")
    if package_str not in current_path:
        if current_path:
            os.environ["PATH"] = f"{package_str};{current_path}"
        else:
            os.environ["PATH"] = package_str

    # Use Windows-specific DLL search path API (Python 3.8+)
    # Might fail to add DLL directory, but PATH is set as fallback
    if sys.version_info >= (3, 8) and hasattr(os, "add_dll_directory"):
        with contextlib.suppress(OSError, AttributeError):
            os.add_dll_directory(str(package_dir))

    # Try to pre-load pdfium.dll
    # Library load might fail, but PATH is set as fallback
    try:
        import ctypes  # noqa: PLC0415

        lib_path = package_dir / "pdfium.dll"
        if lib_path.exists():
            with contextlib.suppress(OSError):
                ctypes.CDLL(str(lib_path))
    except (ImportError, AttributeError):
        pass


# Run setup immediately when module is imported
setup_library_paths()
