#!/usr/bin/env bash

set -euo pipefail

target="${CLI_TARGET:-}"

sudo apt-get update
case "$target" in
aarch64-unknown-linux-gnu)
	sudo apt-get install -y gcc-aarch64-linux-gnu
	;;
*) ;;
esac
