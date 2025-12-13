#!/usr/bin/env bash
#
# Run Ruby tests
# Used by: ci-ruby.yaml - Run Ruby tests step
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# scripts/ci/ruby lives three levels below repo root
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/common.sh"
source "$REPO_ROOT/scripts/lib/library-paths.sh"

validate_repo_root "$REPO_ROOT" || exit 1
setup_rust_ffi_paths "$REPO_ROOT"

echo "=== Running Ruby tests ==="
cd "$REPO_ROOT/packages/ruby"
bundle exec rspec
echo "Tests complete"
