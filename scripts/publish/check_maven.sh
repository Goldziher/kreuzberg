#!/usr/bin/env bash
#
# Check if Java package exists on Maven Central
#
# Arguments:
#   $1: Package version (required)
#
# Environment variables:
#   - GITHUB_OUTPUT: Path to GitHub Actions output file
#
# Usage:
#   ./check_maven.sh "1.0.0"
#

set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <version>" >&2
  exit 1
fi

version="$1"
group="dev.kreuzberg"
artifact="kreuzberg"
url="https://search.maven.org/solrsearch/select?q=g:${group}+AND+a:${artifact}+AND+v:${version}&rows=1&wt=json"
max_attempts=3
attempt=1
response=""
count=0

while [ $attempt -le $max_attempts ]; do
  echo "::debug::Checking Maven Central for ${group}:${artifact}:${version} (attempt ${attempt}/${max_attempts})" >&2

  response=$(curl \
    --silent \
    --show-error \
    --retry 3 \
    --retry-delay 5 \
    --connect-timeout 30 \
    --max-time 60 \
    "$url" 2>/dev/null || echo "")

  if [ -n "$response" ]; then
    count=$(echo "$response" | jq -r '.response.numFound' 2>/dev/null || echo "0")
    if [ "$count" != "0" ] || [ "$count" = "0" ]; then
      break
    fi
  fi

  if [ $attempt -lt $max_attempts ]; then
    sleep_time=$((attempt * 5))
    echo "::warning::Maven Central check failed, retrying in ${sleep_time}s..." >&2
    sleep "$sleep_time"
  fi

  attempt=$((attempt + 1))
done

if [ "$count" -gt 0 ]; then
  echo "exists=true"
  echo "::notice::Java package ${group}:${artifact}:${version} already exists on Maven Central" >&2
else
  echo "exists=false"
  echo "::notice::Java package ${group}:${artifact}:${version} not found on Maven Central, will build and publish" >&2
fi
