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

# Resolve workspace root to find native libraries
WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
LIB_DIR="$WORKSPACE_ROOT/target/release"

# Verify native libraries exist before building
if [ ! -d "$LIB_DIR" ]; then
  echo "::error::Native library directory not found at $LIB_DIR" >&2
  exit 1
fi

# Set library search paths so the Ruby native extension can find libpdfium and other dependencies
export LD_LIBRARY_PATH="${LIB_DIR}:${LD_LIBRARY_PATH:-}"
export DYLD_LIBRARY_PATH="${LIB_DIR}:${DYLD_LIBRARY_PATH:-}"

echo "Ruby gem build environment:"
echo "  WORKSPACE_ROOT: $WORKSPACE_ROOT"
echo "  LIB_DIR: $LIB_DIR"
echo "  LD_LIBRARY_PATH: $LD_LIBRARY_PATH"
echo "  DYLD_LIBRARY_PATH: $DYLD_LIBRARY_PATH"
echo ""

cd "$WORKSPACE_ROOT/packages/ruby"
echo "Building Ruby native gem in: $(pwd)"

# Install Ruby dependencies
echo "Installing Ruby dependencies..."
bundle install

bundle exec rake clean
bundle exec rake compile
bundle exec rake build
