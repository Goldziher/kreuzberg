#!/usr/bin/env bash
# Builds C# benchmark project using dotnet
# No required environment variables
# Assumes current working directory is packages/csharp or changes to it

set -euo pipefail

cd packages/csharp
dotnet build Benchmark/Benchmark.csproj -c Release
