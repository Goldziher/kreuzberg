#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

cd "${REPO_ROOT}/packages/go"
if [ -n "$(gofmt -l .)" ]; then
	exit 1
fi
