#!/usr/bin/env bash
#
# Build Java bindings with Maven
# Used by: ci-java.yaml - Build Java bindings step
#

set -euo pipefail

echo "=== Building Java bindings ==="
cd packages/java
mvn clean package -DskipTests
echo "Java build complete"
