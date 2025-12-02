#!/usr/bin/env bash
#
# Install Ruby dependencies via bundle
# Used by: ci-ruby.yaml - Install Ruby deps step (Unix)
#

set -euo pipefail

echo "=== Installing Ruby dependencies ==="
cd packages/ruby

bundle config set deployment false
bundle config set path vendor/bundle
bundle install --jobs 4

echo "Ruby dependencies installed"
