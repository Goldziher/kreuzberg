#!/usr/bin/env bash
# Builds Python bindings using maturin in release mode
# No required environment variables
# Assumes current working directory is packages/python or changes to it

set -euo pipefail

cd packages/python
uv run maturin develop --release
