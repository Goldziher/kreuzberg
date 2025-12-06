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
url="https://api.nuget.org/v3/registration5-semver1/${package,,}/index.json"
max_attempts=3
attempt=1
response=""
package_found=false

while [ $attempt -le $max_attempts ]; do
	echo "::debug::Checking NuGet for ${package} ${version} (attempt ${attempt}/${max_attempts})" >&2

	response=$(curl \
		--silent \
		--show-error \
		--retry 3 \
		--retry-delay 5 \
		--connect-timeout 30 \
		--max-time 60 \
		"$url" 2>/dev/null || echo "")

	if [ -n "$response" ]; then
		if echo "$response" | jq -e ".items[].items[]?.catalogEntry | select(.version == \"${version}\")" >/dev/null 2>&1; then
			package_found=true
		fi
		break
	elif [ $attempt -lt $max_attempts ]; then
		sleep_time=$((attempt * 5))
		echo "::warning::NuGet check failed, retrying in ${sleep_time}s..." >&2
		sleep "$sleep_time"
	fi

	attempt=$((attempt + 1))
done

if [ "$package_found" = true ]; then
	echo "exists=true"
	echo "::notice::NuGet package ${package} ${version} already exists" >&2
else
	echo "exists=false"
	echo "::notice::NuGet package ${package} ${version} not found, will build and publish" >&2
fi
