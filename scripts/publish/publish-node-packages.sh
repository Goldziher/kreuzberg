#!/usr/bin/env bash

# Publish native binary packages to npm
#
# Publishes all packed npm packages (*.tgz files) from the npm directory.
# Includes idempotent handling for already-published versions.
#
# Environment Variables:
#   - NODE_AUTH_TOKEN: npm registry authentication token (required)
#   - NPM_PACKAGES_DIR: Directory containing packed npm packages (default: crates/kreuzberg-node/npm)

set -euo pipefail

npm_dir="${1:-crates/kreuzberg-node/npm}"

if [ ! -d "$npm_dir" ]; then
	echo "Error: npm directory not found: $npm_dir" >&2
	exit 1
fi

shopt -s nullglob
pkgs=("$npm_dir"/*.tgz)

if [ ${#pkgs[@]} -eq 0 ]; then
	echo "No npm packages found in $npm_dir" >&2
	exit 1
fi

for pkg in "${pkgs[@]}"; do
	echo "Publishing $(basename "$pkg")"
	publish_log=$(mktemp)
	set +e
	npm publish "$pkg" --access public --ignore-scripts 2>&1 | tee "$publish_log"
	status=${PIPESTATUS[0]}
	set -e

	if [ "$status" -ne 0 ]; then
		if grep -q "previously published versions" "$publish_log"; then
			echo "::notice::Package $(basename "$pkg") already published; skipping."
		else
			exit "$status"
		fi
	fi
	rm -f "$publish_log"
done

echo "Native binary packages published"
