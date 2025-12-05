#!/usr/bin/env bash
#
# Run Python tests with optional coverage
# Used by: ci-python.yaml - Run Python tests step
# Arguments: COVERAGE (true|false), optional pytest args
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# scripts/ci/python lives three levels below repo root
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

# Validate REPO_ROOT is correct by checking for Cargo.toml
if [ ! -f "$REPO_ROOT/Cargo.toml" ]; then
    echo "Error: REPO_ROOT validation failed. Expected Cargo.toml at: $REPO_ROOT/Cargo.toml" >&2
    echo "REPO_ROOT resolved to: $REPO_ROOT" >&2
    exit 1
fi

COVERAGE="${1:-false}"
shift || true

cd "$REPO_ROOT/packages/python"

echo "=== Running Python tests ==="

if [ "$COVERAGE" = "true" ]; then
	echo "Coverage enabled"
	uv run pytest -vv --cov=kreuzberg --cov-report=lcov:coverage.lcov --cov-report=term --cov-config=pyproject.toml --reruns 1 --reruns-delay 1 "$@"
else
	echo "Coverage disabled"
	uv run pytest -vv --reruns 1 --reruns-delay 1 "$@"
fi

echo "Tests complete"
