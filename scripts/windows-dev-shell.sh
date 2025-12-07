#!/bin/bash
# Launch interactive shell in Windows build container
# This allows iterative testing and debugging with Claude Code

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Building Windows test Docker image ==="
docker build -f "$REPO_ROOT/Dockerfile.windows-test" -t kreuzberg-windows-test "$REPO_ROOT"

echo ""
echo "=== Launching interactive Windows build environment ==="
echo "You are now in a Linux container with MinGW-w64 cross-compilation tools."
echo ""
echo "Available commands:"
echo "  - Rust: rustc, cargo (with x86_64-pc-windows-gnu target)"
echo "  - Go: go build, go test"
echo "  - MinGW: x86_64-w64-mingw32-gcc, x86_64-w64-mingw32-g++"
echo "  - Build tools: cmake, nasm, pkg-config"
echo "  - Testing: wine64 (run Windows .exe files)"
echo ""
echo "Environment is pre-configured for Windows builds:"
echo "  CC=x86_64-w64-mingw32-gcc"
echo "  Target: x86_64-pc-windows-gnu"
echo ""
echo "To test the build:"
echo "  1. cargo build -p kreuzberg-ffi --release --target x86_64-pc-windows-gnu"
echo "  2. cd packages/go && go build -v ./..."
echo ""
echo "Type 'exit' to leave the container."
echo ""

docker run --rm -it \
	-v "$REPO_ROOT:/workspace" \
	-w /workspace \
	kreuzberg-windows-test \
	/bin/bash
