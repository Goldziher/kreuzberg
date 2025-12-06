#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-${GOLANGCI_LINT_VERSION:-2.6.2}}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

echo "Installing golangci-lint v${VERSION}..."
go install "github.com/golangci/golangci-lint/v2/cmd/golangci-lint@v${VERSION}"

cd "${REPO_ROOT}/packages/go"
go mod download
go mod tidy
