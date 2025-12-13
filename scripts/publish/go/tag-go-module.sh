#!/usr/bin/env bash

# Tag Go module for proper Go proxy server discovery
#
# For Go modules in subdirectories, the Go proxy expects tags with the
# module path prefix. This script creates a lightweight annotated tag
# for the Go module at packages/go/kreuzberg.
#
# Convention: packages/go/v<version>
#
# Arguments:
#   $1: Release tag (e.g., v4.0.0-rc.7)
#
# Environment:
#   GIT_AUTHOR_NAME: Author name (for annotated tag)
#   GIT_AUTHOR_EMAIL: Author email (for annotated tag)

set -euo pipefail

tag="${1:?Release tag argument required}"

# Extract version from tag (remove 'v' prefix if present)
version="${tag#v}"

# Module path prefix for Go module tagging
module_prefix="packages/go"
module_tag="${module_prefix}/v${version}"

echo "Creating Go module tag: $module_tag"

# Create lightweight tag (simpler, no author info needed)
# This points to the same commit as the main release tag
git tag "$module_tag" "$tag"

echo "✅ Go module tag created: $module_tag"
echo "   This enables: go get github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg@v${version}"
