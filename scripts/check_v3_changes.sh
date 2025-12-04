#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
	echo "Usage: $0 <output-file> <before-sha> <after-sha>" >&2
	exit 1
fi

OUTPUT_FILE="$1"
BEFORE="$2"
AFTER="$3"

if git diff --name-only "$BEFORE" "$AFTER" | grep -q '^v3/'; then
	echo "v3_changed=true" >>"$OUTPUT_FILE"
else
	echo "v3_changed=false" >>"$OUTPUT_FILE"
fi
