#!/usr/bin/env bash
#
# Build Node NAPI bindings with artifact collection
# Used by: ci-node.yaml - Build Node bindings step
# Arguments: TARGET (e.g., x86_64-unknown-linux-gnu, aarch64-apple-darwin, x86_64-pc-windows-msvc)
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# scripts/ci/node lives three levels below repo root
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

# Validate REPO_ROOT is correct by checking for Cargo.toml
if [ ! -f "$REPO_ROOT/Cargo.toml" ]; then
	echo "Error: REPO_ROOT validation failed. Expected Cargo.toml at: $REPO_ROOT/Cargo.toml" >&2
	echo "REPO_ROOT resolved to: $REPO_ROOT" >&2
	exit 1
fi

TARGET="${1:-}"

if [ -z "$TARGET" ]; then
	echo "Usage: build-napi.sh <target>"
	echo "  target: NAPI build target (e.g., x86_64-unknown-linux-gnu)"
	exit 1
fi

cd "$REPO_ROOT/crates/kreuzberg-node"

echo "=== Building NAPI bindings for $TARGET ==="
pnpm install
pnpm exec napi build --platform --release --target "$TARGET"
pnpm exec napi prepublish -t npm --no-gh-release

echo "=== Building TypeScript outputs ==="
pnpm exec tsup

# Verify dist directory was created with files
if [ ! -d "dist" ]; then
	echo "Error: dist directory not created by tsup" >&2
	exit 1
fi

if [ ! -f "dist/index.js" ]; then
	echo "Error: dist/index.js not found" >&2
	if [ -d "dist" ]; then
		echo "dist directory exists. Contents:"
		find dist/ -maxdepth 1 -type f
	else
		echo "dist directory does not exist"
	fi
	exit 1
fi

echo "Verified dist directory contains:"
find dist/ -maxdepth 1 -type f | head -20

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

echo "=== Verifying tarball contents ==="
pkg_tgz=$(find . -maxdepth 1 -name "kreuzberg-node-*.tgz" -print | head -n1)
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

	# Verify dist/ files are in tarball
	echo "Checking dist/ files in $pkg_tgz:"
	if tar tzf "$pkg_tgz" | grep "^package/dist/" | head -10; then
		echo "Verified: dist/ files are included in tarball"
	else
		echo "Error: No dist/ files found in tarball!" >&2
		echo "Tarball contents:"
		tar tzf "$pkg_tgz" | grep -E "^package/(dist|index|\.d\.ts)" || echo "No TypeScript build outputs in tarball"
		exit 1
	fi
fi

echo "Build complete"
