#!/usr/bin/env bash

set -euo pipefail

{
	echo "CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc"
	echo "CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++"
	echo "AR_aarch64_unknown_linux_gnu=aarch64-linux-gnu-ar"
	echo "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc"
} >>"${GITHUB_ENV:?GITHUB_ENV not set}"
