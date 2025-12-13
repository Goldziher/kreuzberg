#!/usr/bin/env bash

set -euo pipefail

published="${WASM_ALREADY_PUBLISHED:-false}"
version="${WASM_VERSION:-unknown}"

if [ -z "${GITHUB_STEP_SUMMARY:-}" ]; then
	exit 0
fi

if [ "$published" = "true" ]; then
	echo "WASM package @kreuzberg/wasm@${version} was already published; skipped republish." >>"$GITHUB_STEP_SUMMARY"
else
	echo "Successfully published WASM package @kreuzberg/wasm@${version} to npm" >>"$GITHUB_STEP_SUMMARY"
	echo "Uploaded WASM artifacts to GitHub release" >>"$GITHUB_STEP_SUMMARY"
fi
