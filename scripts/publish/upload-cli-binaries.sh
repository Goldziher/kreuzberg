#!/usr/bin/env bash

# Upload CLI binaries to GitHub Release
#
# Uploads all CLI artifacts matching the pattern cli-* to the specified release.
# Uses gh release upload with --clobber for idempotent uploads.
#
# Environment Variables:
#   - GH_TOKEN: GitHub API token (required for gh command)
#   - ARTIFACTS_DIR: Directory containing CLI artifacts (default: dist/cli)
#
# Arguments:
#   $1: Release tag (e.g., v4.0.0-rc.1)

set -euo pipefail

tag="${1:?Release tag argument required}"
artifacts_dir="${2:-dist/cli}"

if [ ! -d "$artifacts_dir" ]; then
  echo "Error: Artifacts directory not found: $artifacts_dir" >&2
  exit 1
fi

for file in "$artifacts_dir"/kreuzberg-cli-*; do
  if [ -f "$file" ]; then
    gh release upload "$tag" "$file" --clobber
    echo "Uploaded $(basename "$file")"
  fi
done

echo "CLI binaries uploaded to $tag"
