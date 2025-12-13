#!/usr/bin/env bash
set -euo pipefail

# Usage: stage-ffi-artifacts.sh <staging-dir> <build-features>
# Example: stage-ffi-artifacts.sh artifact-staging/kreuzberg-ffi default
#
# Stages FFI artifacts (Unix/Linux/macOS) for packaging into distribution tarball.
# Copies compiled libraries, headers, and pkg-config files to staging directory.

STAGING_DIR="${1:-artifact-staging/kreuzberg-ffi}"
BUILD_FEATURES="${2:-default}"

echo "=== Staging FFI artifacts to ${STAGING_DIR} ==="

# Enable nullglob to handle glob patterns safely
shopt -s nullglob

# Copy FFI library (required)
ffi_libs=(target/release/libkreuzberg_ffi.*)
if [ ${#ffi_libs[@]} -eq 0 ]; then
	echo "ERROR: No FFI library found in target/release/" >&2
	exit 1
fi
cp "${ffi_libs[@]}" "${STAGING_DIR}/lib/"
echo "✓ Staged FFI library: ${ffi_libs[*]}"

# Copy PDFium (optional, bundled during build)
pdfium_libs=(target/release/libpdfium.*)
if [ ${#pdfium_libs[@]} -gt 0 ]; then
	cp "${pdfium_libs[@]}" "${STAGING_DIR}/lib/"
	echo "✓ Staged PDFium library: ${pdfium_libs[*]}"
fi

# Copy ONNX Runtime if available (Linux/macOS with default features)
if [ -n "${ORT_LIB_LOCATION:-}" ] && [ "${BUILD_FEATURES}" = "default" ]; then
	ort_libs_so=("${ORT_LIB_LOCATION}"/libonnxruntime*.so)
	ort_libs_dylib=("${ORT_LIB_LOCATION}"/libonnxruntime*.dylib)
	ort_count=$((${#ort_libs_so[@]} + ${#ort_libs_dylib[@]}))
	if [ $ort_count -gt 0 ]; then
		[ ${#ort_libs_so[@]} -gt 0 ] && cp -L "${ort_libs_so[@]}" "${STAGING_DIR}/lib/" 2>/dev/null || true
		[ ${#ort_libs_dylib[@]} -gt 0 ] && cp -L "${ort_libs_dylib[@]}" "${STAGING_DIR}/lib/" 2>/dev/null || true
		echo "✓ Staged ONNX Runtime libraries"
	fi
fi

shopt -u nullglob

# Copy header file
cp crates/kreuzberg-ffi/kreuzberg.h "${STAGING_DIR}/include/"

# Copy pkg-config file (installation variant)
cp crates/kreuzberg-ffi/kreuzberg-ffi-install.pc "${STAGING_DIR}/share/pkgconfig/kreuzberg-ffi.pc"

echo "✓ FFI artifacts staged successfully"
