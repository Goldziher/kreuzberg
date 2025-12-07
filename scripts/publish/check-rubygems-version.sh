#!/usr/bin/env bash

# Check if Ruby gem version exists on RubyGems
#
# Environment Variables:
#   - VERSION: Package version to check (e.g., 4.0.0-rc.1)

set -euo pipefail

version="${1:?VERSION argument required}"

# gem search returns versions that exist
if gem search kreuzberg --remote --exact --version "=${version}" | grep -q "kreuzberg (${version})"; then
	echo "exists=true"
	echo "::notice::Ruby gem kreuzberg ${version} already exists on RubyGems"
else
	echo "exists=false"
	echo "::notice::Ruby gem kreuzberg ${version} not found on RubyGems, will build and publish"
fi
