#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Set Tesseract data path for all platforms
case "${RUNNER_OS:-}" in
Linux) export TESSDATA_PREFIX="/usr/share/tesseract-ocr/5/tessdata" ;;
macOS) export TESSDATA_PREFIX="$HOME/Library/Application Support/tesseract-rs/tessdata" ;;
Windows) export TESSDATA_PREFIX="$APPDATA/tesseract-rs/tessdata" ;;
esac

cd "${REPO_ROOT}/packages/csharp"
dotnet test Kreuzberg.Tests/Kreuzberg.Tests.csproj -c Release
