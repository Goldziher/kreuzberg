#!/usr/bin/env bash

set -euo pipefail

sudo apt-get update
sudo apt-get install -y gcc-aarch64-linux-gnu

{
	echo "CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc"
	echo "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc"
} >>"${GITHUB_ENV:?GITHUB_ENV not set}"

{
	echo "PKG_CONFIG_PATH=/usr/lib/aarch64-linux-gnu/pkgconfig"
	echo "PKG_CONFIG_LIBDIR=/usr/lib/aarch64-linux-gnu/pkgconfig"
} >>"${GITHUB_ENV:?GITHUB_ENV not set}"
