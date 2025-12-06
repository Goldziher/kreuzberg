#!/bin/bash
# Test Windows Go build in Docker container

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Building Windows test Docker image ==="
docker build -f "$REPO_ROOT/Dockerfile.windows-test" -t kreuzberg-windows-test "$REPO_ROOT"

echo ""
echo "=== Running Windows build test ==="
docker run --rm \
  -v "$REPO_ROOT:/workspace" \
  -w /workspace \
  kreuzberg-windows-test \
  bash -c '
    set -e

    echo "=== Testing minimal Windows build (core library only) ==="
    echo "Note: Full feature builds require native dependencies that are complex to cross-compile"
    echo "This test validates the Windows toolchain and basic FFI compilation"

    # Set cross-compilation environment for Windows target
    export CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc
    export CXX_x86_64_pc_windows_gnu=x86_64-w64-mingw32-g++
    export AR_x86_64_pc_windows_gnu=x86_64-w64-mingw32-ar
    export RANLIB_x86_64_pc_windows_gnu=x86_64-w64-mingw32-ranlib

    # Test 1: Build core Rust library
    echo "Step 1: Building kreuzberg core for Windows..."
    cargo build -p kreuzberg --release --target x86_64-pc-windows-gnu --no-default-features

    echo ""
    echo "Step 2: Building kreuzberg-ffi for Windows (minimal features)..."
    cargo build -p kreuzberg-ffi --release --target x86_64-pc-windows-gnu --no-default-features

    echo ""
    echo "=== Checking FFI library ==="
    ls -lh target/x86_64-pc-windows-gnu/release/libkreuzberg_ffi.a
    file target/x86_64-pc-windows-gnu/release/libkreuzberg_ffi.a

    echo ""
    echo "=== Setting up Go CGO environment ==="
    export CGO_ENABLED=1
    export GOOS=windows
    export GOARCH=amd64
    export CC="x86_64-w64-mingw32-gcc"
    export CXX="x86_64-w64-mingw32-g++"
    export AR="x86_64-w64-mingw32-ar"
    export CGO_CFLAGS="-I$PWD/crates/kreuzberg-ffi -O2"
    export CGO_LDFLAGS="-L$PWD/target/x86_64-pc-windows-gnu/release -lkreuzberg_ffi"

    echo ""
    echo "=== Building Go bindings for Windows ==="
    cd packages/go
    go build -v ./...

    echo ""
    echo "=== Build successful! ==="
    echo "FFI library: $(ls -1 ../../target/x86_64-pc-windows-gnu/release/libkreuzberg_ffi.a)"
    echo "Go packages built successfully"
  '

echo ""
echo "=== Done ==="
