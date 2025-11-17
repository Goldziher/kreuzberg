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
popd >/dev/null
