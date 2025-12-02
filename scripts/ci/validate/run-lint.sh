#!/usr/bin/env bash
#
# Run all linting and validation checks
# Used by: ci-validate.yaml - Run lint step
#

set -euo pipefail

echo "=== Running all lint checks ==="
task lint
echo "Lint checks complete"
