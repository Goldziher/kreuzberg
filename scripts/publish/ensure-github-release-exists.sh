#!/usr/bin/env bash

# Ensure GitHub release exists, creating if necessary
#
# Environment Variables:
#   - GH_TOKEN: GitHub API token (required for gh command)
#
# Arguments:
#   $1: Release tag (e.g., v4.0.0-rc.1)

set -euo pipefail

tag="${1:?Release tag argument required}"

if ! gh release view "$tag" >/dev/null 2>&1; then
  gh release create "$tag" --title "$tag" --generate-notes
  echo "Created release $tag"
fi
