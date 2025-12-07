"""Codesign native binaries inside macOS wheels."""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


def run(cmd: list[str]) -> None:
    subprocess.run(cmd, check=True)


def codesign(binary: Path, identity: str) -> None:
    run(["codesign", "--force", "--timestamp=none", "--sign", identity, str(binary)])


def unpack_wheel(wheel_path: Path, dest: Path) -> Path:
    run([sys.executable, "-m", "wheel", "unpack", str(wheel_path), "-d", str(dest)])
    candidates = [p for p in dest.iterdir() if p.is_dir()]
    if len(candidates) != 1:
        raise RuntimeError(f"Expected exactly one unpacked directory for {wheel_path.name}, found {len(candidates)}")
    return candidates[0]


def repack_wheel(pkg_dir: Path, wheel_dir: Path, original_name: Path) -> None:
    before = {p.name for p in wheel_dir.glob("*.whl")}
    run([sys.executable, "-m", "wheel", "pack", str(pkg_dir), "-d", str(wheel_dir)])
    after = {p.name for p in wheel_dir.glob("*.whl")}
    created = list(after - before)
    if len(created) != 1:
        raise RuntimeError(f"Unable to determine repacked wheel name for {original_name.name}")
    new_path = wheel_dir / created[0]
    if new_path.name != original_name.name:
        new_path.rename(original_name)


def sign_wheel(wheel_path: Path, identity: str) -> None:
    print(f"Signing binaries inside {wheel_path.name}")
    wheel_dir = wheel_path.parent
    with tempfile.TemporaryDirectory() as temp_dir_str:
        temp_dir = Path(temp_dir_str)
        unpacked_dir = unpack_wheel(wheel_path, temp_dir)
        package_root = unpacked_dir / "kreuzberg"

        targets = list(package_root.glob("_internal_bindings*.so"))
        targets.append(package_root / "libpdfium.dylib")

        for target in targets:
            if target.exists():
                print(f"  codesign {target.relative_to(unpacked_dir)}")
                codesign(target, identity)

        backup = wheel_path.with_suffix(wheel_path.suffix + ".unsigned")
        wheel_path.rename(backup)
        try:
            repack_wheel(unpacked_dir, wheel_dir, wheel_path)
            backup.unlink()
        except Exception:
            if wheel_path.exists():
                wheel_path.unlink()
            shutil.move(str(backup), str(wheel_path))
            raise


def main() -> None:
    parser = argparse.ArgumentParser(description="Codesign macOS wheel binaries")
    parser.add_argument(
        "--wheel-dir",
        default="target/wheels",
        help="Directory containing .whl files (default: target/wheels)",
    )
    parser.add_argument(
        "--identity",
        default=None,
        help="Codesign identity to use (defaults to MACOS_CODESIGN_IDENTITY or '-')",
    )
    args = parser.parse_args()

    wheel_dir = Path(args.wheel_dir).resolve()
    wheel_files = sorted(wheel_dir.glob("*.whl"))
    if not wheel_files:
        raise SystemExit(f"No wheels found in {wheel_dir}")

    identity = args.identity or os.environ.get("MACOS_CODESIGN_IDENTITY") or os.environ.get("KREUZBERG_CODESIGN_IDENTITY") or "-"

    for wheel_path in wheel_files:
        sign_wheel(wheel_path, identity)


if __name__ == "__main__":
    main()
