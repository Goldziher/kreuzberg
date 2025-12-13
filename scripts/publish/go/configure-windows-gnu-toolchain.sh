#!/usr/bin/env bash

set -euo pipefail

{
	echo "CC=gcc"
	echo "AR=ar"
	echo "RANLIB=ranlib"
	echo "CXX=g++"
	echo "CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc"
	echo "RUSTFLAGS=-C target-feature=+crt-static"
	echo "CARGO_BUILD_TARGET=x86_64-pc-windows-gnu"
} >>"${GITHUB_ENV:?GITHUB_ENV not set}"
