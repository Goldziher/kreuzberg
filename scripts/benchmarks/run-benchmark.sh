#!/usr/bin/env bash
set -euo pipefail

FRAMEWORK="${FRAMEWORK:-}"
MODE="${MODE:-}"
ITERATIONS="${ITERATIONS:-3}"
TIMEOUT="${TIMEOUT:-900}"
FIXTURES_DIR="${FIXTURES_DIR:-tools/benchmark-harness/fixtures}"
HARNESS_PATH="${HARNESS_PATH:-./target/release/benchmark-harness}"

if [ -z "$FRAMEWORK" ] || [ -z "$MODE" ]; then
	echo "::error::FRAMEWORK and MODE environment variables are required" >&2
	exit 1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# Source shared utilities
source "${REPO_ROOT}/scripts/lib/common.sh"
source "${REPO_ROOT}/scripts/lib/library-paths.sh"

# Validate repository structure
validate_repo_root "$REPO_ROOT" || exit 1

# Setup Go paths for any Go-based frameworks
setup_go_paths "$REPO_ROOT"

OUTPUT_DIR="benchmark-results/${FRAMEWORK}-${MODE}"
rm -rf "${OUTPUT_DIR}"

MAX_CONCURRENT=$([[ "$MODE" == "single-file" ]] && echo 1 || echo 4)

"${HARNESS_PATH}" \
	run \
	--fixtures "${FIXTURES_DIR}" \
	--frameworks "${FRAMEWORK}" \
	--output "${OUTPUT_DIR}" \
	--iterations "${ITERATIONS}" \
	--timeout "${TIMEOUT}" \
	--mode "${MODE}" \
	--max-concurrent "${MAX_CONCURRENT}"
