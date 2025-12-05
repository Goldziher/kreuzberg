#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"

# Prepare Tesseract data and export TESSDATA_PREFIX
source "${REPO_ROOT}/scripts/ci/csharp/setup-tessdata.sh"

export DYLD_LIBRARY_PATH="${REPO_ROOT}/target/release:${DYLD_LIBRARY_PATH:-}"
export LD_LIBRARY_PATH="${REPO_ROOT}/target/release:${LD_LIBRARY_PATH:-}"
# Ensure tesseract binary is on PATH for OCR tests
case "${RUNNER_OS:-$(uname -s)}" in
Linux)
	PATH="/usr/bin:${PATH}"
	;;
macOS)
	PATH="/opt/homebrew/bin:/usr/local/bin:${PATH}"
	;;
Windows*)
	PATH="/c/Program Files/Tesseract-OCR:${PATH}"
	;;
esac

cd "${REPO_ROOT}/e2e/csharp"
dotnet test Kreuzberg.E2E.csproj -c Release
