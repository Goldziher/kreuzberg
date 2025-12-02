#!/usr/bin/env bash
# Builds Ruby native gem using bundler rake tasks
# Required environment variables:
#   - PLATFORM: Ruby platform identifier (e.g., x86_64-linux)

set -euo pipefail

PLATFORM="${PLATFORM:-}"

if [ -z "$PLATFORM" ]; then
  echo "::error::PLATFORM environment variable is required" >&2
  exit 1
fi

cd packages/ruby
bundle exec rake clean
bundle exec rake compile
bundle exec rake build
