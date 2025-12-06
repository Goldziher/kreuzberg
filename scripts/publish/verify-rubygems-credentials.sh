#!/usr/bin/env bash

# Verify RubyGems credentials file exists and has correct permissions
#
# Checks for ~/.gem/credentials file and sets appropriate permissions.
# Required before pushing gems to RubyGems.

set -euo pipefail

credentials_file="${HOME}/.gem/credentials"

if [ ! -f "$credentials_file" ]; then
	echo "::error::RubyGems credentials file not found at $credentials_file"
	ls -la "${HOME}/.gem/" || echo "${HOME}/.gem directory does not exist"
	exit 1
fi

chmod 600 "$credentials_file"
echo "RubyGems credentials verified at $credentials_file"
