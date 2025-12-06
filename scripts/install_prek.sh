#!/usr/bin/env bash
set -euo pipefail

uv tool install prek
echo "$HOME/.local/bin" >>"$GITHUB_PATH"
