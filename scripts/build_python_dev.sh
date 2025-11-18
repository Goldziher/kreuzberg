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

uv run python - <<'PY'
import importlib.util
import os
import pathlib
import shutil
import sys

pkg = importlib.util.find_spec("kreuzberg")
if pkg is None or pkg.origin is None:
    raise SystemExit("kreuzberg package not found after wheel installation")

source_dir = pathlib.Path(pkg.origin).parent
target_dir = pathlib.Path(os.environ["PYTHON_DIR"]).joinpath("kreuzberg")
target_dir.mkdir(parents=True, exist_ok=True)

copied = False
for artifact in source_dir.glob("_internal_bindings*"):
    if artifact.suffix in {".py", ".pyi"}:
        continue
    shutil.copyfile(artifact, target_dir / artifact.name)
    copied = True

if not copied:
    raise SystemExit(f"No compiled bindings found in {source_dir}")
PY
popd >/dev/null

cargo build --release --package kreuzberg-cli --features all
