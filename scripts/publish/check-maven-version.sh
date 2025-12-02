#!/usr/bin/env bash

# Check if Java package version exists on Maven Central
#
# Environment Variables:
#   - VERSION: Package version to check (e.g., 4.0.0-rc.1)

set -euo pipefail

version="${1:?VERSION argument required}"
group="dev.kreuzberg"
artifact="kreuzberg"

# Query Maven Central REST API
url="https://search.maven.org/solrsearch/select?q=g:${group}+AND+a:${artifact}+AND+v:${version}&rows=1&wt=json"
response=$(curl -s "$url")

count=$(echo "$response" | jq -r '.response.numFound')
if [ "$count" -gt 0 ]; then
  echo "exists=true"
  echo "::notice::Java package ${group}:${artifact}:${version} already exists on Maven Central"
else
  echo "exists=false"
  echo "::notice::Java package ${group}:${artifact}:${version} not found on Maven Central, will build and publish"
fi
