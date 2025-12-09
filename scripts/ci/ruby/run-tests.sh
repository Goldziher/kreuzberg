#!/usr/bin/env bash
#
# Run Ruby tests
# Used by: ci-ruby.yaml - Run Ruby tests step
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# scripts/ci/ruby lives three levels below repo root
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

# Validate REPO_ROOT is correct by checking for Cargo.toml
if [ ! -f "$REPO_ROOT/Cargo.toml" ]; then
	echo "Error: REPO_ROOT validation failed. Expected Cargo.toml at: $REPO_ROOT/Cargo.toml" >&2
	echo "REPO_ROOT resolved to: $REPO_ROOT" >&2
	exit 1
fi

echo "=== Running Ruby tests ==="
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" || "$OSTYPE" == "win32" ]]; then
	# Ensure native deps (pdfium, etc.) are on PATH for DLL lookup
	export PATH="$REPO_ROOT/target/x86_64-pc-windows-gnu/release:$REPO_ROOT/target/release:$PATH"
else
	# Ensure native deps are on loader path for Linux/macOS
	export LD_LIBRARY_PATH="$REPO_ROOT/target/release:${LD_LIBRARY_PATH:-}"
	export DYLD_LIBRARY_PATH="$REPO_ROOT/target/release:${DYLD_LIBRARY_PATH:-}"
fi
cd "$REPO_ROOT/packages/ruby"
bundle exec rspec
echo "Tests complete"
