#!/usr/bin/env bash
#
# Unpack and install Node bindings from tarball
# Used by: ci-node.yaml - Unpack and install Node bindings step
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

cd "$REPO_ROOT"

echo "=== Unpacking and installing Node bindings ==="

cd "$REPO_ROOT/crates/kreuzberg-node"

# Extract the tarball to get the built .node file
pkg=$(find . -maxdepth 1 -name "kreuzberg-node-*.tgz" -print | head -n 1)
if [ -z "$pkg" ]; then
	echo "No kreuzberg-node tarball found" >&2
	exit 1
fi

echo "Found package: $pkg"

# Get absolute path to tarball before changing directories
pkg_abs_path="$(cd "$(dirname "$pkg")" && pwd)/$(basename "$pkg")"

# Install the package from tarball in the TypeScript workspace
cd ../../packages/typescript
pnpm add --workspace-root "file:$pkg_abs_path"

echo "Installation complete"
