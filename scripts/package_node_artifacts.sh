#!/usr/bin/env bash
set -euo pipefail

TARGET="${TARGET:?TARGET environment variable must be set}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Verify that the build has already produced .node artifacts
ARTIFACT_DIR="$ROOT/crates/kreuzberg-node/artifacts"
if [[ ! -d "$ARTIFACT_DIR" ]]; then
	echo "ERROR: Artifacts directory not found at $ARTIFACT_DIR" >&2
	echo "You must run build_napi_module.sh BEFORE running this script." >&2
	exit 1
fi

shopt -s nullglob
NODE_FILES=("$ARTIFACT_DIR"/*.node)
if [[ ${#NODE_FILES[@]} -eq 0 ]]; then
	echo "ERROR: No .node files found in $ARTIFACT_DIR" >&2
	echo "The native module must be built first using build_napi_module.sh" >&2
	exit 1
fi

echo "Found ${#NODE_FILES[@]} .node file(s) in $ARTIFACT_DIR"

pushd "$ROOT" >/dev/null

# Run napi artifacts to organize the built .node files into platform packages
pnpm --filter @kreuzberg/node exec napi artifacts --output-dir ./artifacts

if [[ ! -d crates/kreuzberg-node/npm ]]; then
	echo "ERROR: npm artifact directory missing after running napi artifacts" >&2
	exit 1
fi

# Create tarball for distribution
tar -czf "node-bindings-${TARGET}.tar.gz" -C crates/kreuzberg-node npm
echo "Created node-bindings-${TARGET}.tar.gz"

popd >/dev/null
