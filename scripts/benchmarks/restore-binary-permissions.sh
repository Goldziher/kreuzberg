#!/usr/bin/env bash
# Restores executable permissions on benchmark harness binary
# Required environment variables:
#   - BINARY_PATH: Path to benchmark harness binary (default: ./target/release/benchmark-harness)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# Source shared utilities
source "${REPO_ROOT}/scripts/lib/common.sh"

# Validate repository structure
validate_repo_root "$REPO_ROOT" || exit 1

BINARY_PATH="${BINARY_PATH:-$REPO_ROOT/target/release/benchmark-harness}"

if [ ! -f "$BINARY_PATH" ]; then
	echo "::error::Binary not found at $BINARY_PATH" >&2
	exit 1
fi

chmod +x "$BINARY_PATH"
echo "✓ Restored executable permissions on: $BINARY_PATH"
