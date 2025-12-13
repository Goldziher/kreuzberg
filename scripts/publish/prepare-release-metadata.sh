#!/usr/bin/env bash

set -euo pipefail

metadata="$(bash scripts/publish/validate-and-compute-release-metadata.sh)"
echo "$metadata" >release-metadata.json

{
	echo "tag=$(echo "$metadata" | jq -r '.tag')"
	echo "version=$(echo "$metadata" | jq -r '.version')"
	echo "ref=$(echo "$metadata" | jq -r '.ref')"
	echo "dry_run=$(echo "$metadata" | jq -r '.dry_run')"
	echo "checkout_ref=$(echo "$metadata" | jq -r '.checkout_ref')"
	echo "target_sha=$(echo "$metadata" | jq -r '.target_sha')"
	echo "matrix_ref=$(echo "$metadata" | jq -r '.matrix_ref')"
	echo "is_tag=$(echo "$metadata" | jq -r '.is_tag')"
	echo "release_targets=$(echo "$metadata" | jq -r '.release_targets')"
	echo "release_any=$(echo "$metadata" | jq -r '.release_any')"
	echo "release_python=$(echo "$metadata" | jq -r '.release_python')"
	echo "release_node=$(echo "$metadata" | jq -r '.release_node')"
	echo "release_ruby=$(echo "$metadata" | jq -r '.release_ruby')"
	echo "release_cli=$(echo "$metadata" | jq -r '.release_cli')"
	echo "release_crates=$(echo "$metadata" | jq -r '.release_crates')"
	echo "release_docker=$(echo "$metadata" | jq -r '.release_docker')"
	echo "release_homebrew=$(echo "$metadata" | jq -r '.release_homebrew')"
	echo "release_java=$(echo "$metadata" | jq -r '.release_java')"
	echo "release_csharp=$(echo "$metadata" | jq -r '.release_csharp')"
	echo "release_go=$(echo "$metadata" | jq -r '.release_go')"
	echo "release_wasm=$(echo "$metadata" | jq -r '.release_wasm')"
} >>"${GITHUB_OUTPUT:?GITHUB_OUTPUT not set}"
