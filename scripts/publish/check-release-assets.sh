#!/usr/bin/env bash

set -euo pipefail

tag="${TAG:?TAG not set}"
asset_prefix="${ASSET_PREFIX:?ASSET_PREFIX not set}"
summary_label="${SUMMARY_LABEL:-assets}"

tmp_json="$(mktemp)"
exists=false

if gh release view "$tag" --json assets >"$tmp_json" 2>/dev/null; then
	if jq -e --arg prefix "$asset_prefix" '.assets[].name | select(startswith($prefix))' "$tmp_json" >/dev/null; then
		exists=true
	fi
fi

rm -f "$tmp_json"

echo "exists=$exists" >>"${GITHUB_OUTPUT:?GITHUB_OUTPUT not set}"

if [ "$exists" = "true" ] && [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
	echo "${summary_label} artifacts already exist on release ${tag}; skipping." >>"$GITHUB_STEP_SUMMARY"
fi
