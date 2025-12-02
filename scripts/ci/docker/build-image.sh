#!/usr/bin/env bash
#
# Build Docker image for specified variant
# Used by: ci-docker.yaml - Build Docker image step
# Arguments: VARIANT (core|full)
#

set -euo pipefail

VARIANT="${1:-}"

if [ -z "$VARIANT" ]; then
  echo "Usage: build-image.sh <variant>"
  echo "  variant: core or full"
  exit 1
fi

echo "=== Building Docker image ($VARIANT) ==="
docker build -f "docker/Dockerfile.$VARIANT" -t "kreuzberg:$VARIANT" .

echo "=== Docker image built successfully ==="
