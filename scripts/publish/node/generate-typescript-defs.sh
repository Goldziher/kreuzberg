#!/usr/bin/env bash

set -euo pipefail

pnpm --filter @kreuzberg/node exec napi build --platform --dts index.d.ts
mkdir -p typescript-defs
cp crates/kreuzberg-node/index.d.ts typescript-defs/
cp crates/kreuzberg-node/index.js typescript-defs/ || true
