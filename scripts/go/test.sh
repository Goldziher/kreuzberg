#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# Source shared utilities
source "${REPO_ROOT}/scripts/lib/common.sh"
source "${REPO_ROOT}/scripts/lib/library-paths.sh"

# Validate repository structure
validate_repo_root "$REPO_ROOT" || exit 1

# Download pdfium runtime
"${REPO_ROOT}/scripts/download_pdfium_runtime.sh"

# Setup Go paths
setup_go_paths "$REPO_ROOT"

# Run tests
cd "${REPO_ROOT}/packages/go"
go test ./...
