#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PYTHON_DIR="$ROOT/packages/python"
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
popd >/dev/null

cargo build --release --package kreuzberg-cli --features all
