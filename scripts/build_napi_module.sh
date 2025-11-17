#!/usr/bin/env bash
set -euo pipefail

TARGET="${TARGET:?TARGET environment variable must be set}"
USE_CROSS="${USE_CROSS:-false}"
USE_NAPI_CROSS="${USE_NAPI_CROSS:-false}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
pushd "$ROOT/crates/kreuzberg-node" >/dev/null
pnpm install
ARGS=(--platform --release --target "$TARGET" --output-dir ./artifacts)
if [[ "$USE_NAPI_CROSS" == "true" ]]; then
  ARGS+=(--use-napi-cross)
fi
if [[ "$USE_CROSS" == "true" ]]; then
  ARGS+=(--use-cross)
fi
pnpm --filter kreuzberg-node exec napi build "${ARGS[@]}"

ARTIFACT_DIR="$ROOT/crates/kreuzberg-node/artifacts"
if [[ ! -d "$ARTIFACT_DIR" ]]; then
  echo "No NAPI artifacts directory found at $ARTIFACT_DIR" >&2
  exit 1
fi

shopt -s nullglob
NODE_OUTPUTS=("$ARTIFACT_DIR"/*.node)
if [[ ${#NODE_OUTPUTS[@]} -eq 0 ]]; then
  echo "No .node artifacts produced in $ARTIFACT_DIR" >&2
  exit 1
fi

rm -f "$ROOT"/crates/kreuzberg-node/*.node
cp "$ARTIFACT_DIR"/*.node "$ROOT"/crates/kreuzberg-node/
popd >/dev/null
