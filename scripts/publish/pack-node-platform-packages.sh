#!/usr/bin/env bash

set -euo pipefail

npm_dir="${1:-crates/kreuzberg-node/npm}"

if [ ! -d "$npm_dir" ]; then
	echo "Error: npm directory not found: $npm_dir" >&2
	exit 1
fi

cd "$npm_dir"
for dir in */; do
	if [ -f "${dir}package.json" ]; then
		(cd "$dir" && npm pack && mv ./*.tgz ..)
	fi
done
