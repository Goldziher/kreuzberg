#!/usr/bin/env bash

# Verify Cargo.toml version matches release tag
#
# Compares workspace version in Cargo.toml with the release version.
# Exits with error if they don't match.
#
# Arguments:
#   $1: Expected release version (e.g., 4.0.0-rc.1)
#   $2: Path to Cargo.toml (default: Cargo.toml)

set -euo pipefail

tag_version="${1:?Release version argument required}"
cargo_toml="${2:-Cargo.toml}"

if [ ! -f "$cargo_toml" ]; then
  echo "Error: Cargo.toml not found: $cargo_toml" >&2
  exit 1
fi

cargo_version=$(grep '^\[workspace.package\]' -A 10 "$cargo_toml" | grep '^version = ' | head -1 | sed -E 's/version = "(.*)"/\1/')

if [ -z "$cargo_version" ]; then
  echo "Error: Could not extract version from $cargo_toml" >&2
  exit 1
fi

if [ "$cargo_version" != "$tag_version" ]; then
  echo "Version mismatch! Cargo: $cargo_version, tag: $tag_version" >&2
  exit 1
fi

echo "Cargo.toml version matches tag: $cargo_version"
