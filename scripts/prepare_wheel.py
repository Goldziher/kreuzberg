"""Prepare Python wheel by downloading and bundling pdfium library for Windows."""

import json
import platform
import shutil
import tarfile
import urllib.request
from contextlib import suppress
from pathlib import Path

PDFIUM_BASE_URL = "https://github.com/bblanchon/pdfium-binaries/releases/download"
PDFIUM_RELEASES_API = "https://api.github.com/repos/bblanchon/pdfium-binaries/releases/latest"


def get_latest_pdfium_version() -> str:
    """Fetch the latest pdfium version from GitHub releases."""
    with suppress(Exception), urllib.request.urlopen(PDFIUM_RELEASES_API) as response:  # noqa: S310
        data = json.loads(response.read().decode())
        tag = data["tag_name"]
        return tag.split("/")[-1]
    return "7469"


def download_pdfium_windows() -> None:
    """Download pdfium.dll for Windows and place it in the package."""
    version = get_latest_pdfium_version()

    if platform.machine() == "AMD64":
        arch = "x64"
    elif platform.machine() == "ARM64":
        arch = "arm64"
    else:
        return

    filename = f"pdfium-win-{arch}.tgz"
    url = f"{PDFIUM_BASE_URL}/chromium/{version}/{filename}"

    temp_tgz = Path("pdfium_temp.tgz")
    urllib.request.urlretrieve(url, temp_tgz)  # noqa: S310

    temp_dir = Path("pdfium_temp")
    temp_dir.mkdir(exist_ok=True)

    with tarfile.open(temp_tgz, "r:gz") as tar_ref:
        tar_ref.extractall(temp_dir)  # noqa: S202

    dll_path = temp_dir / "bin" / "pdfium.dll"
    if not dll_path.exists():
        return

    possible_paths = [
        Path("kreuzberg"),
        Path("packages/python/kreuzberg"),
    ]

    package_dir = None
    for path in possible_paths:
        if path.exists() and path.is_dir():
            package_dir = path
            break

    if package_dir is None:
        return

    dest_dll = package_dir / "pdfium.dll"
    shutil.copy2(dll_path, dest_dll)

    shutil.rmtree(temp_dir)
    temp_tgz.unlink()


def main() -> None:
    """Main entry point."""
    if platform.system() == "Windows":
        download_pdfium_windows()
    else:
        pass


if __name__ == "__main__":
    main()
