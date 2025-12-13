#!/usr/bin/env bash
set -euo pipefail

# Usage: verify-ffi-artifact.sh <artifact-tarball>
# Example: verify-ffi-artifact.sh "go-ffi-linux-amd64.tar.gz"
#
# Verifies that FFI distribution tarball contains all required files.
# Checks for header files, pkg-config files, and library structure.

ARTIFACT="${1}"

if [ ! -f "${ARTIFACT}" ]; then
	echo "✗ Artifact not found: ${ARTIFACT}"
	exit 1
fi

echo "=== Verifying artifact structure ==="
tar -tzf "${ARTIFACT}"

# Cleanup handler
cleanup() {
	rm -rf verify-temp
}
trap cleanup EXIT

mkdir -p verify-temp
tar -xzf "${ARTIFACT}" -C verify-temp

# Check for required files
REQUIRED_FILES=(
	"kreuzberg-ffi/include/kreuzberg.h"
	"kreuzberg-ffi/share/pkgconfig/kreuzberg-ffi.pc"
)

echo ""
echo "=== Checking required files ==="
for file in "${REQUIRED_FILES[@]}"; do
	if [ -f "verify-temp/$file" ]; then
		echo "✓ Found: $file"
	else
		echo "✗ Missing: $file"
		exit 1
	fi
done

# Platform-specific library checks
echo ""
echo "=== Checking platform-specific libraries ==="
PLATFORM_LIBS_FOUND=0

# Check for Linux libraries (.so)
if find verify-temp/kreuzberg-ffi/lib -name "*.so" -o -name "*.so.*" | grep -q .; then
	LIBKREUZBERG=$(find verify-temp/kreuzberg-ffi/lib -name "libkreuzberg_ffi.so*" | head -1)
	if [ -n "$LIBKREUZBERG" ]; then
		echo "✓ Found Linux library: $(basename "$LIBKREUZBERG")"
		PLATFORM_LIBS_FOUND=1
	fi
fi

# Check for macOS libraries (.dylib)
if find verify-temp/kreuzberg-ffi/lib -name "*.dylib" | grep -q .; then
	LIBKREUZBERG=$(find verify-temp/kreuzberg-ffi/lib -name "libkreuzberg_ffi.dylib" | head -1)
	if [ -n "$LIBKREUZBERG" ]; then
		echo "✓ Found macOS library: $(basename "$LIBKREUZBERG")"
		PLATFORM_LIBS_FOUND=1
	fi
fi

# Check for Windows libraries (.dll)
if find verify-temp/kreuzberg-ffi/lib -name "*.dll" | grep -q .; then
	LIBKREUZBERG=$(find verify-temp/kreuzberg-ffi/lib -name "kreuzberg_ffi.dll" | head -1)
	if [ -n "$LIBKREUZBERG" ]; then
		echo "✓ Found Windows library: $(basename "$LIBKREUZBERG")"
		PLATFORM_LIBS_FOUND=1
	fi
fi

if [ $PLATFORM_LIBS_FOUND -eq 0 ]; then
	echo "✗ No platform libraries found (expected .so, .dylib, or .dll)"
	exit 1
fi

echo ""
echo "✓ Artifact verification passed"
