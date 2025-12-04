#!/usr/bin/env bash

# Publish main Node package to npm
#
# Publishes the main @kreuzberg/node package using pnpm.
# Includes idempotent handling for already-published versions.
#
# Environment Variables:
#   - NODE_AUTH_TOKEN: npm registry authentication token (required)
#
# Arguments:
#   $1: Package directory (default: crates/kreuzberg-node)

set -euo pipefail

pkg_dir="${1:-crates/kreuzberg-node}"

if [ ! -d "$pkg_dir" ]; then
	echo "Error: Package directory not found: $pkg_dir" >&2
	exit 1
fi

cd "$pkg_dir" || exit 1

publish_log=$(mktemp)
set +e
pnpm publish --access public --no-git-checks 2>&1 | tee "$publish_log"
status=${PIPESTATUS[0]}
set -e

if [ "$status" -ne 0 ]; then
	if grep -q "previously published versions" "$publish_log"; then
		echo "::notice::@kreuzberg/node already published; skipping."
	else
		rm -f "$publish_log"
		exit "$status"
	fi
fi

rm -f "$publish_log"
echo "@kreuzberg/node published to npm"
