#!/usr/bin/env bash
#
# Clean up Docker resources
# Used by: ci-docker.yaml - Clean up Docker resources step
# Arguments: VARIANT (core|full)
#

set -euo pipefail

VARIANT="${1:-}"

if [ -z "$VARIANT" ]; then
  echo "Usage: cleanup.sh <variant>"
  echo "  variant: core or full"
  exit 1
fi

echo "=== Cleaning up Docker resources ==="

# Stop and remove all kreuzberg-test containers
docker ps -aq --filter "name=kreuzberg-test" | xargs -r docker rm -f || true

# Remove test image
docker rmi "kreuzberg:$VARIANT" || true

# Clean up Docker system
docker system prune -af --volumes || true

echo "=== Final disk space ==="
df -h /
