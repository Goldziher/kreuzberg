#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PYTHON_DIR="$ROOT/packages/python"
export PYTHON_DIR
WHEEL_DIR="$ROOT/target/dev-wheels"

# Ensure no stale native extensions are left behind (Windows builds fail if both
# the prebuilt .pyd and freshly built DLL are present).
find "$PYTHON_DIR/kreuzberg" -maxdepth 1 -type f \( \
  -name '_internal_bindings*.so' -o \
  -name '_internal_bindings*.pyd' -o \
  -name '_internal_bindings*.dll' -o \
  -name '_internal_bindings*.dylib' \
\) -delete || true

rm -rf "$WHEEL_DIR"
mkdir -p "$WHEEL_DIR"

pushd "$PYTHON_DIR" >/dev/null
uv build --wheel --out-dir "$WHEEL_DIR"
LATEST_WHEEL="$(ls -t "$WHEEL_DIR"/*.whl | head -n1)"
uv pip install --force-reinstall "$LATEST_WHEEL"

export LATEST_WHEEL
python - <<'PY'
import os
import sys
import zipfile
import pathlib
import shutil

wheel = pathlib.Path(os.environ["LATEST_WHEEL"])
target_dir = pathlib.Path(os.environ.get("PYTHON_TARGET_DIR", "")) or pathlib.Path(
    os.environ["PYTHON_DIR"]
) / "kreuzberg"
target_dir.mkdir(parents=True, exist_ok=True)

with zipfile.ZipFile(wheel) as zf:
    members = [
        name
        for name in zf.namelist()
        if name.startswith("kreuzberg/_internal_bindings") and not name.endswith(".pyi")
    ]
    if not members:
        raise SystemExit(
            f"No _internal_bindings artifacts found inside wheel {wheel!s}"
        )
    for member in members:
        destination = target_dir / pathlib.Path(member).name
        with zf.open(member) as src, destination.open("wb") as dst:
            shutil.copyfileobj(src, dst)
PY
popd >/dev/null

cargo build --release --package kreuzberg-cli --features all
