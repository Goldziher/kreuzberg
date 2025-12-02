#!/usr/bin/env bash
#
# Run Python tests with optional coverage
# Used by: ci-python.yaml - Run Python tests step
# Arguments: COVERAGE (true|false), optional pytest args
#

set -euo pipefail

COVERAGE="${1:-false}"
shift || true

cd packages/python

echo "=== Running Python tests ==="

if [ "$COVERAGE" = "true" ]; then
    echo "Coverage enabled"
    uv run pytest -vv --cov=kreuzberg --cov-report=lcov:coverage.lcov --cov-report=term --cov-config=pyproject.toml --reruns 1 --reruns-delay 1 "$@"
else
    echo "Coverage disabled"
    uv run pytest -vv --reruns 1 --reruns-delay 1 "$@"
fi

echo "Tests complete"
