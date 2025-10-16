"""Runtime library loader for bundled Pdfium library."""

import contextlib
import ctypes
import os
import platform
from pathlib import Path


def get_lib_path() -> Path:
    """Get the path to the bundled Pdfium library."""
    lib_dir = Path(__file__).parent

    system = platform.system()
    if system == "Darwin":
        lib_name = "libpdfium.dylib"
    elif system == "Windows":
        lib_name = "libpdfium.dll"
    else:  # Linux
        lib_name = "libpdfium.so"

    lib_path = lib_dir / lib_name
    if not lib_path.exists():
        msg = f"Pdfium library not found at {lib_path}. Please reinstall kreuzberg."
        raise FileNotFoundError(msg)

    return lib_path


def load_pdfium() -> None:
    """Load the bundled Pdfium library.

    This should be called before any PDF operations that use pdfium-render.
    The library is loaded using RTLD_GLOBAL so it's available to the Rust code.
    """
    lib_path = get_lib_path()

    # Load with RTLD_GLOBAL so Rust code can find it
    if platform.system() == "Windows":
        # Windows uses LoadLibrary
        ctypes.cdll.LoadLibrary(str(lib_path))
    else:
        # Unix-like systems use RTLD_GLOBAL so the library symbols are available globally
        mode = ctypes.RTLD_GLOBAL
        # Add RTLD_LAZY if available (lazy binding for better performance)
        if hasattr(ctypes, "RTLD_LAZY"):
            mode |= ctypes.RTLD_LAZY
        ctypes.CDLL(str(lib_path), mode=mode)


# Optionally auto-load on import (can be disabled if needed)
_AUTO_LOAD = os.environ.get("KREUZBERG_AUTO_LOAD_PDFIUM", "1") == "1"

if _AUTO_LOAD:
    # Don't fail import if library loading fails - user can still use non-PDF features
    with contextlib.suppress(Exception):
        load_pdfium()
