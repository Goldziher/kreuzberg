#!/usr/bin/env bash
# Builds Python bindings using maturin in release mode
# No required environment variables
# Assumes current working directory is packages/python or changes to it

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

cd "$REPO_ROOT/packages/python"
uv run maturin develop --release
