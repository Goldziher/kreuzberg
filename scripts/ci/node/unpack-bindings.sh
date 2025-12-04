#!/usr/bin/env bash
#
# Unpack and install Node bindings from tarball
# Used by: ci-node.yaml - Unpack and install Node bindings step
#

set -euo pipefail

echo "=== Unpacking and installing Node bindings ==="

cd crates/kreuzberg-node

# Extract the tarball to get the built .node file
pkg=$(find . -maxdepth 1 -name "kreuzberg-node-*.tgz" -print | head -n 1)
if [ -z "$pkg" ]; then
    echo "No kreuzberg-node tarball found" >&2
    exit 1
fi

echo "Found package: $pkg"

# Install the package from tarball in the TypeScript workspace
cd ../../packages/typescript
pnpm add "file:../../crates/kreuzberg-node/$pkg"

echo "Installation complete"
