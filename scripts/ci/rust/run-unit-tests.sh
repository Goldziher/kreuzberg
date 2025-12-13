#!/usr/bin/env bash
#
# Run Rust unit tests
# Used by: ci-rust.yaml - Run unit tests step
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/common.sh"
source "$REPO_ROOT/scripts/lib/tessdata.sh"

validate_repo_root "$REPO_ROOT" || exit 1

cd "$REPO_ROOT"

echo "=== Running Rust unit tests ==="

setup_tessdata

echo "TESSDATA_PREFIX: ${TESSDATA_PREFIX:-not set}"

cargo test \
	--workspace \
	--exclude kreuzberg-e2e-generator \
	--exclude kreuzberg-py \
	--exclude kreuzberg-node \
	--all-features

echo "Tests complete"
