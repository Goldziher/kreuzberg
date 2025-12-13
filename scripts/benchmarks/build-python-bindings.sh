#!/usr/bin/env bash
# Builds Python bindings using maturin in release mode
# No required environment variables
# Assumes current working directory is packages/python or changes to it

set -euo pipefail

# Get repo root
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# Source shared utilities
source "$REPO_ROOT/scripts/lib/common.sh"
source "$REPO_ROOT/scripts/lib/library-paths.sh"

# Validate repo root
validate_repo_root "$REPO_ROOT" || exit 1

# Setup library paths for build
setup_all_library_paths "$REPO_ROOT"

cd "$REPO_ROOT/packages/python"
uv run maturin develop --release
