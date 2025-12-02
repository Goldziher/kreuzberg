#!/usr/bin/env bash
#
# Install built Ruby gem
# Used by: ci-ruby.yaml - Install gem step
#

set -euo pipefail

echo "=== Installing Ruby gem ==="
cd packages/ruby
gem install pkg/kreuzberg-*.gem
echo "Gem installation complete"
