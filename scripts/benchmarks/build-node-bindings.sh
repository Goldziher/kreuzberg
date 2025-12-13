#!/usr/bin/env bash
# Builds Node.js native bindings and installs them locally
# Required environment variables:
#   - TARGET: Rust target triple (e.g., x86_64-unknown-linux-gnu)
# No optional environment variables

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/common.sh"
source "$REPO_ROOT/scripts/lib/library-paths.sh"

validate_repo_root "$REPO_ROOT" || exit 1

TARGET="${TARGET:-}"

if [ -z "$TARGET" ]; then
	echo "::error::TARGET environment variable is required" >&2
	exit 1
fi

# Setup all library paths (PDFium + ONNX + Rust FFI) for Node NAPI build
setup_all_library_paths "$REPO_ROOT"

cd "$REPO_ROOT"
# Install workspace dependencies with full support (including optional deps)
# This is done at workspace root to maintain consistency with shared-workspace-lockfile
pnpm install

cd "$REPO_ROOT/crates/kreuzberg-node"
# Build native bindings with napi (passing TARGET for Rust compilation)
pnpm exec napi build --platform --release --target "${TARGET}"
# Build TypeScript separately (no TARGET needed for tsup/esbuild)
pnpm run build:ts
pkg=$(pnpm pack | tail -n1 | tr -d '\r')
cd "$REPO_ROOT"
pnpm add --workspace-root "file:crates/kreuzberg-node/${pkg}"
