#!/usr/bin/env bash
#
# Smoke test wheel installation
# Used by: ci-python.yaml - Smoke test wheel step
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# scripts/ci/python lives three levels below repo root
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

echo "=== Installing and testing wheel ==="
pip install --no-index --find-links "$REPO_ROOT/target/wheels" kreuzberg
python "$REPO_ROOT/scripts/python/print_kreuzberg_version.py"
echo "Smoke test passed!"
