#!/usr/bin/env bash

# Upload Go FFI libraries to GitHub Release
#
# Uploads all Go FFI library artifacts (go-ffi-*.tar.gz) to the specified release.
# Uses gh release upload with --clobber for idempotent uploads.
#
# Environment Variables:
#   - GH_TOKEN: GitHub API token (required for gh command)
#
# Arguments:
#   $1: Release tag (e.g., v4.0.0-rc.1)
#   $2: Directory containing Go FFI artifacts (default: dist/go)

set -euo pipefail

tag="${1:?Release tag argument required}"
artifacts_dir="${2:-dist/go}"

if [ ! -d "$artifacts_dir" ]; then
	echo "Error: Artifacts directory not found: $artifacts_dir" >&2
	exit 1
fi

for file in "$artifacts_dir"/go-ffi-*.tar.gz; do
	if [ -f "$file" ]; then
		gh release upload "$tag" "$file" --clobber
		echo "Uploaded $(basename "$file")"
	fi
done

echo "Go FFI libraries uploaded to $tag"
