#!/usr/bin/env bash
# Builds Go bindings for benchmarks
# No required environment variables
# Sets up CGO library paths for native FFI bindings

set -euo pipefail

# Resolve workspace root to find native libraries
WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
LIB_DIR="$WORKSPACE_ROOT/target/release"

# Verify native libraries exist before building
if [ ! -d "$LIB_DIR" ]; then
	echo "::error::Native library directory not found at $LIB_DIR" >&2
	exit 1
fi

# Set library search paths so CGO can find native libraries
export LD_LIBRARY_PATH="${LIB_DIR}:${LD_LIBRARY_PATH:-}"
export DYLD_LIBRARY_PATH="${LIB_DIR}:${DYLD_LIBRARY_PATH:-}"
export CGO_ENABLED=1

echo "Go bindings build environment:"
echo "  WORKSPACE_ROOT: $WORKSPACE_ROOT"
echo "  LIB_DIR: $LIB_DIR"
echo "  LD_LIBRARY_PATH: $LD_LIBRARY_PATH"
echo "  DYLD_LIBRARY_PATH: $DYLD_LIBRARY_PATH"
echo ""

cd "$WORKSPACE_ROOT/packages/go"
echo "Building Go bindings in: $(pwd)"

# Build Go bindings (creates compiled artifacts for benchmarks)
go build -v ./...

echo "Go bindings build complete"
