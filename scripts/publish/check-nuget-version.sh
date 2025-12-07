#!/usr/bin/env bash

# Check if C# package version exists on NuGet
#
# Environment Variables:
#   - VERSION: Package version to check (e.g., 4.0.0-rc.1)

set -euo pipefail

version="${1:?VERSION argument required}"
package="Kreuzberg"

# Query NuGet API
url="https://api.nuget.org/v3/registration5-semver1/${package,,}/index.json"
response=$(curl -s "$url")

if echo "$response" | jq -e ".items[].items[]?.catalogEntry | select(.version == \"${version}\")" >/dev/null 2>&1; then
	echo "exists=true"
	echo "::notice::NuGet package ${package} ${version} already exists"
else
	echo "exists=false"
	echo "::notice::NuGet package ${package} ${version} not found, will build and publish"
fi
