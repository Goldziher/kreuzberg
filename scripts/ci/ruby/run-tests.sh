#!/usr/bin/env bash
#
# Run Ruby tests
# Used by: ci-ruby.yaml - Run Ruby tests step
#

set -euo pipefail

echo "=== Running Ruby tests ==="
cd packages/ruby
bundle exec rspec
echo "Tests complete"
