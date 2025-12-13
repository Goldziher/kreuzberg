#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

# Source shared utilities
source "${REPO_ROOT}/scripts/lib/common.sh"
source "${REPO_ROOT}/scripts/lib/library-paths.sh"

# Validate repository structure
validate_repo_root "$REPO_ROOT" || exit 1

echo "=========================================="
echo "Go Test Configuration and Diagnostics"
echo "=========================================="
echo "Script directory: $SCRIPT_DIR"
echo "Repository root: $REPO_ROOT"
echo "Operating system: $OSTYPE"
echo "Go version: $(go version)"
echo

cd "$REPO_ROOT/packages/go"
echo "Working directory: $(pwd)"
echo

# Build list of Go packages that actually contain source files (skip empty module root)
echo "Discovering Go packages (excluding empty roots)..."
packages=()
while IFS= read -r dir; do
	# Skip the module root if it contains no Go sources
	if [[ "$dir" != "." ]]; then
		packages+=("./$dir")
	fi
done < <(find . -name '*.go' -not -path './vendor/*' -exec dirname {} \; | sort -u)

echo "Found packages: $(printf '%s ' "${packages[@]}")"
echo

if [[ ${#packages[@]} -eq 0 ]]; then
	echo "Error: No Go packages found in $(pwd)"
	find . -name '*.go' -not -path './vendor/*' | head -20
	exit 1
fi

if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" || "$OSTYPE" == "win32" ]]; then
	# Windows-specific setup
	setup_go_paths "$REPO_ROOT"

	echo "=========================================="
	echo "Windows Test Configuration"
	echo "=========================================="
	echo "PKG_CONFIG_PATH: ${PKG_CONFIG_PATH:-<not set>}"
	echo "CGO_LDFLAGS: ${CGO_LDFLAGS:-<not set>}"
	echo "CGO_ENABLED: ${CGO_ENABLED:-<not set>}"
	echo

	# Skip -race on Windows: mingw + static CRT regularly fails to link race runtime
	extra_flags=()
	if [[ -n "${GO_TEST_FLAGS:-}" ]]; then
		read -r -a extra_flags <<<"$GO_TEST_FLAGS"
	fi

	go test -v -x "${extra_flags[@]}" "${packages[@]}"
else
	# Unix (Linux/macOS) setup
	setup_go_paths "$REPO_ROOT"

	echo "=========================================="
	echo "Unix Test Configuration"
	echo "=========================================="
	echo "LD_LIBRARY_PATH: $LD_LIBRARY_PATH"
	echo "DYLD_FALLBACK_LIBRARY_PATH: ${DYLD_FALLBACK_LIBRARY_PATH:-<not set>}"
	echo "CGO_LDFLAGS: ${CGO_LDFLAGS:-<not set>}"
	echo "CGO_ENABLED: ${CGO_ENABLED:-<not set>}"
	echo "PKG_CONFIG_PATH: ${PKG_CONFIG_PATH:-<not set>}"
	echo

	extra_flags=()
	if [[ -n "${GO_TEST_FLAGS:-}" ]]; then
		read -r -a extra_flags <<<"$GO_TEST_FLAGS"
	fi

	go test -v -race -x "${extra_flags[@]}" "${packages[@]}"
fi
