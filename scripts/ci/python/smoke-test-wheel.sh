#!/usr/bin/env bash
#
# Smoke test wheel installation
# Used by: ci-python.yaml - Smoke test wheel step
#

set -euo pipefail

echo "=== Installing and testing wheel ==="
pip install --no-index --find-links target/wheels/ kreuzberg
python -c "import kreuzberg; print(f'Kreuzberg version: {kreuzberg.__version__}')"
echo "Smoke test passed!"
