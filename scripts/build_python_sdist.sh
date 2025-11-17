#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_ARG="${1:-$ROOT/dist}"
if [[ "$OUT_ARG" = /* ]]; then
  OUT_DIR="$OUT_ARG"
else
  OUT_DIR="$ROOT/$OUT_ARG"
fi

mkdir -p "$OUT_DIR"

pushd "$ROOT/packages/python" >/dev/null
uv build --sdist --out-dir "$OUT_DIR"
popd >/dev/null
