#!/usr/bin/env bash
#
# Clean previous wheel artifacts
# Used by: ci-python.yaml - Clean previous wheel artifacts step
#

set -euo pipefail

echo "=== Cleaning previous wheel artifacts ==="
rm -rf target/wheels target/maturin
rm -f packages/python/kreuzberg/_internal_bindings.*
echo "Cleanup complete"
