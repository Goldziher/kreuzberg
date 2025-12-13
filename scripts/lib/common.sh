#!/usr/bin/env bash
#
# Common shell utility functions shared across all build/test scripts
#
# This library provides:
#   - get_repo_root()      - Walk up directory tree to find repo root
#   - validate_repo_root() - Verify repo root by checking for Cargo.toml
#   - error_exit()         - Standardized error handling with context
#   - get_platform()       - Detect OS platform (Linux/macOS/Windows)
#
# Usage:
#   source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
#   REPO_ROOT=$(get_repo_root)
#

set -euo pipefail

# get_repo_root: Walk up from current working directory until finding Cargo.toml
# Returns the absolute path to the repository root
# Exits with code 1 if Cargo.toml is not found
get_repo_root() {
	local start_dir current_dir
	start_dir="$(pwd)"
	current_dir="$start_dir"

	# Walk up the directory tree looking for Cargo.toml
	while [ "$current_dir" != "/" ]; do
		if [ -f "$current_dir/Cargo.toml" ]; then
			echo "$current_dir"
			return 0
		fi
		current_dir="$(dirname "$current_dir")"
	done

	# Cargo.toml not found
	echo "Error: Could not find repository root (Cargo.toml) from: $start_dir" >&2
	return 1
}

# validate_repo_root: Verify that REPO_ROOT contains Cargo.toml
# Args:
#   $1 - Path to validate (uses REPO_ROOT env var if not provided)
# Returns 0 if valid, 1 if invalid
validate_repo_root() {
	local repo_root="${1:-${REPO_ROOT:-}}"

	if [ -z "$repo_root" ]; then
		echo "Error: REPO_ROOT not provided and env var not set" >&2
		return 1
	fi

	if [ ! -f "$repo_root/Cargo.toml" ]; then
		echo "Error: REPO_ROOT validation failed. Expected Cargo.toml at: $repo_root/Cargo.toml" >&2
		echo "REPO_ROOT resolved to: $repo_root" >&2
		return 1
	fi

	return 0
}

# error_exit: Standardized error handling with context
# Args:
#   $1 - Error message
#   $2 - Exit code (optional, defaults to 1)
# Never returns; always exits
error_exit() {
	local message="${1:-Unknown error}"
	local exit_code="${2:-1}"
	echo "Error: $message" >&2
	exit "$exit_code"
}

# get_platform: Detect the operating system
# Returns: Linux, macOS, Windows, or unknown
get_platform() {
	if [ -n "${RUNNER_OS:-}" ]; then
		# GitHub Actions environment variable
		echo "$RUNNER_OS"
	else
		case "$(uname -s)" in
		Linux*)
			echo "Linux"
			;;
		Darwin*)
			echo "macOS"
			;;
		MINGW* | MSYS* | CYGWIN*)
			echo "Windows"
			;;
		*)
			echo "unknown"
			;;
		esac
	fi
}

# Export functions so they can be used in sourced scripts
export -f get_repo_root
export -f validate_repo_root
export -f error_exit
export -f get_platform
