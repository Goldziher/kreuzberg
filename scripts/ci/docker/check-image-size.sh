#!/usr/bin/env bash
#
# Check Docker image size and warn if it exceeds thresholds
# Used by: ci-docker.yaml - Test Docker image size step
# Arguments: VARIANT (core|full)
#

set -euo pipefail

VARIANT="${1:-}"

if [ -z "$VARIANT" ]; then
  echo "Usage: check-image-size.sh <variant>"
  echo "  variant: core or full"
  exit 1
fi

size=$(docker images "kreuzberg:$VARIANT" --format "{{.Size}}")
echo "Docker image size ($VARIANT): $size"

# Extract numeric value in MB
size_mb=$(docker inspect "kreuzberg:$VARIANT" --format='{{.Size}}' | awk '{print int($1/1024/1024)}')
echo "Image size in MB: $size_mb"

# Warn if image is larger than expected (2.5GB for full, 1.5GB for core)
if [ "$VARIANT" = "full" ] && [ "$size_mb" -gt 2560 ]; then
  echo "::warning::Full image is larger than 2.5GB ($size_mb MB). Consider optimization."
elif [ "$VARIANT" = "core" ] && [ "$size_mb" -gt 1536 ]; then
  echo "::warning::Core image is larger than 1.5GB ($size_mb MB). Consider optimization."
fi
