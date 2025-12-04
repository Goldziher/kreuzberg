#!/usr/bin/env bash
#
# Build Java bindings with Maven
# Used by: ci-java.yaml - Build Java bindings step
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

echo "=== Building Java bindings ==="
cd "$REPO_ROOT/packages/java"
mvn clean package -DskipTests
echo "Java build complete"
