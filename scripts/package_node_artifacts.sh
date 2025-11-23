#!/usr/bin/env bash
set -euo pipefail

TARGET="${TARGET:?TARGET environment variable must be set}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
pushd "$ROOT" >/dev/null
pnpm --filter kreuzberg exec napi artifacts --output-dir ./artifacts
if [[ ! -d crates/kreuzberg-node/npm ]]; then
  echo "npm artifact directory missing" >&2
  exit 1
fi
tar -czf "node-bindings-${TARGET}.tar.gz" -C crates/kreuzberg-node npm
popd >/dev/null
