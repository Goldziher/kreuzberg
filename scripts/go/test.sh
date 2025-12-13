#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

"${REPO_ROOT}/scripts/download_pdfium_runtime.sh"

cd "${REPO_ROOT}/packages/go"

# Set PKG_CONFIG_PATH for development .pc file
export PKG_CONFIG_PATH="${REPO_ROOT}/crates/kreuzberg-ffi:${PKG_CONFIG_PATH:-}"

# Verify pkg-config finds the library
if pkg-config --exists kreuzberg-ffi 2>/dev/null; then
	echo "✓ pkg-config found kreuzberg-ffi $(pkg-config --modversion kreuzberg-ffi 2>/dev/null)"
else
	echo "⚠ pkg-config not found, using fallback"
fi

# Runtime library paths (still needed for dynamic linking)
export LD_LIBRARY_PATH="${REPO_ROOT}/target/release:${LD_LIBRARY_PATH:-}"
export DYLD_FALLBACK_LIBRARY_PATH="${REPO_ROOT}/target/release:${DYLD_FALLBACK_LIBRARY_PATH:-}"

go test ./...
