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

echo "=========================================="
echo "Go Test Configuration and Diagnostics"
echo "=========================================="
echo "Script directory: $SCRIPT_DIR"
echo "Repository root: $REPO_ROOT"
echo "Operating system: $OSTYPE"
echo "Go version: $(go version)"
echo "Go environment:"
go env
echo

cd "$REPO_ROOT/packages/go"
echo "Working directory: $(pwd)"
echo

# Build list of Go packages that actually contain source files (skip an empty module root)
echo "Discovering Go packages (excluding empty roots)..."
mapfile -t go_dirs < <(find . -name '*.go' -not -path './vendor/*' -print0 | xargs -0 -n1 dirname | sort -u)
packages=()
for dir in "${go_dirs[@]}"; do
	# Skip the module root if it contains no Go sources
	if [[ "$dir" == "." ]]; then
		continue
	fi
	packages+=("./${dir#./}")
done

if [[ ${#packages[@]} -eq 0 ]]; then
	echo "No Go packages found; nothing to test."
	exit 0
fi

if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" || "$OSTYPE" == "win32" ]]; then
	# Windows path - handled via PowerShell wrapper
	workspace=$(cd ../.. && pwd)
	ffiPathGnu="$workspace/target/x86_64-pc-windows-gnu/release"
	ffiPathRelease="$workspace/target/release"
	export PATH="$ffiPathGnu:$ffiPathRelease:$PATH"
	# Set CGO_LDFLAGS to help linker find the library
	export CGO_LDFLAGS="-L$ffiPathGnu -L$ffiPathRelease"

	echo "=========================================="
	echo "Windows-specific Configuration"
	echo "=========================================="
	echo "Workspace root: $workspace"
	echo "FFI path (GNU): $ffiPathGnu"
	echo "FFI path (Release): $ffiPathRelease"
	echo "PATH: $PATH"
	echo "CGO_LDFLAGS: $CGO_LDFLAGS"
	echo "CGO_CFLAGS: ${CGO_CFLAGS:-<not set>}"
	echo "CC: ${CC:-<not set>}"
	echo "CXX: ${CXX:-<not set>}"
	echo

	echo "=== FFI Library Files ==="
	if [ -d "$ffiPathGnu" ]; then
		echo "Contents of $ffiPathGnu:"
		find "$ffiPathGnu" -type f \( -name "*.dll" -o -name "*.a" -o -name "*.lib" \) | head -20
	else
		echo "Directory not found: $ffiPathGnu"
	fi
	if [ -d "$ffiPathRelease" ]; then
		echo "Contents of $ffiPathRelease:"
		find "$ffiPathRelease" -type f \( -name "*.dll" -o -name "*.a" -o -name "*.lib" \) | head -20
	else
		echo "Directory not found: $ffiPathRelease"
	fi
	echo

	# Skip -race on Windows: mingw + static CRT regularly fails to link race runtime
	# -x to print the underlying compile/link commands for debugging toolchain issues
	echo "Running Go tests with verbose output and compile command tracing..."
	go test -v -x "${GO_TEST_FLAGS:-}" "${packages[@]}"
else
	# Unix paths (Linux/macOS)
	workspace=$(cd ../.. && pwd)
	ffiPath="$workspace/target/release"
	export LD_LIBRARY_PATH="$ffiPath:${LD_LIBRARY_PATH:-}"
	export DYLD_LIBRARY_PATH="$ffiPath:${DYLD_LIBRARY_PATH:-}"
	export DYLD_FALLBACK_LIBRARY_PATH="$ffiPath:${DYLD_FALLBACK_LIBRARY_PATH:-}"

	# Add rpath for runtime library resolution (critical for macOS with SIP)
	if [[ "$OSTYPE" == "darwin"* ]]; then
		export CGO_LDFLAGS="-L$ffiPath -Wl,-rpath,$ffiPath"
	else
		export CGO_LDFLAGS="-L$ffiPath -Wl,-rpath,$ffiPath"
	fi

	export TESSDATA_PREFIX=/usr/share/tesseract-ocr/5/tessdata

	echo "=========================================="
	echo "Unix-specific Configuration"
	echo "=========================================="
	echo "Workspace root: $workspace"
	echo "FFI library path: $ffiPath"
	echo "LD_LIBRARY_PATH: $LD_LIBRARY_PATH"
	echo "DYLD_LIBRARY_PATH: $DYLD_LIBRARY_PATH"
	echo "DYLD_FALLBACK_LIBRARY_PATH: $DYLD_FALLBACK_LIBRARY_PATH"
	echo "CGO_LDFLAGS: $CGO_LDFLAGS"
	echo "CGO_CFLAGS: ${CGO_CFLAGS:-<not set>}"
	echo "CC: ${CC:-<not set>}"
	echo "CXX: ${CXX:-<not set>}"
	echo "TESSDATA_PREFIX: $TESSDATA_PREFIX"
	echo

	echo "=== FFI Library Files ==="
	if [ -d "$ffiPath" ]; then
		echo "Contents of $ffiPath:"
		find "$ffiPath" -type f \( -name "*.so" -o -name "*.dylib" -o -name "*.a" \) | head -20
		echo
		echo "File details:"
		ls -lh "$ffiPath"/libkreuzberg_ffi.* 2>/dev/null || echo "No libkreuzberg_ffi files found"
	else
		echo "Directory not found: $ffiPath"
	fi
	echo

	echo "=== Library dependencies ==="
	if command -v pkg-config &>/dev/null; then
		echo "pkg-config version:"
		pkg-config --version
	fi
	echo

	# Running Go tests with verbose output and race detector
	# -x prints the underlying compile/link commands for debugging toolchain issues
	# -race enables the race condition detector (not available on Windows)
	echo "Running Go tests with verbose output, race detection, and compile command tracing..."
	go test -v -race -x "${GO_TEST_FLAGS:-}" "${packages[@]}"
fi
