#!/usr/bin/env bash

set -euo pipefail

tag="${DOCKER_TAG:?DOCKER_TAG not set}"
label="${SUMMARY_LABEL:-image}"

exists=false
if docker manifest inspect "$tag" >/dev/null 2>&1; then
	exists=true
fi

echo "exists=$exists" >>"${GITHUB_OUTPUT:?GITHUB_OUTPUT not set}"

if [ "$exists" = "true" ] && [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
	echo "Docker tag $tag already exists; ${label} publish will be skipped." >>"$GITHUB_STEP_SUMMARY"
fi
