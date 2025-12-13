#!/usr/bin/env bash

set -euo pipefail

echo "=== Initial disk usage ==="
df -h /

echo "=== Removing unnecessary packages ==="
sudo rm -rf /usr/share/dotnet /usr/local/lib/android /opt/ghc /opt/hostedtoolcache/CodeQL || true
sudo rm -rf /usr/local/share/boost /usr/local/lib/node_modules /opt/microsoft /usr/local/.ghcup || true

echo "=== Cleaning apt cache ==="
sudo apt-get clean || true
sudo rm -rf /var/lib/apt/lists/* || true

echo "=== Cleaning Docker ==="
docker system prune -af --volumes || true
docker builder prune -af || true

echo "=== Disk usage after cleanup ==="
df -h /
