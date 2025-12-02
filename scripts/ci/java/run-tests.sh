#!/usr/bin/env bash
#
# Run Java tests with Maven
# Used by: ci-java.yaml - Run Java tests step
#

set -euo pipefail

echo "=== Running Java tests ==="
cd packages/java
mvn test
echo "Java tests complete"
