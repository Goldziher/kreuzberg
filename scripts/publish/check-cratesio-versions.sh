#!/usr/bin/env bash

# Check if Rust crate versions exist on crates.io
#
# Checks three crates:
#   - kreuzberg-tesseract
#   - kreuzberg (core)
#   - kreuzberg-cli
#
# Environment Variables:
#   - VERSION: Package version to check (e.g., 4.0.0-rc.1)

set -euo pipefail

version="${1:?VERSION argument required}"

# Check kreuzberg crate
if cargo search kreuzberg --limit 1 | grep -q "kreuzberg = \"${version}\""; then
  echo "kreuzberg_exists=true"
  echo "::notice::Rust crate kreuzberg ${version} already exists on crates.io"
else
  echo "kreuzberg_exists=false"
fi

# Check kreuzberg-tesseract crate
if cargo search kreuzberg-tesseract --limit 1 | grep -q "kreuzberg-tesseract = \"${version}\""; then
  echo "tesseract_exists=true"
  echo "::notice::Rust crate kreuzberg-tesseract ${version} already exists on crates.io"
else
  echo "tesseract_exists=false"
fi

# Check kreuzberg-cli crate
if cargo search kreuzberg-cli --limit 1 | grep -q "kreuzberg-cli = \"${version}\""; then
  echo "cli_exists=true"
  echo "::notice::Rust crate kreuzberg-cli ${version} already exists on crates.io"
else
  echo "cli_exists=false"
fi

# Set all_exist if all three crates exist (check outputs)
# This requires downstream processing since we're using stdout
