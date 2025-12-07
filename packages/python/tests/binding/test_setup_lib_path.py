"""Tests to improve coverage for kreuzberg/_setup_lib_path.py."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path
from typing import TYPE_CHECKING
from unittest.mock import Mock, patch

from kreuzberg._setup_lib_path import (
    _fix_macos_install_names,
    _setup_linux_paths,
    _setup_macos_paths,
    _setup_windows_paths,
    setup_library_paths,
)

if TYPE_CHECKING:
    import pytest


def test_setup_library_paths_routes_to_macos(monkeypatch: pytest.MonkeyPatch) -> None:
    package_calls: list[Path] = []

    monkeypatch.setattr("platform.system", lambda: "Darwin")
    monkeypatch.setattr("kreuzberg._setup_lib_path._setup_macos_paths", lambda path: package_calls.append(path))
    monkeypatch.setattr("kreuzberg._setup_lib_path._fix_macos_install_names", lambda path: package_calls.append(path))

    setup_library_paths()
    assert package_calls


def test_setup_library_paths_routes_to_linux(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr("platform.system", lambda: "Linux")
    called: list[Path] = []
    monkeypatch.setattr("kreuzberg._setup_lib_path._setup_linux_paths", lambda path: called.append(path))

    setup_library_paths()
    assert called


def test_setup_library_paths_routes_to_windows(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr("platform.system", lambda: "Windows")
    called: list[Path] = []
    monkeypatch.setattr("kreuzberg._setup_lib_path._setup_windows_paths", lambda path: called.append(path))

    setup_library_paths()
    assert called


def test_setup_macos_paths_adds_to_empty_dyld_path() -> None:
    """Test macOS path setup when DYLD_LIBRARY_PATH is empty."""
    package_dir = Path("/fake/package")

    with patch.dict(os.environ, {"DYLD_LIBRARY_PATH": "", "DYLD_FALLBACK_LIBRARY_PATH": ""}, clear=False):
        _setup_macos_paths(package_dir)

        assert os.environ["DYLD_LIBRARY_PATH"] == str(package_dir)
        assert f"{package_dir}:/usr/local/lib:/usr/lib" in os.environ["DYLD_FALLBACK_LIBRARY_PATH"]


def test_setup_macos_paths_appends_to_existing_dyld_path() -> None:
    """Test macOS path setup when DYLD_LIBRARY_PATH exists."""
    package_dir = Path("/fake/package")
    existing_path = "/existing/path"

    with patch.dict(os.environ, {"DYLD_LIBRARY_PATH": existing_path, "DYLD_FALLBACK_LIBRARY_PATH": ""}, clear=False):
        _setup_macos_paths(package_dir)

        assert os.environ["DYLD_LIBRARY_PATH"] == f"{package_dir}:{existing_path}"


def test_setup_macos_paths_appends_to_existing_fallback_path() -> None:
    """Test macOS fallback path setup when it exists."""
    package_dir = Path("/fake/package")
    existing_fallback = "/existing/fallback"

    with patch.dict(
        os.environ, {"DYLD_LIBRARY_PATH": "", "DYLD_FALLBACK_LIBRARY_PATH": existing_fallback}, clear=False
    ):
        _setup_macos_paths(package_dir)

        assert os.environ["DYLD_FALLBACK_LIBRARY_PATH"] == f"{package_dir}:{existing_fallback}"


def test_setup_macos_paths_skips_if_already_present() -> None:
    """Test macOS path setup skips when package dir already in path."""
    package_dir = Path("/fake/package")
    existing_path = f"/other/path:{package_dir}:/another/path"

    with patch.dict(os.environ, {"DYLD_LIBRARY_PATH": existing_path}, clear=False):
        original = os.environ["DYLD_LIBRARY_PATH"]
        _setup_macos_paths(package_dir)

        assert os.environ["DYLD_LIBRARY_PATH"] == original


def test_setup_macos_paths_skips_existing_fallback_entry() -> None:
    """Ensure existing fallback entry is not duplicated."""
    package_dir = Path("/fake/package")
    existing_fallback = f"/usr/local/lib:{package_dir}"

    with patch.dict(
        os.environ,
        {"DYLD_LIBRARY_PATH": "", "DYLD_FALLBACK_LIBRARY_PATH": existing_fallback},
        clear=False,
    ):
        _setup_macos_paths(package_dir)

        assert os.environ["DYLD_FALLBACK_LIBRARY_PATH"] == existing_fallback


def test_setup_linux_paths_adds_to_empty_ld_path(tmp_path: Path) -> None:
    """Test Linux path setup when LD_LIBRARY_PATH is empty."""
    package_dir = tmp_path / "package"
    package_dir.mkdir()

    pdfium_lib = package_dir / "libpdfium.so"
    pdfium_lib.write_text("")

    with (
        patch.dict(os.environ, {"LD_LIBRARY_PATH": ""}, clear=False),
        patch("ctypes.CDLL") as mock_cdll,
    ):
        _setup_linux_paths(package_dir)

        assert os.environ["LD_LIBRARY_PATH"] == str(package_dir)
        assert mock_cdll.call_args_list[-1] == ((str(pdfium_lib),),)


def test_setup_linux_paths_appends_to_existing_ld_path() -> None:
    """Test Linux path setup when LD_LIBRARY_PATH exists."""
    package_dir = Path("/fake/package")
    existing_path = "/existing/path"

    with (
        patch.dict(os.environ, {"LD_LIBRARY_PATH": existing_path}, clear=False),
        patch("pathlib.Path.exists", return_value=False),
    ):
        _setup_linux_paths(package_dir)

        assert os.environ["LD_LIBRARY_PATH"] == f"{package_dir}:{existing_path}"


def test_setup_linux_paths_handles_ctypes_import_error() -> None:
    """Test Linux path setup handles missing ctypes gracefully."""
    package_dir = Path("/fake/package")

    with (
        patch.dict(os.environ, {"LD_LIBRARY_PATH": ""}, clear=False),
        patch("builtins.__import__", side_effect=ImportError("no ctypes")),
    ):
        _setup_linux_paths(package_dir)

        assert os.environ["LD_LIBRARY_PATH"] == str(package_dir)


def test_setup_linux_paths_skips_when_package_already_listed() -> None:
    """LD_LIBRARY_PATH is unchanged if package already present."""
    package_dir = Path("/fake/package")
    existing = f"/opt/lib:{package_dir}"

    with patch.dict(os.environ, {"LD_LIBRARY_PATH": existing}, clear=False):
        _setup_linux_paths(package_dir)

        assert os.environ["LD_LIBRARY_PATH"] == existing


def test_setup_linux_paths_handles_ctypes_oserror(tmp_path: Path) -> None:
    """Test Linux path setup handles OSError from ctypes.CDLL."""
    package_dir = tmp_path / "package"
    package_dir.mkdir()
    pdfium_lib = package_dir / "libpdfium.so"
    pdfium_lib.write_text("")

    with (
        patch.dict(os.environ, {"LD_LIBRARY_PATH": ""}, clear=False),
        patch("ctypes.CDLL", side_effect=OSError("library load failed")),
    ):
        _setup_linux_paths(package_dir)

        assert os.environ["LD_LIBRARY_PATH"] == str(package_dir)


def test_setup_windows_paths_adds_to_empty_path(tmp_path: Path) -> None:
    """Test Windows path setup when PATH is empty."""
    package_dir = tmp_path / "package"
    package_dir.mkdir()

    pdfium_dll = package_dir / "pdfium.dll"
    pdfium_dll.write_text("")

    with (
        patch.dict(os.environ, {"PATH": ""}, clear=False),
        patch("sys.version_info", (3, 8)),
        patch.object(os, "add_dll_directory", create=True) as mock_add_dll,
        patch("ctypes.CDLL") as mock_cdll,
    ):
        _setup_windows_paths(package_dir)

        assert os.environ["PATH"] == str(package_dir)
        mock_add_dll.assert_called_once_with(str(package_dir))
        mock_cdll.assert_called_once_with(str(pdfium_dll))


def test_setup_windows_paths_appends_to_existing_path() -> None:
    """Test Windows path setup when PATH exists."""
    package_dir = Path("C:\\fake\\package")
    existing_path = "C:\\existing\\path"

    with (
        patch.dict(os.environ, {"PATH": existing_path}, clear=False),
        patch("pathlib.Path.exists", return_value=False),
        patch("sys.version_info", (3, 8)),
        patch.object(os, "add_dll_directory", create=True),
    ):
        _setup_windows_paths(package_dir)

        assert os.environ["PATH"] == f"{package_dir};{existing_path}"


def test_setup_windows_paths_handles_add_dll_directory_error() -> None:
    """Test Windows path setup handles add_dll_directory errors."""
    package_dir = Path("C:\\fake\\package")

    with (
        patch.dict(os.environ, {"PATH": ""}, clear=False),
        patch("sys.version_info", (3, 8)),
        patch.object(os, "add_dll_directory", create=True, side_effect=OSError("failed")),
    ):
        _setup_windows_paths(package_dir)

        assert os.environ["PATH"] == str(package_dir)


def test_setup_windows_paths_skips_add_dll_directory_on_old_python() -> None:
    """Test Windows path setup skips add_dll_directory on Python < 3.8."""
    package_dir = Path("C:\\fake\\package")

    with (
        patch.dict(os.environ, {"PATH": ""}, clear=False),
        patch("sys.version_info", (3, 7)),
        patch("pathlib.Path.exists", return_value=False),
    ):
        _setup_windows_paths(package_dir)

        assert os.environ["PATH"] == str(package_dir)


def test_setup_windows_paths_handles_ctypes_import_error() -> None:
    """Test Windows path setup handles missing ctypes."""
    package_dir = Path("C:\\fake\\package")

    with (
        patch.dict(os.environ, {"PATH": ""}, clear=False),
        patch("sys.version_info", (3, 8)),
        patch.object(os, "add_dll_directory", create=True),
        patch("builtins.__import__", side_effect=ImportError("no ctypes")),
    ):
        _setup_windows_paths(package_dir)

        assert os.environ["PATH"] == str(package_dir)


def test_setup_windows_paths_skips_when_present() -> None:
    """PATH is unchanged if package directory already listed."""
    package_dir = Path("C:\\fake\\package")
    existing = f"{package_dir};C:\\other"

    with (
        patch.dict(os.environ, {"PATH": existing}, clear=False),
        patch("sys.version_info", (3, 8)),
        patch.object(os, "add_dll_directory", create=True),
    ):
        _setup_windows_paths(package_dir)

        assert os.environ["PATH"] == existing


def test_setup_windows_paths_handles_ctypes_oserror(tmp_path: Path) -> None:
    """Test Windows path setup handles OSError from ctypes.CDLL."""
    package_dir = tmp_path / "package"
    package_dir.mkdir()
    pdfium_dll = package_dir / "pdfium.dll"
    pdfium_dll.write_text("")

    with (
        patch.dict(os.environ, {"PATH": ""}, clear=False),
        patch("sys.version_info", (3, 8)),
        patch.object(os, "add_dll_directory", create=True),
        patch("ctypes.CDLL", side_effect=OSError("DLL load failed")),
    ):
        _setup_windows_paths(package_dir)

        assert os.environ["PATH"] == str(package_dir)


def test_fix_macos_install_names_returns_early_if_files_missing(tmp_path: Path) -> None:
    """Test _fix_macos_install_names returns early if files don't exist."""
    package_dir = tmp_path / "package"
    package_dir.mkdir()

    with patch("subprocess.run") as mock_run:
        _fix_macos_install_names(package_dir)

        mock_run.assert_not_called()


def test_fix_macos_install_names_returns_if_already_fixed(tmp_path: Path) -> None:
    """Test _fix_macos_install_names returns if already using @loader_path."""
    package_dir = tmp_path / "package"
    package_dir.mkdir()

    so_file = package_dir / "_internal_bindings.abi3.so"
    pdfium_lib = package_dir / "libpdfium.dylib"
    so_file.write_text("")
    pdfium_lib.write_text("")

    mock_result = Mock()
    mock_result.stdout = "/usr/lib/libsystem.dylib\n@loader_path/libpdfium.dylib\n"

    with patch("subprocess.run", return_value=mock_result) as mock_run:
        _fix_macos_install_names(package_dir)

        assert mock_run.call_count == 1


def test_fix_macos_install_names_fixes_loader_path(tmp_path: Path) -> None:
    """Test _fix_macos_install_names fixes ./libpdfium.dylib to @loader_path."""
    package_dir = tmp_path / "package"
    package_dir.mkdir()

    so_file = package_dir / "_internal_bindings.abi3.so"
    pdfium_lib = package_dir / "libpdfium.dylib"
    so_file.write_text("")
    pdfium_lib.write_text("")

    otool_result = Mock()
    otool_result.stdout = "/usr/lib/libsystem.dylib\n./libpdfium.dylib\n"

    install_name_result = Mock()

    with patch("subprocess.run", side_effect=[otool_result, install_name_result]) as mock_run:
        _fix_macos_install_names(package_dir)

        assert mock_run.call_count == 2

        install_name_call = mock_run.call_args_list[1]
        args = install_name_call[0][0]
        assert "install_name_tool" in args
        assert "-change" in args
        assert "./libpdfium.dylib" in args
        assert "@loader_path/libpdfium.dylib" in args


def test_fix_macos_install_names_handles_otool_error(tmp_path: Path) -> None:
    """Test _fix_macos_install_names handles otool subprocess errors."""
    package_dir = tmp_path / "package"
    package_dir.mkdir()

    so_file = package_dir / "_ bindings.abi3.so"
    pdfium_lib = package_dir / "libpdfium.dylib"
    so_file.write_text("")
    pdfium_lib.write_text("")

    with patch("subprocess.run", side_effect=subprocess.CalledProcessError(1, "otool")):
        _fix_macos_install_names(package_dir)


def test_fix_macos_install_names_handles_timeout(tmp_path: Path) -> None:
    """Test _fix_macos_install_names handles subprocess timeout."""
    package_dir = tmp_path / "package"
    package_dir.mkdir()

    so_file = package_dir / "_internal_bindings.abi3.so"
    pdfium_lib = package_dir / "libpdfium.dylib"
    so_file.write_text("")
    pdfium_lib.write_text("")

    with patch("subprocess.run", side_effect=subprocess.TimeoutExpired("otool", 5)):
        _fix_macos_install_names(package_dir)


def test_fix_macos_install_names_handles_install_name_tool_error(tmp_path: Path) -> None:
    """Test _fix_macos_install_names handles install_name_tool errors."""
    package_dir = tmp_path / "package"
    package_dir.mkdir()

    so_file = package_dir / "_internal_bindings.abi3.so"
    pdfium_lib = package_dir / "libpdfium.dylib"
    so_file.write_text("")
    pdfium_lib.write_text("")

    otool_result = Mock()
    otool_result.stdout = "./libpdfium.dylib\n"

    with patch(
        "subprocess.run",
        side_effect=[otool_result, subprocess.CalledProcessError(1, "install_name_tool")],
    ):
        _fix_macos_install_names(package_dir)


def test_fix_macos_install_names_handles_file_not_found(tmp_path: Path) -> None:
    """Test _fix_macos_install_names handles FileNotFoundError."""
    package_dir = tmp_path / "package"
    package_dir.mkdir()

    so_file = package_dir / "_internal_bindings.abi3.so"
    pdfium_lib = package_dir / "libpdfium.dylib"
    so_file.write_text("")
    pdfium_lib.write_text("")

    with patch("subprocess.run", side_effect=FileNotFoundError("install_name_tool not found")):
        _fix_macos_install_names(package_dir)
