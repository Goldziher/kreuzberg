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

# Check kreuzberg crate
if cargo search kreuzberg --limit 1 | grep -q "kreuzberg = \"${version}\""; then
  echo "kreuzberg_exists=true" >> "$GITHUB_OUTPUT"
  echo "::notice::Rust crate kreuzberg ${version} already exists on crates.io"
else
  echo "kreuzberg_exists=false" >> "$GITHUB_OUTPUT"
fi

# Check kreuzberg-tesseract crate
if cargo search kreuzberg-tesseract --limit 1 | grep -q "kreuzberg-tesseract = \"${version}\""; then
  echo "tesseract_exists=true" >> "$GITHUB_OUTPUT"
  echo "::notice::Rust crate kreuzberg-tesseract ${version} already exists on crates.io"
else
  echo "tesseract_exists=false" >> "$GITHUB_OUTPUT"
fi

# Check kreuzberg-cli crate
if cargo search kreuzberg-cli --limit 1 | grep -q "kreuzberg-cli = \"${version}\""; then
  echo "cli_exists=true" >> "$GITHUB_OUTPUT"
  echo "::notice::Rust crate kreuzberg-cli ${version} already exists on crates.io"
else
  echo "cli_exists=false" >> "$GITHUB_OUTPUT"
fi

# Set all_exist if all three crates exist
if grep -q "kreuzberg_exists=true" "$GITHUB_OUTPUT" && \
   grep -q "tesseract_exists=true" "$GITHUB_OUTPUT" && \
   grep -q "cli_exists=true" "$GITHUB_OUTPUT"; then
  echo "all_exist=true" >> "$GITHUB_OUTPUT"
else
  echo "all_exist=false" >> "$GITHUB_OUTPUT"
fi
