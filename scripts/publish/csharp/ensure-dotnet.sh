#!/usr/bin/env bash

set -euo pipefail

if command -v dotnet >/dev/null 2>&1; then
	dotnet --info
	exit 0
fi

echo "dotnet not found on PATH; installing SDK from dotnet-install.sh (channel 10.0)"
curl -sSL https://dot.net/v1/dotnet-install.sh -o dotnet-install.sh
chmod +x dotnet-install.sh
./dotnet-install.sh --channel 10.0 --quality ga --install-dir "$HOME/.dotnet"

echo "$HOME/.dotnet" >>"${GITHUB_PATH:?GITHUB_PATH not set}"
echo "$HOME/.dotnet/tools" >>"${GITHUB_PATH:?GITHUB_PATH not set}"

dotnet --info
