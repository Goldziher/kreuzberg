#!/usr/bin/env bash
#
# Build Ruby gem
# Used by: ci-ruby.yaml - Build Ruby gem step
#

set -euo pipefail

echo "=== Building Ruby gem ==="
cd packages/ruby
bundle exec rake build
echo "Gem build complete"
