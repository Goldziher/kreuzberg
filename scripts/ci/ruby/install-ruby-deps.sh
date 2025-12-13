#!/usr/bin/env bash
#
# Install Ruby dependencies via bundle
# Used by: ci-ruby.yaml - Install Ruby deps step (Unix)
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/common.sh"

validate_repo_root "$REPO_ROOT" || exit 1

echo "=== Installing Ruby dependencies ==="
cd "$REPO_ROOT/packages/ruby"

bundle config set deployment false
bundle config set path vendor/bundle
bundle install --jobs 4

echo "Ruby dependencies installed"
