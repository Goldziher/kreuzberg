#!/usr/bin/env bash
#
# Run Java tests with Maven
# Used by: ci-java.yaml - Run Java tests step
#
# Java FFM API requires Rust FFI libraries at runtime, so this script
# configures library paths before executing Maven tests.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/common.sh"
source "$REPO_ROOT/scripts/lib/library-paths.sh"

validate_repo_root "$REPO_ROOT" || exit 1
setup_rust_ffi_paths "$REPO_ROOT"

echo "=== Running Java tests ==="
cd "$REPO_ROOT/packages/java"
mvn -B -DtrimStackTrace=false -Dsurefire.useFile=false test
echo "Java tests complete"
