#!/usr/bin/env bash
set -euo pipefail

artifacts_dir="${1:-artifacts/java-natives}"
resources_root="${2:-packages/java/src/main/resources/natives}"

mkdir -p "$resources_root"

find "$artifacts_dir" -maxdepth 2 -type f -print

for artifact in "$artifacts_dir"/*; do
	if [ ! -d "$artifact" ]; then
		continue
	fi
	for rid_dir in "$artifact"/*; do
		if [ ! -d "$rid_dir" ]; then
			continue
		fi
		rid="$(basename "$rid_dir")"
		mkdir -p "$resources_root/$rid"
		cp -f "$rid_dir"/* "$resources_root/$rid/"
	done
done

find "$resources_root" -maxdepth 2 -type f -print | sort
