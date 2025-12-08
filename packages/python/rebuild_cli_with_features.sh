#!/bin/bash
# Helper script to rebuild the Kreuzberg CLI with all features enabled
# This is needed for the CLI server tests (serve and mcp commands)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "Building Kreuzberg CLI with all features..."
echo "Workspace: $WORKSPACE_ROOT"

cd "$WORKSPACE_ROOT"

# Build with all features (includes api and mcp)
cargo build -p kreuzberg-cli --features all

echo ""
echo "Build complete! The CLI binary is now available with all features."
echo "You can now run the CLI server tests successfully."
