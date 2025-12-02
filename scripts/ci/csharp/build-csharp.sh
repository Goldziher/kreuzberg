#!/usr/bin/env bash
#
# Build C# bindings
# Used by: ci-csharp.yaml - Build C# bindings step
#

set -euo pipefail

echo "=== Building C# bindings ==="
cd packages/csharp
dotnet build Kreuzberg/Kreuzberg.csproj -c Release
echo "C# build complete"
