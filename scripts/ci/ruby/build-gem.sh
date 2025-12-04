#!/usr/bin/env bash
#
# Build Ruby gem
# Used by: ci-ruby.yaml - Build Ruby gem step
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

echo "=== Building Ruby gem ==="
cd "$REPO_ROOT/packages/ruby"
bundle exec rake build
echo "Gem build complete"
