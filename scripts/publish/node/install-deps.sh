#!/usr/bin/env bash

set -euo pipefail

unset RUSTC_WRAPPER || true
corepack enable
pnpm install -C crates/kreuzberg-node
