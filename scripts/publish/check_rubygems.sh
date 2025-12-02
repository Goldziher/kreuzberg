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
max_attempts=3
attempt=1
gem_found=false

while [ $attempt -le $max_attempts ]; do
  echo "::debug::Checking RubyGems for kreuzberg ${version} (attempt ${attempt}/${max_attempts})"

  if gem search kreuzberg --remote --exact --version "=${version}" 2>/dev/null | grep -q "kreuzberg (${version})"; then
    gem_found=true
    break
  elif [ $attempt -lt $max_attempts ]; then
    sleep_time=$((attempt * 5))
    echo "::warning::RubyGems check failed, retrying in ${sleep_time}s..."
    sleep "$sleep_time"
  fi

  attempt=$((attempt + 1))
done

if [ "$gem_found" = true ]; then
  echo "exists=true" >> "$GITHUB_OUTPUT"
  echo "::notice::Ruby gem kreuzberg ${version} already exists on RubyGems"
else
  echo "exists=false" >> "$GITHUB_OUTPUT"
  echo "::notice::Ruby gem kreuzberg ${version} not found on RubyGems, will build and publish"
fi
