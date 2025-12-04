#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"

# Prepare Tesseract data and export TESSDATA_PREFIX
source "${REPO_ROOT}/scripts/ci/csharp/setup-tessdata.sh"

export DYLD_LIBRARY_PATH="${REPO_ROOT}/target/release:${DYLD_LIBRARY_PATH:-}"
export LD_LIBRARY_PATH="${REPO_ROOT}/target/release:${LD_LIBRARY_PATH:-}"

cd "${REPO_ROOT}/e2e/csharp"
dotnet test Kreuzberg.E2E.csproj -c Release
