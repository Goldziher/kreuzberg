#!/usr/bin/env bash
#
# Run Java tests with Maven
# Used by: ci-java.yaml - Run Java tests step
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# scripts/ci/java lives three levels below repo root
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

echo "=== Running Java tests ==="
cd "$REPO_ROOT/packages/java"
mvn test
echo "Java tests complete"
