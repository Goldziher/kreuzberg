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
	ffiPath="$workspace/target/x86_64-pc-windows-gnu/release"
	export PATH="$ffiPath:$PATH"
	go test -v -race ./...
else
	# Unix paths (Linux/macOS)
	export LD_LIBRARY_PATH="${PWD}/../../target/release:${LD_LIBRARY_PATH:-}"
	export DYLD_LIBRARY_PATH="${PWD}/../../target/release:${DYLD_LIBRARY_PATH:-}"
	export TESSDATA_PREFIX=/usr/share/tesseract-ocr/5/tessdata
	go test -v -race ./...
fi
