#!/usr/bin/env bash

set -euo pipefail

rid="${RID:?RID not set}"
rust_target="${RUST_TARGET:?RUST_TARGET not set}"
ffi_name="${FFI_NAME:?FFI_NAME not set}"

out_root="runtimes/${rid}/native"
mkdir -p "$out_root"

ffi_path="target/${rust_target}/release/${ffi_name}"
if [ ! -f "$ffi_path" ]; then
	echo "FFI library missing at $ffi_path" >&2
	ls -la "target/${rust_target}/release" >&2 || true
	exit 1
fi
cp -f "$ffi_path" "$out_root/${ffi_name}"

deps_dir="target/csharp-native/${rid}"
if [ -d "$deps_dir" ]; then
	case "${RUNNER_OS}" in
	Windows)
		find "$deps_dir" -maxdepth 1 -type f -name '*.dll' -exec cp -f '{}' "$out_root/" ';'
		;;
	macOS)
		find "$deps_dir" -maxdepth 1 -type f -name '*.dylib' -exec cp -f '{}' "$out_root/" ';'
		;;
	Linux)
		find "$deps_dir" -maxdepth 1 -type f -name '*.so*' -exec cp -f '{}' "$out_root/" ';'
		;;
	esac
fi

ls -lah "$out_root"
echo "Native assets directory structure:"
find "$out_root" -type f -exec file {} \;
