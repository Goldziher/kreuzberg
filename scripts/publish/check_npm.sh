#!/usr/bin/env bash
#
# Check if Node package exists on npm registry
#
# Arguments:
#   $1: Package version (required)
#
# Environment variables:
#   - GITHUB_OUTPUT: Path to GitHub Actions output file
#
# Usage:
#   ./check_npm.sh "1.0.0"
#

set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <version>" >&2
  exit 1
fi

version="$1"
package="@kreuzberg/node"

# npm view returns non-zero if version doesn't exist
if npm view "${package}@${version}" version >/dev/null 2>&1; then
  echo "exists=true" >> "$GITHUB_OUTPUT"
  echo "::notice::Node package ${package}@${version} already exists on npm"
else
  echo "exists=false" >> "$GITHUB_OUTPUT"
  echo "::notice::Node package ${package}@${version} not found on npm, will build and publish"
fi
