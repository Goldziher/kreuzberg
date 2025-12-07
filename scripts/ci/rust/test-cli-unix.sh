#!/usr/bin/env bash
#
# Extract and test CLI binary (Unix)
# Used by: ci-rust.yaml - Extract and test CLI (Unix) step
# Arguments: TARGET (e.g., x86_64-unknown-linux-gnu, aarch64-apple-darwin)
#

set -euo pipefail

TARGET="${1:-}"

if [ -z "$TARGET" ]; then
	echo "Usage: test-cli-unix.sh <target>"
	echo "  target: Rust build target"
	exit 1
fi

echo "=== Testing CLI binary for $TARGET ==="

# Setup library paths
if [ -n "${KREUZBERG_PDFIUM_PREBUILT:-}" ]; then
	if [ "$RUNNER_OS" = "macOS" ]; then
		export DYLD_LIBRARY_PATH="$KREUZBERG_PDFIUM_PREBUILT/lib:${DYLD_LIBRARY_PATH:-}"
	else
		export LD_LIBRARY_PATH="$KREUZBERG_PDFIUM_PREBUILT/lib:${LD_LIBRARY_PATH:-}"
	fi
fi

# Extract and test
tar xzf "kreuzberg-cli-$TARGET.tar.gz"
chmod +x kreuzberg
./kreuzberg --version
./kreuzberg --help

echo "CLI tests passed!"
