#!/usr/bin/env bash
#
# Run Go tests with proper library path setup
# Used by: ci-go.yaml - Run Go tests step
# Supports: Unix (Linux/macOS) and Windows (via PowerShell)
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# scripts/ci/go lives three levels below repo root
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

# Validate REPO_ROOT is correct by checking for Cargo.toml
if [ ! -f "$REPO_ROOT/Cargo.toml" ]; then
	echo "Error: REPO_ROOT validation failed. Expected Cargo.toml at: $REPO_ROOT/Cargo.toml" >&2
	echo "REPO_ROOT resolved to: $REPO_ROOT" >&2
	exit 1
fi

cd "$REPO_ROOT/packages/go"

if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" || "$OSTYPE" == "win32" ]]; then
	# Windows path - handled via PowerShell wrapper
	workspace=$(cd ../.. && pwd)
	ffiPathGnu="$workspace/target/x86_64-pc-windows-gnu/release"
	ffiPathRelease="$workspace/target/release"
	export PATH="$ffiPathGnu:$ffiPathRelease:$PATH"
	# Set CGO_LDFLAGS to help linker find the library
	export CGO_LDFLAGS="-L$ffiPathGnu -L$ffiPathRelease"
	# Skip -race on Windows: mingw + static CRT regularly fails to link race runtime
	# -x to print the underlying compile/link commands for debugging toolchain issues
	go test -v -x "${GO_TEST_FLAGS:-}" ./...
else
	# Unix paths (Linux/macOS)
	workspace=$(cd ../.. && pwd)
	ffiPath="$workspace/target/release"
	export LD_LIBRARY_PATH="$ffiPath:${LD_LIBRARY_PATH:-}"
	export DYLD_LIBRARY_PATH="$ffiPath:${DYLD_LIBRARY_PATH:-}"
	export CGO_LDFLAGS="-L$ffiPath"
	export TESSDATA_PREFIX=/usr/share/tesseract-ocr/5/tessdata
	go test -v -race -x "${GO_TEST_FLAGS:-}" ./...
fi
