#!/usr/bin/env bash
#
# Unpack and install Node bindings from tarball
# Used by: ci-node.yaml - Unpack and install Node bindings step
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/common.sh"

validate_repo_root "$REPO_ROOT" || exit 1

cd "$REPO_ROOT"

echo "=== Unpacking and installing Node bindings ==="

cd "$REPO_ROOT/crates/kreuzberg-node"

# Extract the tarball to get the built .node file
pkg=$(find . -maxdepth 1 -name "kreuzberg-node-*.tgz" -print | head -n 1)
if [ -z "$pkg" ]; then
	echo "No kreuzberg-node tarball found" >&2
	exit 1
fi

echo "Found package: $pkg"

# Get absolute path to tarball before changing directories
pkg_abs_path="$(cd "$(dirname "$pkg")" && pwd)/$(basename "$pkg")"

# On Windows (Git Bash/MSYS), convert POSIX path to Windows path for file: URL
if command -v cygpath >/dev/null 2>&1; then
	pkg_abs_path="$(cygpath -w "$pkg_abs_path")"
fi

# Install the package from tarball in the workspace root
cd "$REPO_ROOT"
pnpm add --workspace-root "file:$pkg_abs_path"

echo "Installation complete"
