#!/usr/bin/env bash
set -euo pipefail

TARGET="${1:-${TARGET:-}}"
if [[ -z "$TARGET" ]]; then
  echo "Usage: $0 <target>" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STAGE="$ROOT/kreuzberg-cli-${TARGET}"
rm -rf "$STAGE"
mkdir -p "$STAGE"
cp "$ROOT/target/${TARGET}/release/kreuzberg" "$STAGE/"
cp "$ROOT/LICENSE" "$STAGE/"
cp "$ROOT/README.md" "$STAGE/"
tar -czf "kreuzberg-cli-${TARGET}.tar.gz" -C "$ROOT" "kreuzberg-cli-${TARGET}"
rm -rf "$STAGE"
