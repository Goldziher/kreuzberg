#!/usr/bin/env bash

# Push Ruby gems to RubyGems registry
#
# Publishes all gem artifacts matching kreuzberg-*.gem pattern.
# Requires RubyGems credentials to be configured.
# Handles already-published versions idempotently.
#
# Arguments:
#   $1: Directory containing gem files (default: current directory)

set -euo pipefail

artifacts_dir="${1:-.}"

cd "$artifacts_dir" || {
	echo "Error: Cannot change to directory: $artifacts_dir" >&2
	exit 1
}

shopt -s nullglob
mapfile -t gems < <(find . -maxdepth 1 -name 'kreuzberg-*.gem' -print | sort)

if [ ${#gems[@]} -eq 0 ]; then
	echo "No gem artifacts found in $artifacts_dir" >&2
	exit 1
fi

failed_gems=()
for gem in "${gems[@]}"; do
	echo "Pushing ${gem} to RubyGems"
	publish_log=$(mktemp)
	set +e
	gem push "$gem" 2>&1 | tee "$publish_log"
	status=${PIPESTATUS[0]}
	set -e

	if [ "$status" -ne 0 ]; then
		if grep -qE "Repushing of gem versions is not allowed|already been pushed" "$publish_log"; then
			echo "::notice::Gem $gem version already published on RubyGems; skipping."
			if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
				echo "Gem $(basename "$gem") already published; skipping." >>"$GITHUB_STEP_SUMMARY"
			fi
		else
			failed_gems+=("$gem")
		fi
	fi

	rm -f "$publish_log"
done

if [ ${#failed_gems[@]} -gt 0 ]; then
	echo "::error::Failed to publish the following gems:" >&2
	for gem in "${failed_gems[@]}"; do
		echo "  - $gem" >&2
	done
	exit 1
fi

if [ -n "${GITHUB_STEP_SUMMARY:-}" ] && [ -n "${RUBYGEMS_VERSION:-}" ]; then
	echo "Successfully published kreuzberg version ${RUBYGEMS_VERSION} to RubyGems" >>"$GITHUB_STEP_SUMMARY"
fi

echo "All gems processed"
