#!/usr/bin/env bash
#
# Build C# bindings
# Used by: ci-csharp.yaml - Build C# bindings step
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# scripts/ci/csharp lives three levels below repo root
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

echo "=== Building C# bindings ==="
cd "$REPO_ROOT/packages/csharp"
dotnet build Kreuzberg/Kreuzberg.csproj -c Release
echo "C# build complete"
