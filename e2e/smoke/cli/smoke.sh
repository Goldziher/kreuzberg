#!/usr/bin/env bash
set -euo pipefail

if [[ -z "${KREUZBERG_CLI_BINARY:-}" ]]; then
  echo "KREUZBERG_CLI_BINARY env var is required" >&2
  exit 1
fi

BINARY="$KREUZBERG_CLI_BINARY"
if [[ ! -x "$BINARY" ]]; then
  echo "CLI binary not executable: $BINARY" >&2
  exit 1
fi

$BINARY --version

FIXTURE="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/fixtures/report.txt"

OUTPUT=$(mktemp)
trap 'rm -f "$OUTPUT"' EXIT

$BINARY extract "$FIXTURE" --format text > "$OUTPUT"
if ! grep -qi "smoke" "$OUTPUT"; then
  echo "Smoke test failed: snippet missing" >&2
  exit 1
fi

echo "[cli smoke] extraction succeeded"
