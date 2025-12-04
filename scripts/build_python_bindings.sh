#!/bin/bash
# Build Python bindings with automatic library path fixes
#
# Usage:
#   ./scripts/build_python_bindings.sh [--release|--debug]
#
# This script:
# 1. Runs maturin develop to build the Python extension
# 2. On macOS, fixes library install names to use @loader_path
# 3. Verifies the build was successful

set -e # Exit on error

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/.." && pwd)}"

# Parse arguments
BUILD_TYPE="--release"
if [ "$1" = "--debug" ]; then
	BUILD_TYPE=""
fi

echo "🔨 Building Python bindings${BUILD_TYPE:+ in release mode}..."

# Run maturin develop
cd "$REPO_ROOT/crates/kreuzberg-py"
maturin develop $BUILD_TYPE

# On macOS, fix library install names
if [[ "$OSTYPE" == "darwin"* ]]; then
	echo "🔧 Fixing library install names for macOS..."

	# Find the .so file (it might be in different locations)
	SO_FILE="../../packages/python/kreuzberg/_internal_bindings.abi3.so"

	if [ -f "$SO_FILE" ]; then
		# Check current install name
		if otool -L "$SO_FILE" | grep -q "./libpdfium.dylib"; then
			echo "   Fixing ./libpdfium.dylib → @loader_path/libpdfium.dylib"
			install_name_tool -change ./libpdfium.dylib @loader_path/libpdfium.dylib "$SO_FILE"
			echo "   ✅ Fixed"
		elif otool -L "$SO_FILE" | grep -q "@loader_path/libpdfium.dylib"; then
			echo "   ✅ Already using @loader_path (no fix needed)"
		else
			echo "   ⚠️  Unexpected pdfium reference in binary"
			otool -L "$SO_FILE" | grep pdfium || true
		fi
	else
		echo "   ⚠️  .so file not found at expected location"
	fi
fi

echo ""
echo "✅ Build complete!"
echo ""
echo "To test the build:"
echo "  cd packages/python"
echo "  python -c 'import kreuzberg; print(kreuzberg.__version__)'"
echo ""
