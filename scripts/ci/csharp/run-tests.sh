#!/usr/bin/env bash
#
# Run C# tests
# Used by: ci-csharp.yaml - Run C# tests step
# Requires: KREUZBERG_FFI_DIR environment variable
#

set -euo pipefail

if [ -z "${KREUZBERG_FFI_DIR:-}" ]; then
	echo "Error: KREUZBERG_FFI_DIR environment variable not set"
	exit 1
fi

echo "=== Running C# tests ==="
echo "FFI directory: $KREUZBERG_FFI_DIR"

cd packages/csharp
dotnet test Kreuzberg.Tests/Kreuzberg.Tests.csproj -c Release

echo "C# tests complete"
