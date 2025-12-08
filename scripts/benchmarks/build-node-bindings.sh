#!/usr/bin/env bash
# Builds Node.js native bindings and installs them locally
# Required environment variables:
#   - TARGET: Rust target triple (e.g., x86_64-unknown-linux-gnu)
# No optional environment variables

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

TARGET="${TARGET:-}"

if [ -z "$TARGET" ]; then
	echo "::error::TARGET environment variable is required" >&2
	exit 1
fi

cd "$REPO_ROOT/crates/kreuzberg-node"
pnpm install --no-optional
# Build native bindings with napi (passing TARGET for Rust compilation)
pnpm exec napi build --platform --release --target "${TARGET}"
# Build TypeScript separately (no TARGET needed for tsup/esbuild)
pnpm run build:ts
pkg=$(pnpm pack | tail -n1 | tr -d '\r')
cd "$REPO_ROOT"
pnpm add --workspace-root "file:crates/kreuzberg-node/${pkg}"
