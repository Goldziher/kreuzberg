#!/usr/bin/env bash

# Push Ruby gems to RubyGems registry
#
# Publishes all gem artifacts matching kreuzberg-*.gem pattern.
# Requires RubyGems credentials to be configured.
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

for gem in "${gems[@]}"; do
  echo "Pushing ${gem} to RubyGems"
  gem push "$gem"
  echo "Pushed ${gem}"
done

echo "All gems published to RubyGems"
