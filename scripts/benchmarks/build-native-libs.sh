#!/usr/bin/env bash
# Builds workspace with all features and benchmark harness in release mode
# No required environment variables

set -euo pipefail

cargo build --workspace --release --all-features
cargo build --manifest-path tools/benchmark-harness/Cargo.toml --release
