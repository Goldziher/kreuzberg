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

# Query Maven Central REST API
url="https://search.maven.org/solrsearch/select?q=g:${group}+AND+a:${artifact}+AND+v:${version}&rows=1&wt=json"
response=$(curl -s "$url")

count=$(echo "$response" | jq -r '.response.numFound')
if [ "$count" -gt 0 ]; then
  echo "exists=true" >> "$GITHUB_OUTPUT"
  echo "::notice::Java package ${group}:${artifact}:${version} already exists on Maven Central"
else
  echo "exists=false" >> "$GITHUB_OUTPUT"
  echo "::notice::Java package ${group}:${artifact}:${version} not found on Maven Central, will build and publish"
fi
