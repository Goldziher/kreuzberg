#!/usr/bin/env bash
#
# Run C# tests
# Used by: ci-csharp.yaml - Run C# tests step
# Requires: KREUZBERG_FFI_DIR environment variable
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# scripts/ci/csharp lives three levels below repo root
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/common.sh"
source "$REPO_ROOT/scripts/lib/library-paths.sh"
source "$REPO_ROOT/scripts/lib/tessdata.sh"

validate_repo_root "$REPO_ROOT" || exit 1

if [ -z "${KREUZBERG_FFI_DIR:-}" ]; then
	echo "Error: KREUZBERG_FFI_DIR environment variable not set"
	exit 1
fi

# Ensure tesseract binary is available
if ! command -v tesseract &>/dev/null; then
	echo "Error: tesseract binary not found in PATH"
	echo "PATH: $PATH"
	exit 1
fi

# Setup Rust FFI and Tesseract paths
setup_rust_ffi_paths "$REPO_ROOT"
setup_tessdata

echo "=== Running C# tests ==="
echo "FFI directory: $KREUZBERG_FFI_DIR"
echo "Tesseract version: $(tesseract --version 2>&1 | head -1)"
echo "TESSDATA_PREFIX: ${TESSDATA_PREFIX}"
find "${TESSDATA_PREFIX}/" -maxdepth 1 -name "*.traineddata" 2>&1 | head -5 || echo "Warning: No tessdata files found"

cd "$REPO_ROOT/packages/csharp"
dotnet test Kreuzberg.Tests/Kreuzberg.Tests.csproj -c Release

echo "C# tests complete"
