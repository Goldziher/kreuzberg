"""Prepare Python wheel by downloading and bundling pdfium library for Windows."""

import json
import platform
import shutil
import tarfile
import urllib.request
from pathlib import Path

PDFIUM_BASE_URL = "https://github.com/bblanchon/pdfium-binaries/releases/download"
PDFIUM_RELEASES_API = "https://api.github.com/repos/bblanchon/pdfium-binaries/releases/latest"


def get_latest_pdfium_version() -> str:
    """Fetch the latest pdfium version from GitHub releases."""
    try:
        with urllib.request.urlopen(PDFIUM_RELEASES_API) as response:
            data = json.loads(response.read().decode())
            # Tag format is "chromium/7469" - extract just the number
            tag = data["tag_name"]
            version = tag.split("/")[-1]
            print(f"Latest pdfium version: {version}")
            return version
    except Exception as e:
        # Fallback to known working version
        print(f"Warning: Could not fetch latest version ({e}), using fallback version 7469")
        return "7469"


def download_pdfium_windows() -> None:
    """Download pdfium.dll for Windows and place it in the package."""
    print("Downloading pdfium binaries for Windows...")

    # Get latest version
    version = get_latest_pdfium_version()

    # Determine architecture
    if platform.machine() == "AMD64":
        arch = "x64"
    elif platform.machine() == "ARM64":
        arch = "arm64"
    else:
        print(f"Unsupported Windows architecture: {platform.machine()}")
        return

    # Download URL
    filename = f"pdfium-win-{arch}.tgz"
    url = f"{PDFIUM_BASE_URL}/chromium/{version}/{filename}"

    print(f"Downloading from: {url}")

    # Download to temp location
    temp_tgz = Path("pdfium_temp.tgz")
    urllib.request.urlretrieve(url, temp_tgz)

    # Extract
    temp_dir = Path("pdfium_temp")
    temp_dir.mkdir(exist_ok=True)

    with tarfile.open(temp_tgz, "r:gz") as tar_ref:
        tar_ref.extractall(temp_dir)

    # Find the DLL
    dll_path = temp_dir / "bin" / "pdfium.dll"
    if not dll_path.exists():
        print(f"Error: pdfium.dll not found in extracted archive at {dll_path}")
        return

    # Copy to kreuzberg package directory
    # cibuildwheel runs from packages/python, so check both locations
    possible_paths = [
        Path("kreuzberg"),  # When running from packages/python (cibuildwheel context)
        Path("packages/python/kreuzberg"),  # When running from root
    ]

    package_dir = None
    for path in possible_paths:
        if path.exists() and path.is_dir():
            package_dir = path
            break

    if package_dir is None:
        print(f"Error: Could not find kreuzberg package directory")
        print(f"Tried: {[str(p) for p in possible_paths]}")
        print(f"Current directory: {Path.cwd()}")
        return

    dest_dll = package_dir / "pdfium.dll"
    shutil.copy2(dll_path, dest_dll)
    print(f"Copied pdfium.dll to {dest_dll}")

    # Cleanup
    shutil.rmtree(temp_dir)
    temp_tgz.unlink()

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
