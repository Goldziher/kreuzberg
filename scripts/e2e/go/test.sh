#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"

# Source shared utilities
source "${REPO_ROOT}/scripts/lib/common.sh"
source "${REPO_ROOT}/scripts/lib/library-paths.sh"

# Validate repository structure
validate_repo_root "$REPO_ROOT" || exit 1

# Setup Go paths
setup_go_paths "$REPO_ROOT"

# Run E2E tests
cd "${REPO_ROOT}/e2e/go"
go test ./...
