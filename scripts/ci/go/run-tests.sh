#!/usr/bin/env bash
#
# Run Go tests with proper library path setup
# Used by: ci-go.yaml - Run Go tests step
# Supports: Unix (Linux/macOS) and Windows (via PowerShell)
#

set -euo pipefail

cd packages/go

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
