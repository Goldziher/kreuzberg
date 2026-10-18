#!/usr/bin/env bash
set -euo pipefail

ort_version="${1:?ort-version required}"
dest_dir="${2:-crates/xberg-node}"
arch_id="${3:-}"
strategy="${4:-system}"
add_macos_rpath="${5-true}"

case "$add_macos_rpath" in
true | false) ;;
*)
  echo "add-macos-rpath must be true or false" >&2
  exit 2
  ;;
esac

extract_dir="$RUNNER_TEMP/onnxruntime"

if [ -z "$arch_id" ]; then
  arch="$(uname -m)"
  if [ "$arch" = "arm64" ]; then
    arch_id="arm64"
  else
    arch_id="x64"
  fi
fi

case "$arch_id" in
arm64) ort_arch="arm64" ;;
x64) ort_arch="x86_64" ;;
*)
  echo "Unsupported macOS arch-id: $arch_id" >&2
  exit 1
  ;;
esac
echo "Using macOS ONNX Runtime arch: $ort_arch"

# Last Microsoft x86_64 macOS release (1.24 dropped the arch). Self-contained
# with a macOS 13.4 floor; ort built with api-18 accepts any runtime >= 1.18. ~keep
if [ "$ort_arch" = "x86_64" ]; then
  ort_version="1.23.2"
fi

ort_root="$extract_dir/onnxruntime-osx-${ort_arch}-${ort_version}"

if [ ! -d "$ort_root" ]; then
  echo "Cache miss: Downloading ONNX Runtime ${ort_version} for macOS ${ort_arch}"
  archive="onnxruntime-osx-${ort_arch}-${ort_version}.tgz"
  mkdir -p "$extract_dir"
  curl -fsSL --retry 5 --retry-delay 5 --retry-all-errors -o "$RUNNER_TEMP/$archive" \
    "https://github.com/microsoft/onnxruntime/releases/download/v${ort_version}/$archive"
  tar -xzf "$RUNNER_TEMP/$archive" -C "$extract_dir"
else
  echo "Cache hit: Using cached ONNX Runtime ${ort_version}"
fi

if [ ! -d "$ort_root/lib" ]; then
  echo "ERROR: ONNX Runtime lib directory missing at $ort_root/lib" >&2
  echo "Available directories:" >&2
  ls -la "$extract_dir" >&2 || true
  exit 1
fi

if ! ls "$ort_root/lib"/libonnxruntime*.dylib 1>/dev/null 2>&1; then
  echo "ERROR: No ONNX Runtime libraries found in $ort_root/lib" >&2
  echo "Directory contents:" >&2
  ls -la "$ort_root/lib" >&2 || true
  exit 1
fi

dest="$GITHUB_WORKSPACE/$dest_dir"
mkdir -p "$dest"
cp -f "$ort_root/lib/"libonnxruntime*.dylib "$dest/"

if [ -n "${RUSTFLAGS:-}" ]; then
  rustflags="$RUSTFLAGS -L $ort_root/lib"
else
  rustflags="-L $ort_root/lib"
fi
if [ "$add_macos_rpath" = "true" ]; then
  rustflags="$rustflags -C link-arg=-Wl,-rpath,@loader_path"
fi

if [ "$strategy" = "bundled" ]; then
  echo "Using bundled ORT strategy — letting ort-sys download-binaries handle static linking"
  {
    echo "DYLD_LIBRARY_PATH=$ort_root/lib:$dest:${DYLD_LIBRARY_PATH:-}"
    echo "DYLD_FALLBACK_LIBRARY_PATH=$ort_root/lib:$dest:${DYLD_FALLBACK_LIBRARY_PATH:-}"
    echo "LIBRARY_PATH=$ort_root/lib:$dest:${LIBRARY_PATH:-}"
  } >>"$GITHUB_ENV"
else
  {
    ort_lib=$(find "$ort_root/lib" -name "libonnxruntime*.dylib" -print -quit)
    echo "ORT_LIB_LOCATION=$ort_root/lib"
    echo "ORT_PREFER_DYNAMIC_LINK=1"
    echo "ORT_SKIP_DOWNLOAD=1"
    echo "ORT_STRATEGY=system"
    echo "ORT_DYLIB_PATH=$ort_root/lib/${ort_lib##*/}"
    echo "DYLD_LIBRARY_PATH=$ort_root/lib:$dest:${DYLD_LIBRARY_PATH:-}"
    echo "DYLD_FALLBACK_LIBRARY_PATH=$ort_root/lib:$dest:${DYLD_FALLBACK_LIBRARY_PATH:-}"
    echo "LIBRARY_PATH=$ort_root/lib:$dest:${LIBRARY_PATH:-}"
    echo "RUSTFLAGS=$rustflags"
  } >>"$GITHUB_ENV"
fi
