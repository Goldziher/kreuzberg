#!/usr/bin/env bash

set -euo pipefail

# Enable arm64 architecture support and install cross-compilation toolchain
sudo dpkg --add-architecture arm64
sudo apt-get update
sudo apt-get install -y \
	gcc-aarch64-linux-gnu \
	g++-aarch64-linux-gnu \
	binutils-aarch64-linux-gnu \
	pkg-config-aarch64-linux-gnu

{
	echo "CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc"
	echo "CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++"
	echo "AR_aarch64_unknown_linux_gnu=aarch64-linux-gnu-ar"
	echo "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc"
} >>"${GITHUB_ENV:?GITHUB_ENV not set}"

{
	echo "PKG_CONFIG_PATH=/usr/lib/aarch64-linux-gnu/pkgconfig"
	echo "PKG_CONFIG_LIBDIR=/usr/lib/aarch64-linux-gnu/pkgconfig"
} >>"${GITHUB_ENV:?GITHUB_ENV not set}"
