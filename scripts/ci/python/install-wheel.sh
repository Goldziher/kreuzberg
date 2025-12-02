#!/usr/bin/env bash
#
# Install appropriate wheel based on platform
# Used by: ci-python.yaml - Install wheel step
#

set -euo pipefail

echo "=== Installing wheel for current platform ==="

if ls dist/kreuzberg-*-manylinux*.whl 1> /dev/null 2>&1; then
    echo "Found manylinux wheel"
    python -m pip install dist/kreuzberg-*-manylinux*.whl
elif ls dist/kreuzberg-*-macos*.whl 1> /dev/null 2>&1; then
    echo "Found macOS wheel"
    python -m pip install dist/kreuzberg-*-macos*.whl
elif ls dist/kreuzberg-*-win_amd64.whl 1> /dev/null 2>&1; then
    echo "Found Windows wheel"
    python -m pip install dist/kreuzberg-*-win_amd64.whl
else
    echo "Installing generic wheel"
    python -m pip install dist/kreuzberg-*.whl
fi

echo "Wheel installation complete"
