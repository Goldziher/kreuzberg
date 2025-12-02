#!/usr/bin/env bash
#
# Compile Ruby native extension
# Used by: ci-ruby.yaml - Build local native extension step
#

set -euo pipefail

echo "=== Compiling Ruby native extension ==="
cd packages/ruby

# Enable verbose output for debugging
export CARGO_BUILD_JOBS=1
export RUST_BACKTRACE=1

bundle exec rake compile

echo "Compilation complete"
