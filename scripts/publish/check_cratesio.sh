#!/usr/bin/env bash
#
# Check if Rust crates exist on crates.io
#
# Arguments:
#   $1: Package version (required)
#
# Environment variables:
#   - GITHUB_OUTPUT: Path to GitHub Actions output file
#
# Usage:
#   ./check_cratesio.sh "1.0.0"
#

set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <version>" >&2
  exit 1
fi

version="$1"
max_attempts=3

check_crate() {
  local crate_name="$1"
  local version="$2"
  local max_attempts="$3"
  local attempt=1
  local found=false

  while [ "$attempt" -le "$max_attempts" ]; do
    echo "::debug::Checking crates.io for ${crate_name} ${version} (attempt ${attempt}/${max_attempts})" >&2

    if cargo search "$crate_name" --limit 1 2>/dev/null | grep -q "${crate_name} = \"${version}\""; then
      found=true
      break
    elif [ "$attempt" -lt "$max_attempts" ]; then
      sleep_time=$((attempt * 5))
      echo "::warning::crates.io check for ${crate_name} failed, retrying in ${sleep_time}s..." >&2
      sleep "$sleep_time"
    fi

    attempt=$((attempt + 1))
  done

  echo "$found"
}

# Check kreuzberg crate
if [ "$(check_crate "kreuzberg" "$version" "$max_attempts")" = "true" ]; then
  kreuzberg_exists=true
  echo "::notice::Rust crate kreuzberg ${version} already exists on crates.io" >&2
else
  kreuzberg_exists=false
  echo "::notice::Rust crate kreuzberg ${version} not found on crates.io" >&2
fi

# Check kreuzberg-tesseract crate
if [ "$(check_crate "kreuzberg-tesseract" "$version" "$max_attempts")" = "true" ]; then
  tesseract_exists=true
  echo "::notice::Rust crate kreuzberg-tesseract ${version} already exists on crates.io" >&2
else
  tesseract_exists=false
  echo "::notice::Rust crate kreuzberg-tesseract ${version} not found on crates.io" >&2
fi

# Check kreuzberg-cli crate
if [ "$(check_crate "kreuzberg-cli" "$version" "$max_attempts")" = "true" ]; then
  cli_exists=true
  echo "::notice::Rust crate kreuzberg-cli ${version} already exists on crates.io" >&2
else
  cli_exists=false
  echo "::notice::Rust crate kreuzberg-cli ${version} not found on crates.io" >&2
fi

# Output results
echo "kreuzberg_exists=$kreuzberg_exists"
echo "tesseract_exists=$tesseract_exists"
echo "cli_exists=$cli_exists"

# Set all_exist if all three crates exist
if [ "$kreuzberg_exists" = "true" ] && [ "$tesseract_exists" = "true" ] && [ "$cli_exists" = "true" ]; then
  echo "all_exist=true"
else
  echo "all_exist=false"
fi
