#!/usr/bin/env bash
#
# Run Rust unit tests
# Used by: ci-rust.yaml - Run unit tests step
#

set -euo pipefail

echo "=== Running Rust unit tests ==="

# Set Tesseract data path for all platforms
if [ "$RUNNER_OS" = "Linux" ]; then
    export TESSDATA_PREFIX=/usr/share/tesseract-ocr/5/tessdata
elif [ "$RUNNER_OS" = "macOS" ]; then
    export TESSDATA_PREFIX="$HOME/Library/Application Support/tesseract-rs/tessdata"
elif [ "$RUNNER_OS" = "Windows" ]; then
    # Windows uses shorter path to avoid MAX_PATH issues
    export TESSDATA_PREFIX="$APPDATA/tesseract-rs/tessdata"
fi

echo "TESSDATA_PREFIX: ${TESSDATA_PREFIX:-not set}"

cargo test \
    --workspace \
    --exclude kreuzberg-e2e-generator \
    --exclude kreuzberg-rb \
    --exclude kreuzberg-py \
    --exclude kreuzberg-node \
    --all-features

echo "Tests complete"
