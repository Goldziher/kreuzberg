#!/usr/bin/env bash

set -euo pipefail

target="${TARGET:?TARGET not set}"

pnpm --filter @kreuzberg/node exec napi artifacts --output-dir ./artifacts
if [ ! -d crates/kreuzberg-node/npm ]; then
	echo "npm artifact directory missing" >&2
	exit 1
fi
tar -czf "node-bindings-${target}.tar.gz" -C crates/kreuzberg-node npm
