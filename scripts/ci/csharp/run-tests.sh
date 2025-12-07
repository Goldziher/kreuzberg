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

# Validate REPO_ROOT is correct by checking for Cargo.toml
if [ ! -f "$REPO_ROOT/Cargo.toml" ]; then
	echo "Error: REPO_ROOT validation failed. Expected Cargo.toml at: $REPO_ROOT/Cargo.toml" >&2
	echo "REPO_ROOT resolved to: $REPO_ROOT" >&2
	exit 1
fi

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

# Verify TESSDATA_PREFIX
if [ -z "${TESSDATA_PREFIX:-}" ]; then
	echo "Warning: TESSDATA_PREFIX not set, sourcing setup script"
	# shellcheck disable=SC1091 # file is in-repo but shellcheck cannot resolve the relative include
	# shellcheck source=scripts/ci/csharp/setup-tessdata.sh
	source "$SCRIPT_DIR/setup-tessdata.sh"
fi

echo "=== Running C# tests ==="
echo "FFI directory: $KREUZBERG_FFI_DIR"
echo "Tesseract version: $(tesseract --version 2>&1 | head -1)"
echo "TESSDATA_PREFIX: ${TESSDATA_PREFIX}"
find "${TESSDATA_PREFIX}/" -maxdepth 1 -name "*.traineddata" 2>&1 | head -5 || echo "Warning: No tessdata files found"

cd "$REPO_ROOT/packages/csharp"
dotnet test Kreuzberg.Tests/Kreuzberg.Tests.csproj -c Release

echo "C# tests complete"
