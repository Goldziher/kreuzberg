#!/usr/bin/env bash
set -euo pipefail

rid="${1:?rid required}"
out="${2:-java-natives/${rid}}"

mkdir -p "$out"

case "$rid" in
windows-x86_64)
	cp -f target/release/kreuzberg_ffi.dll "$out/"
	cp -f target/release/pdfium.dll "$out/"
	shopt -s nullglob
	copied_ort=0
	for dll in target/release/*onnxruntime*.dll target/release/DirectML.dll; do
		if [ -f "$dll" ]; then
			cp -f "$dll" "$out/"
			copied_ort=1
		fi
	done
	if [ "$copied_ort" -ne 1 ]; then
		echo "Missing ONNX Runtime DLLs in target/release (embeddings required)" >&2
		exit 1
	fi
	;;
macos-x86_64 | macos-arm64)
	cp -f target/release/libkreuzberg_ffi.dylib "$out/"
	cp -f target/release/libpdfium.dylib "$out/"
	shopt -s nullglob
	copied_ort=0
	for dylib in target/release/libonnxruntime*.dylib; do
		cp -f "$dylib" "$out/"
		copied_ort=1
	done
	if [ "$copied_ort" -ne 1 ]; then
		echo "Missing ONNX Runtime dylibs in target/release (embeddings required)" >&2
		exit 1
	fi
	;;
linux-x86_64)
	cp -f target/release/libkreuzberg_ffi.so "$out/"
	cp -f target/release/libpdfium.so "$out/"
	shopt -s nullglob
	copied_ort=0
	for so in target/release/libonnxruntime*.so*; do
		cp -f "$so" "$out/"
		copied_ort=1
	done
	if [ "$copied_ort" -ne 1 ]; then
		echo "Missing ONNX Runtime .so files in target/release (embeddings required)" >&2
		exit 1
	fi
	;;
*)
	echo "Unsupported rid: $rid" >&2
	exit 1
	;;
esac

ls -la "$out"
