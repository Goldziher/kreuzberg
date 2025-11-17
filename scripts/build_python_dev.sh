#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PYTHON_DIR="$ROOT/packages/python"
WHEEL_DIR="$ROOT/target/dev-wheels"

rm -rf "$WHEEL_DIR"
mkdir -p "$WHEEL_DIR"

pushd "$PYTHON_DIR" >/dev/null
uv build --wheel --out-dir "$WHEEL_DIR"
LATEST_WHEEL="$(ls -t "$WHEEL_DIR"/*.whl | head -n1)"
uv pip install --force-reinstall "$LATEST_WHEEL"
popd >/dev/null

cargo build --release --package kreuzberg-cli --features all
