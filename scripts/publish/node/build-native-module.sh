#!/usr/bin/env bash

set -euo pipefail

target="${TARGET:?TARGET not set}"
use_cross="${USE_CROSS:-false}"
use_napi_cross="${USE_NAPI_CROSS:-false}"

args=(--platform --release --target "$target" --output-dir ./artifacts)
if [ "$use_napi_cross" = "true" ]; then
	args+=(--use-napi-cross)
fi
if [ "$use_cross" = "true" ]; then
	args+=(--use-cross)
fi

pnpm --filter @kreuzberg/node exec napi build "${args[@]}"
