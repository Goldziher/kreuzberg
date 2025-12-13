#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"

source "$REPO_ROOT/scripts/lib/common.sh"
source "$REPO_ROOT/scripts/lib/library-paths.sh"
source "$REPO_ROOT/scripts/lib/tessdata.sh"

validate_repo_root "$REPO_ROOT" || exit 1

# Setup Rust FFI and Tesseract paths
setup_rust_ffi_paths "$REPO_ROOT"
setup_tessdata

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
