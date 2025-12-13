#!/usr/bin/env bash
#
# Install appropriate wheel based on platform
# Used by: ci-python.yaml - Install wheel step
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/common.sh"

validate_repo_root "$REPO_ROOT" || exit 1

cd "$REPO_ROOT"

echo "=== Installing wheel for current platform ==="

# Find first matching wheel regardless of platform-specific suffix
wheel_path="$(find dist -maxdepth 1 -name "kreuzberg-*.whl" -print -quit 2>/dev/null || true)"

if [ -z "$wheel_path" ]; then
	echo "No wheel found in dist/. Contents:"
	ls -l dist || true
	exit 1
fi

echo "Installing wheel: $wheel_path"
python -m pip install "$wheel_path"

echo "Wheel installation complete"
