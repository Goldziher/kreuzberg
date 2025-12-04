#!/usr/bin/env bash
# Builds Node.js native bindings and installs them locally
# Required environment variables:
#   - TARGET: Rust target triple (e.g., x86_64-unknown-linux-gnu)
# No optional environment variables

set -euo pipefail

TARGET="${TARGET:-}"

if [ -z "$TARGET" ]; then
	echo "::error::TARGET environment variable is required" >&2
	exit 1
fi

cd crates/kreuzberg-node
pnpm install --no-optional
# Build native bindings with napi (passing TARGET for Rust compilation)
pnpm exec napi build --platform --release --target "${TARGET}"
# Build TypeScript separately (no TARGET needed for tsup/esbuild)
pnpm run build:ts
pkg=$(pnpm pack | tail -n1 | tr -d '\r')
pnpm install -w "crates/kreuzberg-node/${pkg}"
cd ../..
