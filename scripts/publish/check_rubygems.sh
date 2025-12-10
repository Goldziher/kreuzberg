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
package_name="kreuzberg"
url="https://rubygems.org/api/v1/versions/${package_name}.json"
max_attempts=3
attempt=1
http_code=""

while [ $attempt -le $max_attempts ]; do
	echo "::debug::Checking RubyGems for ${package_name}==${version} (attempt ${attempt}/${max_attempts})" >&2

	http_code=$(curl \
		--silent \
		--show-error \
		--retry 3 \
		--retry-delay 5 \
		--connect-timeout 30 \
		--max-time 60 \
		-o /tmp/rubygems-check.json \
		-w "%{http_code}" \
		"$url" 2>/dev/null || echo "000")

	if [ "$http_code" = "200" ] || [ "$http_code" = "404" ]; then
		break
	fi

	if [ $attempt -lt $max_attempts ]; then
		sleep_time=$((attempt * 5))
		echo "::warning::RubyGems check failed (HTTP $http_code), retrying in ${sleep_time}s..." >&2
		sleep "$sleep_time"
	fi

	attempt=$((attempt + 1))
done

if [ "$http_code" = "200" ]; then
	# Check if the specific version exists in the response
	if jq -e ".[] | select(.number == \"${version}\")" /tmp/rubygems-check.json >/dev/null 2>&1; then
		echo "exists=true"
		echo "::notice::Ruby gem ${package_name}==${version} already exists on RubyGems" >&2
	else
		echo "exists=false"
		echo "::notice::Ruby gem ${package_name}==${version} not found on RubyGems, will build and publish" >&2
	fi
elif [ "$http_code" = "404" ]; then
	# Package doesn't exist yet (first publish)
	echo "exists=false"
	echo "::notice::Ruby gem ${package_name} not found on RubyGems (first publish), will build and publish" >&2
else
	echo "::error::Failed to check RubyGems after $max_attempts attempts (last HTTP code: $http_code)"
	exit 1
fi
