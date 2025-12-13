#!/usr/bin/env bash
# Runs benchmark harness for a specific framework and mode
# Required environment variables:
#   - FRAMEWORK: Name of framework to benchmark (e.g., kreuzberg-native, kreuzberg-python-sync)
#   - MODE: Benchmark mode - "single-file" or "batch"
#   - ITERATIONS: Number of iterations to run
#   - TIMEOUT: Timeout per document in seconds (default: 900)
# Optional environment variables:
#   - FIXTURES_DIR: Path to fixtures directory (default: tools/benchmark-harness/fixtures)
#   - HARNESS_PATH: Path to benchmark harness binary (default: ./target/release/benchmark-harness)

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

# Set PKG_CONFIG_PATH for Go bindings (needed for kreuzberg-go-* frameworks)
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export PKG_CONFIG_PATH="${REPO_ROOT}/crates/kreuzberg-ffi:${PKG_CONFIG_PATH:-}"

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
