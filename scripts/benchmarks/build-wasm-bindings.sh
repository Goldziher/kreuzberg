#!/usr/bin/env bash
# Builds @kreuzberg/wasm and its WASM artifacts for Node.js benchmarks.
#
# This is a workspace package (pnpm-workspace.yaml includes crates/kreuzberg-wasm),
# so we build it in-place and consume it directly from the workspace.
#
# No required environment variables.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/common.sh"
source "$REPO_ROOT/scripts/lib/library-paths.sh"

validate_repo_root "$REPO_ROOT" || exit 1

# Setup library paths for Rust FFI (used by WASM indirect dependencies)
setup_rust_ffi_paths "$REPO_ROOT"

cd "$REPO_ROOT"

if ! command -v wasm-pack >/dev/null 2>&1; then
	cargo install wasm-pack --locked
fi

rustup target add wasm32-unknown-unknown

pnpm install
pnpm -C crates/kreuzberg-wasm run build
