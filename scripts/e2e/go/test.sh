#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"

export DYLD_LIBRARY_PATH="${REPO_ROOT}/target/release:${DYLD_LIBRARY_PATH:-}"
export LD_LIBRARY_PATH="${REPO_ROOT}/target/release:${LD_LIBRARY_PATH:-}"
export CGO_LDFLAGS="-L${REPO_ROOT}/target/release -Wl,-rpath,${REPO_ROOT}/target/release"

cd "${REPO_ROOT}/e2e/go"
go test ./...
