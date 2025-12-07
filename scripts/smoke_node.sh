#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}/e2e/smoke/node"

: "${KREUZBERG_NODE_SPEC:?Set KREUZBERG_NODE_SPEC=file:/abs/path/to/kreuzberg.tgz}"
node ../../.github/actions/smoke-node/update-package-spec.js
rm -f pnpm-lock.yaml
pnpm install --no-frozen-lockfile
pnpm run check
