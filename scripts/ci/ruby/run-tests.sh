#!/usr/bin/env bash
#
# Run Ruby tests
# Used by: ci-ruby.yaml - Run Ruby tests step
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

echo "=== Running Ruby tests ==="
cd "$REPO_ROOT/packages/ruby"
bundle exec rspec
echo "Tests complete"
