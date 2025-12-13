#!/usr/bin/env bash

# Check if Node package version exists on npm
#
# Environment Variables:
#   - VERSION: Package version to check (e.g., 4.0.0-rc.1)

set -euo pipefail

version="${1:?VERSION argument required}"
package="${2:-@kreuzberg/node}"

# npm view returns non-zero if version doesn't exist
if npm view "${package}@${version}" version >/dev/null 2>&1; then
	echo "exists=true"
	echo "::notice::Node package ${package}@${version} already exists on npm"
else
	echo "exists=false"
	echo "::notice::Node package ${package}@${version} not found on npm, will build and publish"
fi
