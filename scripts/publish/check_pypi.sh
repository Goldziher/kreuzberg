#!/usr/bin/env bash
#
# Check if Python package version exists on PyPI
#
# Arguments:
#   $1: Package version (required)
#
# Environment variables:
#   - GITHUB_OUTPUT: Path to GitHub Actions output file
#
# Usage:
#   ./check_pypi.sh "1.0.0"
#

set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <version>" >&2
  exit 1
fi

version="$1"

# Check if package version exists on PyPI
http_code=$(curl -s -o /dev/null -w "%{http_code}" \
  "https://pypi.org/pypi/kreuzberg/${version}/json")

if [ "$http_code" = "200" ]; then
  echo "exists=true" >> "$GITHUB_OUTPUT"
  echo "::notice::Python package kreuzberg==${version} already exists on PyPI"
else
  echo "exists=false" >> "$GITHUB_OUTPUT"
  echo "::notice::Python package kreuzberg==${version} not found on PyPI, will build and publish"
fi
