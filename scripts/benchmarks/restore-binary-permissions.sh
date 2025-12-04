#!/usr/bin/env bash
# Restores executable permissions on benchmark harness binary
# Required environment variables:
#   - BINARY_PATH: Path to benchmark harness binary (default: ./target/release/benchmark-harness)

set -euo pipefail

BINARY_PATH="${BINARY_PATH:-./target/release/benchmark-harness}"

if [ ! -f "$BINARY_PATH" ]; then
	echo "::error::Binary not found at $BINARY_PATH" >&2
	exit 1
fi

chmod +x "$BINARY_PATH"
