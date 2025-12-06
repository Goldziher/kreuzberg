#!/usr/bin/env bash
set -euo pipefail

version="${PDFIUM_VERSION:-${1:-7529}}"
root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dest_release="$root_dir/target/release"
dest_debug="$root_dir/target/debug"

platform="$(uname -s)"
case "$platform" in
Linux*) platform_id="linux" ;;
Darwin*) platform_id="mac" ;;
MINGW* | MSYS* | CYGWIN*) platform_id="win" ;;
*)
	echo "Unsupported platform: $platform" >&2
	exit 1
	;;
esac

arch="$(uname -m)"
case "$arch" in
x86_64 | amd64) arch_id="x64" ;;
arm64 | aarch64) arch_id="arm64" ;;
*)
	echo "Unsupported architecture: $arch" >&2
	exit 1
	;;
esac

tmpdir="$(mktemp -d)"
echo "Downloading Pdfium ${version} for ${platform_id}/${arch_id}..."
curl -fsSL -o "$tmpdir/pdfium.tgz" "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/${version}/pdfium-${platform_id}-${arch_id}.tgz"
mkdir -p "$tmpdir/extracted"
tar -xzf "$tmpdir/pdfium.tgz" -C "$tmpdir/extracted"

src_lib="$tmpdir/extracted/lib"
if [[ ! -d "$src_lib" ]]; then
	echo "Pdfium archive did not contain lib directory" >&2
	exit 1
fi

mkdir -p "$dest_release" "$dest_debug"
cp -a "$src_lib/." "$dest_release/"
cp -a "$src_lib/." "$dest_debug/"

echo "Pdfium runtime staged to:"
echo "  $dest_release"
echo "  $dest_debug"

rm -rf "$tmpdir"
