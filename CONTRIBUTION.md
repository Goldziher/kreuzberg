# Contribution Guide

## Build Requirements

- Rust toolchain with `cargo` (install via `rustup`)
- C/C++ build support (`clang` or `gcc`) for compiling native extensions
- `cmake` 3.24 or newer (required by the `tesseract-rs` build scripts)
- `uv` for Python dependency management
- `task` CLI for running project automation

If any requirement is missing, install it before running project tasks. On macOS you can install `cmake` with `brew install cmake`; on Linux use your distribution package manager.

## Environment Setup

Run `task setup` from the repository root to install dependencies and install the `prek` git hooks. The task now verifies the build requirements above before attempting to build the mixed Rust/Python package.
