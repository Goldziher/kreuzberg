#!/usr/bin/env bash
#
# Install built Ruby gem
# Used by: ci-ruby.yaml - Install gem step
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# scripts/ci/ruby lives three levels below repo root
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

echo "=== Installing Ruby gem ==="
cd "$REPO_ROOT/packages/ruby"
gem install pkg/kreuzberg-*.gem
echo "Gem installation complete"
