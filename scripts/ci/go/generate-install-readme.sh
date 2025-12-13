#!/usr/bin/env bash
set -euo pipefail

# Usage: generate-install-readme.sh <output-file>
# Example: generate-install-readme.sh artifact-staging/kreuzberg-ffi/README.md
#
# Generates installation instructions README for FFI distribution tarball.

OUTPUT_FILE="${1:-artifact-staging/kreuzberg-ffi/README.md}"

cat >"${OUTPUT_FILE}" <<'EOF'
# Kreuzberg FFI Installation Guide

## System-wide installation (requires sudo):
```bash
tar -xzf go-ffi-*.tar.gz
cd kreuzberg-ffi
sudo cp -r lib/* /usr/local/lib/
sudo cp -r include/* /usr/local/include/
sudo cp -r share/* /usr/local/share/
sudo ldconfig  # Linux only
```

## User-local installation:
```bash
tar -xzf go-ffi-*.tar.gz
cd kreuzberg-ffi
cp -r {lib,include,share} ~/.local/
export PKG_CONFIG_PATH="$HOME/.local/share/pkgconfig:$PKG_CONFIG_PATH"
export LD_LIBRARY_PATH="$HOME/.local/lib:$LD_LIBRARY_PATH"  # Linux
export DYLD_FALLBACK_LIBRARY_PATH="$HOME/.local/lib:$DYLD_FALLBACK_LIBRARY_PATH"  # macOS
```

## Using with Go:
```bash
pkg-config --modversion kreuzberg-ffi  # Verify installation
go get github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg@latest
```
EOF

echo "✓ Installation README generated: ${OUTPUT_FILE}"
