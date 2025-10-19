"""Prepare Python wheel by downloading and bundling pdfium library for Windows."""

import platform
import shutil
import sys
import urllib.request
import zipfile
from pathlib import Path

# pdfium-binaries version to download
PDFIUM_VERSION = "6721"
PDFIUM_BASE_URL = "https://github.com/bblanchon/pdfium-binaries/releases/download"


def download_pdfium_windows() -> None:
    """Download pdfium.dll for Windows and place it in the package."""
    print("Downloading pdfium binaries for Windows...")

    # Determine architecture
    if platform.machine() == "AMD64":
        arch = "x64"
    elif platform.machine() == "ARM64":
        arch = "arm64"
    else:
        print(f"Unsupported Windows architecture: {platform.machine()}")
        return

    # Download URL
    filename = f"pdfium-win-{arch}.zip"
    url = f"{PDFIUM_BASE_URL}/chromium%2F{PDFIUM_VERSION}/{filename}"

    print(f"Downloading from: {url}")

    # Download to temp location
    temp_zip = Path("pdfium_temp.zip")
    urllib.request.urlretrieve(url, temp_zip)

    # Extract
    temp_dir = Path("pdfium_temp")
    temp_dir.mkdir(exist_ok=True)

    with zipfile.ZipFile(temp_zip, "r") as zip_ref:
        zip_ref.extractall(temp_dir)

    # Find the DLL
    dll_path = temp_dir / "bin" / "pdfium.dll"
    if not dll_path.exists():
        print(f"Error: pdfium.dll not found in extracted archive at {dll_path}")
        return

    # Copy to kreuzberg package directory
    package_dir = Path("packages/python/kreuzberg")
    if not package_dir.exists():
        package_dir = Path("kreuzberg")  # When running from packages/python

    if not package_dir.exists():
        print(f"Error: Could not find kreuzberg package directory")
        return

    dest_dll = package_dir / "pdfium.dll"
    shutil.copy2(dll_path, dest_dll)
    print(f"Copied pdfium.dll to {dest_dll}")

    # Cleanup
    shutil.rmtree(temp_dir)
    temp_zip.unlink()

    print("✓ pdfium.dll downloaded and bundled successfully")


def main() -> None:
    """Main entry point."""
    print(f"Platform: {platform.system()}")
    print(f"Machine: {platform.machine()}")

    if platform.system() == "Windows":
        download_pdfium_windows()
    else:
        print("pdfium.dll bundling only needed on Windows, skipping...")


if __name__ == "__main__":
    main()
