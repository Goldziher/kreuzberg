#!/usr/bin/env bash
#
# Run Python tests with optional coverage
# Used by: ci-python.yaml - Run Python tests step
# Arguments: COVERAGE (true|false), optional pytest args
#

set -euo pipefail

# Get repo root
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# Source shared utilities
source "$REPO_ROOT/scripts/lib/common.sh"
source "$REPO_ROOT/scripts/lib/library-paths.sh"

# Validate repo root
validate_repo_root "$REPO_ROOT" || exit 1

# Setup library paths for Python tests
setup_all_library_paths "$REPO_ROOT"

COVERAGE="${1:-false}"
shift || true

cd "$REPO_ROOT/packages/python"

echo "=== Running Python tests ==="

# Check if pytest-timeout should be used (environment variable or CI flag)
TIMEOUT_ARGS=""
if [ "${PYTEST_TIMEOUT:-}" != "" ]; then
	TIMEOUT_ARGS="--timeout=${PYTEST_TIMEOUT}"
fi

if [ "$COVERAGE" = "true" ]; then
	echo "Coverage enabled"
	uv run pytest -vv --cov=kreuzberg --cov-report=lcov:coverage.lcov --cov-report=term --cov-config=pyproject.toml --reruns 1 --reruns-delay 1 "$TIMEOUT_ARGS" "$@"
else
	echo "Coverage disabled"
	uv run pytest -vv --reruns 1 --reruns-delay 1 "$TIMEOUT_ARGS" "$@"
fi

echo "Tests complete"
