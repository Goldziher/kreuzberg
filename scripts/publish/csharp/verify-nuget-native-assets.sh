#!/usr/bin/env bash

set -euo pipefail

pkg="$(find artifacts/csharp -maxdepth 1 -name '*.nupkg' -print | sort | head -n 1)"
if [ -z "$pkg" ]; then
	echo "No .nupkg found under artifacts/csharp" >&2
	exit 1
fi

for rid in linux-x64 linux-arm64 osx-x64 osx-arm64 win-x64 win-arm64; do
	unzip -l "$pkg" | rg -n "^\\s+\\d+\\s+\\d{4}-\\d{2}-\\d{2}\\s+\\d{2}:\\d{2}\\s+runtimes/${rid}/native/.*kreuzberg_ffi\\.(dll|so|dylib)$" >/dev/null
	unzip -l "$pkg" | rg -n "^\\s+\\d+\\s+\\d{4}-\\d{2}-\\d{2}\\s+\\d{2}:\\d{2}\\s+runtimes/${rid}/native/(lib)?pdfium\\.(dll|so|dylib)$" >/dev/null
	unzip -l "$pkg" | rg -n "^\\s+\\d+\\s+\\d{4}-\\d{2}-\\d{2}\\s+\\d{2}:\\d{2}\\s+runtimes/${rid}/native/.*onnxruntime" >/dev/null
done
