#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUBY_DIR="${1:-$ROOT/packages/ruby}"
BUNDLE_CMD="${BUNDLE_CMD:-bundle}"
SKIP_BUNDLE_PATH="${SKIP_BUNDLE_PATH:-false}"

pushd "$RUBY_DIR" >/dev/null
if [[ "$SKIP_BUNDLE_PATH" != "true" ]]; then
	$BUNDLE_CMD config set --local path 'vendor/bundle'
fi
$BUNDLE_CMD install --jobs 4 --retry 3
popd >/dev/null
