#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

"${REPO_ROOT}/scripts/download_pdfium_runtime.sh"

cd "${REPO_ROOT}/packages/go"
export LD_LIBRARY_PATH="${REPO_ROOT}/target/release:${LD_LIBRARY_PATH:-}"
export DYLD_FALLBACK_LIBRARY_PATH="${REPO_ROOT}/target/release:${DYLD_FALLBACK_LIBRARY_PATH:-}"
go test ./...
