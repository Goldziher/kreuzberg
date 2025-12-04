#!/usr/bin/env bash
set -euo pipefail

TARGET="${1:-${TARGET:-}}"
if [[ -z "$TARGET" ]]; then
	echo "Usage: $0 <target>" >&2
	exit 1
fi

cargo build --release --target "$TARGET" --package kreuzberg-cli
