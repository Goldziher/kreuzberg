#!/usr/bin/env bash

set -euo pipefail

echo "=== Cleaning up Docker resources ==="
docker system prune -af --volumes || true
docker builder prune -af || true
echo "=== Final disk space ==="
df -h /
