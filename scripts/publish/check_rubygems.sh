#!/usr/bin/env bash
#
# Check if Ruby gem exists on RubyGems registry
#
# Arguments:
#   $1: Package version (required)
#
# Environment variables:
#   - GITHUB_OUTPUT: Path to GitHub Actions output file
#
# Usage:
#   ./check_rubygems.sh "1.0.0"
#

set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <version>" >&2
  exit 1
fi

version="$1"

# gem search returns versions that exist
if gem search kreuzberg --remote --exact --version "=${version}" | grep -q "kreuzberg (${version})"; then
  echo "exists=true" >> "$GITHUB_OUTPUT"
  echo "::notice::Ruby gem kreuzberg ${version} already exists on RubyGems"
else
  echo "exists=false" >> "$GITHUB_OUTPUT"
  echo "::notice::Ruby gem kreuzberg ${version} not found on RubyGems, will build and publish"
fi
