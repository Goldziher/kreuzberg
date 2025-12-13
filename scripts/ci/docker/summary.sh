#!/usr/bin/env bash
#
# Print test summary
# Used by: ci-docker.yaml - Summary step
# Arguments: VARIANT (core|full), RESULTS_FILE (optional, defaults to /tmp/kreuzberg-docker-test-results.json)
#

set -euo pipefail

VARIANT="${1:-}"
RESULTS_FILE="${2:-/tmp/kreuzberg-docker-test-results.json}"

if [ -z "$VARIANT" ]; then
	echo "Usage: summary.sh <variant> [results-file]"
	echo "  variant: core or full"
	echo "  results-file: path to test results JSON (default: /tmp/kreuzberg-docker-test-results.json)"
	exit 1
fi

echo "✅ Docker image built and tested successfully!"
echo ""
echo "Variant: $VARIANT"
echo "Image: kreuzberg:$VARIANT"
echo ""

if [ -f "$RESULTS_FILE" ]; then
	echo "Test Results:"
	jq . <"$RESULTS_FILE" || cat "$RESULTS_FILE"
fi
