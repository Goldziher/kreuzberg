#!/usr/bin/env bash
#
# Build Node NAPI bindings with artifact collection
# Used by: ci-node.yaml - Build Node bindings step
# Arguments: TARGET (e.g., x86_64-unknown-linux-gnu, aarch64-apple-darwin, x86_64-pc-windows-msvc)
#

set -euo pipefail

TARGET="${1:-}"

if [ -z "$TARGET" ]; then
  echo "Usage: build-napi.sh <target>"
  echo "  target: NAPI build target (e.g., x86_64-unknown-linux-gnu)"
  exit 1
fi

cd crates/kreuzberg-node

echo "=== Building NAPI bindings for $TARGET ==="
pnpm install
pnpm exec napi build --platform --release --target "$TARGET"
pnpm exec napi prepublish -t npm --no-gh-release

mkdir -p artifacts

# Collect artifacts from napi (if produced) and fallback to build outputs
pnpm exec napi artifacts --output-dir ./artifacts || true

shopt -s nullglob globstar
mapfile -t artifacts < <(find artifacts npm target . -maxdepth 6 -type f -name "*.node")

if [ "${#artifacts[@]}" -eq 0 ]; then
    echo "No .node artifacts produced under artifacts/, npm/**/, target/**/release, or workspace root" >&2
    find . -maxdepth 4 -type f -name "*.node" || true
    exit 1
fi

echo "Found ${#artifacts[@]} artifact(s)"
for f in "${artifacts[@]}"; do
    dest="./$(basename "$f")"
    if [ "$f" != "$dest" ]; then
        echo "Copying $f to $dest"
        cp "$f" "$dest"
    fi
done

# Repack tarball with the .node file for specific platforms
pnpm pack

pkg_tgz=$(ls kreuzberg-node-*.tgz | head -n1)
if [[ -n "$pkg_tgz" ]]; then
    case "$TARGET" in
        x86_64-unknown-linux-gnu)
            node_file="kreuzberg-node.linux-x64-gnu.node"
            ;;
        x86_64-pc-windows-msvc)
            node_file="kreuzberg-node.win32-x64-msvc.node"
            ;;
        aarch64-pc-windows-msvc)
            node_file="kreuzberg-node.win32-arm64-msvc.node"
            ;;
        x86_64-apple-darwin)
            node_file="kreuzberg-node.darwin-x64.node"
            ;;
        aarch64-apple-darwin)
            node_file="kreuzberg-node.darwin-arm64.node"
            ;;
        *)
            node_file=""
            ;;
    esac

    if [[ -n "$node_file" && -f "$node_file" ]]; then
        echo "Repacking tarball with $node_file"
        tmpdir=$(mktemp -d)
        tar xzf "$pkg_tgz" -C "$tmpdir"
        cp "$node_file" "$tmpdir/package/"
        tar czf "$pkg_tgz" -C "$tmpdir" package
        rm -rf "$tmpdir"
    fi
fi

echo "Build complete"
