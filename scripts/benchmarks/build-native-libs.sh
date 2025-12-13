#!/usr/bin/env bash
# Builds workspace with all features and benchmark harness in release mode
# No required environment variables

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# Source shared utilities
source "${REPO_ROOT}/scripts/lib/common.sh"
source "${REPO_ROOT}/scripts/lib/library-paths.sh"

# Validate repository structure
validate_repo_root "$REPO_ROOT" || exit 1

# Setup all library paths (PDFium + ONNX + Rust FFI)
setup_all_library_paths "$REPO_ROOT"

echo "Building native libraries in release mode:"
echo "  REPO_ROOT: $REPO_ROOT"
echo "  LD_LIBRARY_PATH: ${LD_LIBRARY_PATH:-<not set>}"
echo "  DYLD_LIBRARY_PATH: ${DYLD_LIBRARY_PATH:-<not set>}"
echo

cd "$REPO_ROOT"
cargo build --workspace --release --all-features
cargo build --manifest-path tools/benchmark-harness/Cargo.toml --release

echo "Native libraries build complete"
