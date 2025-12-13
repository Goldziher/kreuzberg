#!/usr/bin/env bash
#
# Extract and test CLI binary (Unix)
# Used by: ci-rust.yaml - Extract and test CLI (Unix) step
# Arguments: TARGET (e.g., x86_64-unknown-linux-gnu, aarch64-apple-darwin)
#

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
source "${REPO_ROOT}/scripts/lib/library-paths.sh"

TARGET="${1:-}"

if [ -z "$TARGET" ]; then
	echo "Usage: test-cli-unix.sh <target>"
	echo "  target: Rust build target"
	exit 1
fi

echo "=== Testing CLI binary for $TARGET ==="

# Setup library paths
setup_pdfium_paths

# Extract and test
tar xzf "kreuzberg-cli-$TARGET.tar.gz"
chmod +x kreuzberg
./kreuzberg --version
./kreuzberg --help

echo "CLI tests passed!"
