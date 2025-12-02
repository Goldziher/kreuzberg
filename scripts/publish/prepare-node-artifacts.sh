#!/usr/bin/env bash

# Prepare Node package artifacts for publishing
#
# Unpacks Node binding tarballs and organizes npm packages.
# Merges TypeScript definitions if available.
#
# Arguments:
#   $1: Node artifacts source directory (default: node-artifacts)
#   $2: TypeScript defs source directory (default: typescript-defs)
#   $3: Destination directory (default: crates/kreuzberg-node)

set -euo pipefail

artifacts_dir="${1:-node-artifacts}"
typescript_defs_dir="${2:-typescript-defs}"
dest_dir="${3:-crates/kreuzberg-node}"

if [ ! -d "$artifacts_dir" ]; then
  echo "Error: Artifacts directory not found: $artifacts_dir" >&2
  exit 1
fi

# Clean and prepare destination
rm -rf "$dest_dir/npm"
mkdir -p "$dest_dir/npm"

shopt -s nullglob
for pkg in "$artifacts_dir"/*.tar.gz; do
  echo "Unpacking $pkg"
  tmpdir=$(mktemp -d)
  tar -xzf "$pkg" -C "$tmpdir"

  if [ ! -d "$tmpdir/npm" ]; then
    echo "::warning::npm directory missing inside $pkg"
    rm -rf "$tmpdir"
    continue
  fi

  while IFS= read -r -d '' dir; do
    name=$(basename "$dir")
    dest="$dest_dir/npm/$name"
    rm -rf "$dest"
    cp -R "$dir" "$dest"
  done < <(find "$tmpdir/npm" -mindepth 1 -maxdepth 1 -type d -print0)

  rm -rf "$tmpdir"
done

# Merge TypeScript definitions if available
if [ -d "$typescript_defs_dir" ]; then
  cp "$typescript_defs_dir"/index.js "$typescript_defs_dir"/index.d.ts "$dest_dir/" || true
  echo "TypeScript definitions merged"
fi

echo "Node artifacts prepared successfully"
