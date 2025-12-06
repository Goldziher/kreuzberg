#!/usr/bin/env bash
#
# Compile Ruby native extension
# Used by: ci-ruby.yaml - Build local native extension step
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# scripts/ci/ruby lives three levels below repo root
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

echo "=== Compiling Ruby native extension ==="
cd "$REPO_ROOT/packages/ruby"

# Enable verbose output for debugging
export CARGO_BUILD_JOBS=1
export RUST_BACKTRACE=1

# Always vendor core to ensure fresh copy for native extension build
"$REPO_ROOT/scripts/ci/ruby/vendor-kreuzberg-core.sh"

bundle exec rake compile

echo "Compilation complete"
