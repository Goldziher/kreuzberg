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
url="https://pypi.org/pypi/kreuzberg/${version}/json"
max_attempts=3
attempt=1
http_code=""

while [ $attempt -le $max_attempts ]; do
  echo "::debug::Checking PyPI for kreuzberg==${version} (attempt ${attempt}/${max_attempts})"

  http_code=$(curl \
    --silent \
    --show-error \
    --retry 3 \
    --retry-delay 5 \
    --connect-timeout 30 \
    --max-time 60 \
    -o /dev/null \
    -w "%{http_code}" \
    "$url" 2>/dev/null || echo "000")

  if [ "$http_code" = "200" ] || [ "$http_code" = "404" ]; then
    break
  fi

  if [ $attempt -lt $max_attempts ]; then
    sleep_time=$((attempt * 5))
    echo "::warning::PyPI check failed (HTTP $http_code), retrying in ${sleep_time}s..."
    sleep "$sleep_time"
  fi

  attempt=$((attempt + 1))
done

if [ "$http_code" = "200" ]; then
  echo "exists=true" >> "$GITHUB_OUTPUT"
  echo "::notice::Python package kreuzberg==${version} already exists on PyPI"
elif [ "$http_code" = "404" ]; then
  echo "exists=false" >> "$GITHUB_OUTPUT"
  echo "::notice::Python package kreuzberg==${version} not found on PyPI, will build and publish"
else
  echo "::error::Failed to check PyPI after $max_attempts attempts (last HTTP code: $http_code)"
  exit 1
fi
