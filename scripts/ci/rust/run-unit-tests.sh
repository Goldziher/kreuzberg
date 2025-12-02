#!/usr/bin/env bash
#
# Run Rust unit tests
# Used by: ci-rust.yaml - Run unit tests step
#

set -euo pipefail

echo "=== Running Rust unit tests ==="

# Set Tesseract data path for Linux
if [ "$RUNNER_OS" = "Linux" ]; then
    export TESSDATA_PREFIX=/usr/share/tesseract-ocr/5/tessdata
fi

cargo test \
    --workspace \
    --exclude kreuzberg-e2e-generator \
    --exclude kreuzberg-rb \
    --exclude kreuzberg-py \
    --exclude kreuzberg-node \
    --all-features

echo "Tests complete"
