#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUBY_DIR="${1:-$ROOT/packages/ruby}"
BUNDLE_CMD="${BUNDLE_CMD:-bundle}"

EXTRA_INCLUDE="$ROOT/packages/ruby/ext/kreuzberg_rb/native/include"
UNAME="$(uname -s || echo "")"
if [[ "$UNAME" == MINGW* || "$UNAME" == MSYS* || "$UNAME" == CYGWIN* ]]; then
	EXTRA_INCLUDE="$(cygpath -m "$EXTRA_INCLUDE")"
fi

if [[ -z "${BINDGEN_EXTRA_CLANG_ARGS:-}" ]]; then
	export BINDGEN_EXTRA_CLANG_ARGS="-I$EXTRA_INCLUDE"
else
	export BINDGEN_EXTRA_CLANG_ARGS="$BINDGEN_EXTRA_CLANG_ARGS -I$EXTRA_INCLUDE"
fi

pushd "$RUBY_DIR" >/dev/null
$BUNDLE_CMD exec rake compile
popd >/dev/null
