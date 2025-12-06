#!/usr/bin/env bash
# Validates that the benchmark harness directory exists
# Required environment variables: GITHUB_REF (optional, for logging)

set -euo pipefail

if [ ! -d tools/benchmark-harness ]; then
	echo "::error::tools/benchmark-harness not found on branch ${GITHUB_REF}." >&2
	exit 1
fi
