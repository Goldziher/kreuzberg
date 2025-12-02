#!/usr/bin/env bash
#
# Check if C# NuGet package exists on NuGet registry
#
# Arguments:
#   $1: Package version (required)
#
# Environment variables:
#   - GITHUB_OUTPUT: Path to GitHub Actions output file
#
# Usage:
#   ./check_nuget.sh "1.0.0"
#

set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <version>" >&2
  exit 1
fi

version="$1"
package="Kreuzberg"

# Query NuGet API
url="https://api.nuget.org/v3/registration5-semver1/${package,,}/index.json"
response=$(curl -s "$url")

if echo "$response" | jq -e ".items[].items[]?.catalogEntry | select(.version == \"${version}\")" >/dev/null 2>&1; then
  echo "exists=true" >> "$GITHUB_OUTPUT"
  echo "::notice::NuGet package ${package} ${version} already exists"
else
  echo "exists=false" >> "$GITHUB_OUTPUT"
  echo "::notice::NuGet package ${package} ${version} not found, will build and publish"
fi
