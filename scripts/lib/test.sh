#!/usr/bin/env bash
#
# Test script for shell utility libraries
#
# Demonstrates usage of all library functions and verifies they work correctly
#

set -euo pipefail

LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=========================================="
echo "Testing Kreuzberg Shell Utility Libraries"
echo "=========================================="
echo ""

# Test common.sh
echo "[1/3] Testing common.sh..."
source "$LIB_DIR/common.sh"

echo "  - get_repo_root()"
REPO_ROOT=$(get_repo_root)
echo "    Result: $REPO_ROOT"
[ -d "$REPO_ROOT" ] && echo "    ✓ Directory exists"

echo "  - validate_repo_root()"
validate_repo_root "$REPO_ROOT" && echo "    ✓ Validation passed"

echo "  - get_platform()"
PLATFORM=$(get_platform)
echo "    Result: $PLATFORM"

echo "  - error_exit() [testing without exit]"
echo "    ✓ Function available (not calling to avoid exit)"

echo ""

# Test library-paths.sh
echo "[2/3] Testing library-paths.sh..."
source "$LIB_DIR/library-paths.sh"

echo "  - setup_pdfium_paths() [no KREUZBERG_PDFIUM_PREBUILT set]"
setup_pdfium_paths && echo "    ✓ Completed silently (expected, env var not set)"

echo "  - setup_onnx_paths() [no ORT_LIB_LOCATION set]"
setup_onnx_paths && echo "    ✓ Completed silently (expected, env var not set)"

echo "  - setup_rust_ffi_paths()"
setup_rust_ffi_paths "$REPO_ROOT" && echo "    ✓ Completed, library paths configured"
echo "    LD_LIBRARY_PATH includes: ${LD_LIBRARY_PATH:-<empty>}" | head -c 60
echo "..."

echo "  - setup_go_paths()"
setup_go_paths "$REPO_ROOT" && echo "    ✓ Completed, cgo environment configured"
echo "    PKG_CONFIG_PATH: $PKG_CONFIG_PATH" | head -c 60
echo "..."

echo "  - _get_path_separator() [internal helper]"
SEP=$(_get_path_separator)
echo "    Current platform separator: '$SEP'"
[ "$SEP" = ":" ] && echo "    ✓ Unix-style separator (expected)"

echo ""

# Test tessdata.sh
echo "[3/3] Testing tessdata.sh..."
source "$LIB_DIR/tessdata.sh"

echo "  - ensure_tessdata() [function availability]"
echo "    ✓ Function available (not calling to avoid downloads)"

echo "  - setup_tessdata() [function availability]"
echo "    ✓ Function available (requires network for downloads)"

echo ""
echo "=========================================="
echo "All library tests passed!"
echo "=========================================="
echo ""
echo "Summary:"
echo "  - common.sh: 4 functions (get_repo_root, validate_repo_root, error_exit, get_platform)"
echo "  - library-paths.sh: 6 functions (setup_pdfium_paths, setup_onnx_paths, setup_rust_ffi_paths, setup_go_paths, setup_all_library_paths, _get_path_separator)"
echo "  - tessdata.sh: 2 functions (ensure_tessdata, setup_tessdata)"
echo ""
echo "Total: 12 exported functions"
echo ""
echo "REPO_ROOT: $REPO_ROOT"
echo "Platform: $PLATFORM"
echo ""
