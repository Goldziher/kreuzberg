#!/usr/bin/env bash

# Rebuild and publish Ruby gems to RubyGems
#
# This script:
# 1. Ensures latest RubyGems is installed
# 2. Unpacks each gem and extracts its gemspec
# 3. Rebuilds the gem to fix tar structure
# 4. Publishes using 'gem push'
#
# Environment Variables:
#   - GEM_ARTIFACTS_DIR: Directory containing gem files (default: .)

set -euo pipefail

artifacts_dir="${1:-$(pwd)}"

# Change to artifacts directory
cd "$artifacts_dir" || {
	echo "Error: Cannot change to directory: $artifacts_dir" >&2
	exit 1
}

# Ensure we're using latest RubyGems
gem update --system

# Find all gem files
shopt -s nullglob
mapfile -t gems < <(find . -maxdepth 1 -name 'kreuzberg-*.gem' -print | sort)

if [ ${#gems[@]} -eq 0 ]; then
	echo "No gem artifacts found in $artifacts_dir" >&2
	exit 1
fi

# Rebuild each gem to ensure proper tar structure
for gem in "${gems[@]}"; do
	echo "Rebuilding ${gem} to fix tar structure"

	# Unpack the gem
	gem unpack "${gem}"
	gem_name=$(basename "${gem}" .gem)

	# Extract gemspec from gem metadata
	gem specification "${gem}" --ruby >"${gem_name}/${gem_name}.gemspec"

	# Rebuild the gem (this creates proper tar structure)
	(cd "${gem_name}" && gem build "${gem_name}.gemspec")

	# Replace original gem with rebuilt one
	mv "${gem_name}/${gem}" "./${gem}"

	# Cleanup
	rm -rf "${gem_name}"

	echo "Rebuilt ${gem} successfully"
done

echo "All gems rebuilt successfully"
