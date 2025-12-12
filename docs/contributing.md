# Contributing Guide

Thanks for helping improve Kreuzberg! This guide summarizes the workflow, tooling, and expectations for contributions across all language bindings.

## Prerequisites

- Rust toolchain (`rustup`) with the stable channel.
- Python 3.10+ (we use `uv` for virtualenv and dependency management).
- Node.js 20+ with `pnpm`.
- Ruby 3.3+ via `rbenv` (preferred) or `rvm`.
- Go 1.25+ (install via `brew install go`, `asdf install golang`, or the official installer).
- Java 25+ (required for FFM API bindings).
- .NET 9+ (required for C# bindings).
- Homebrew (macOS) or system equivalents for Tesseract/Pdfium dependencies.

Install all project dependencies in one shot:

```bash title="Terminal"
task setup
```

This runs language-specific installers (Python `uv sync`, `pnpm install`, `bundle install`) and builds the Rust workspace.

## Building from Source

### Build-Time Dependencies

Kreuzberg uses pure Rust dependencies and requires no system libraries beyond standard build tools.

**Fastembed Fork**: We maintain a fork at `kreuzberg-dev/fastembed-rs` that uses `rustls` (pure Rust TLS) instead of `native-tls` (OpenSSL). This eliminates OpenSSL as a build dependency and simplifies cross-platform builds. The fork will be retired once upstream `ort` publishes rustls support to crates.io.

### Platform-Specific Requirements

**Linux**:
```bash title="Terminal"
# Ubuntu/Debian
sudo apt-get install build-essential libssl-dev pkg-config

# Fedora/RHEL
sudo dnf groupinstall "Development Tools"
sudo dnf install openssl-devel pkg-config
```

**macOS**:
```bash title="Terminal"
# Install OpenSSL 3.x
brew install openssl@3

# Install Xcode Command Line Tools (required for compilation)
xcode-select --install
```

**Windows**:
- Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022) with C++ support
- Or use MSYS2 with MinGW-w64: `pacman -S mingw-w64-x86_64-toolchain`

### Building the Rust Core

```bash title="Terminal"
# Build all workspace crates
cargo build --workspace

# Build with specific features
cargo build -p kreuzberg --features full

# Build release binaries
cargo build --release --workspace
```

### Building Language Bindings

**Python** (PyO3):
```bash title="Terminal"
cd packages/python

# Build and install in development mode (editable install)
maturin develop

# Build wheel package for distribution
maturin build
```

**TypeScript** (NAPI-RS):
```bash title="Terminal"
cd packages/typescript

# Build native module and generate TypeScript declarations
pnpm build
```

**Ruby** (Magnus):
```bash title="Terminal"
cd packages/ruby

# Compile native extension
bundle exec rake compile

# Build platform-specific gem
bundle exec rake native:gem
```

**Go** (cgo):
```bash title="Terminal"
# Build FFI library with full features (Linux/macOS)
cargo build -p kreuzberg-ffi --release

# Build FFI library for Windows (MinGW cannot link ONNX Runtime, so use core features only)
cargo build -p kreuzberg-ffi --release --target x86_64-pc-windows-gnu --no-default-features --features core

# Set library paths for Go to find the FFI library
export DYLD_FALLBACK_LIBRARY_PATH=$PWD/target/release  # macOS
export LD_LIBRARY_PATH=$PWD/target/release             # Linux
# Windows: Add target\release to PATH environment variable

# Install Go development tools (golangci-lint) and download modules
task go:install

# Run code formatters and linters (gofmt + golangci-lint)
task go:lint

# Run Go tests (requires libkreuzberg_ffi in target/release)
task go:test

# Regenerate test fixtures and run end-to-end Go tests
task e2e:go:verify
```

**Note:** Windows Go builds use MinGW (GNU toolchain) which cannot link MSVC-only ONNX Runtime. The `core` feature excludes embeddings but includes all other functionality.

**Java** (FFM API):
```bash title="Terminal"
cd packages/java

# Clean, compile, and run tests
mvn clean compile test

# Run code quality checks (Checkstyle)
mvn checkstyle:check
```

**C#** (.NET):
```bash title="Terminal"
cd packages/csharp

# Build the C# project
dotnet build

# Run all tests
dotnet test
```

### Common Build Issues

**Cross-compilation**:
```bash title="Terminal"
# Install target
rustup target add x86_64-unknown-linux-musl

# Build with cross
cargo install cross
cross build --target x86_64-unknown-linux-musl
```

**Linker errors on Linux**:
Ensure you have `gcc` and `binutils` installed:
```bash title="Terminal"
sudo apt-get install build-essential
```

## Development Workflow

1. **Create a branch** off `main` with a descriptive name (e.g., `feat/python-config-alias`).
2. **Make changes** with small, focused commits. Code should compile on all supported platforms.
3. **Run tests/lint** for the areas you touched:
   - `task lint` – cross-language linters (cargo clippy, Ruff, Rubocop, Biome/Oxlint, Mypy).
   - `task dev:test` – full test matrix (Rust + Python + Ruby + TypeScript + Go + Java + C#).
   - Language-specific shortcuts: `task python:test`, `task typescript:test`, `task ruby:test`, `task rust:test`, `task go:test`, `task java:test`, `task csharp:test`.
4. **Write/Update docs** when adding features. User-facing content lives under `docs/` and must be referenced in `mkdocs.yaml`.
5. **Ensure conventional commits** (`feat: ...`, `fix: ...`, `docs: ...`). The pre-commit hook checks commit messages.
6. **Create a pull request** with a clear summary, screenshots/logs if relevant, and a checklist of tests you ran.

## Coding Standards

- **Rust**: edition 2024, no `unwrap` in production paths, document all public items, add `SAFETY` comments for unsafe blocks.
- **Python**: dataclasses use `frozen=True`, `slots=True`; function-based pytest tests; follow Ruff/Mypy rules.
- **TypeScript**: maintain strict types, avoid `any`, keep bindings in `packages/typescript/src` and tests under `tests/binding|smoke|cli`.
- **Ruby**: no global state outside `Kreuzberg` module, keep native bridge panic-free, follow Rubocop defaults.
- **Java**: FFM API (Foreign Function & Memory), sealed classes, records, pattern matching; JUnit 5; follow Checkstyle rules.
- **C#**: .NET 9+; follow Microsoft C# coding conventions; use records for data types, nullable reference types enabled.
- **Testing strategy**: Only language-specific smoke/binding tests live in each package; shared behavior belongs to the `e2e/` fixtures (Python, Ruby, TypeScript, Rust, Go, Java, C# runners). When adding a new feature, update the relevant fixture and regenerate via `task e2e:<lang>:generate`.

## Documentation

- User docs only belong under `docs/`. Each new page must be added to `mkdocs.yaml`.
- Prefer linking to existing guides or references rather than duplicating explanations.
- Run `mkdocs build` (or `task docs:build`) if you add/rename files to ensure nav entries resolve.

## Submitting a PR

Before opening a PR, verify:

- [ ] `task lint` passes.
- [ ] Targeted tests or `task dev:test` pass.
- [ ] Docs and changelog entries are updated (if applicable).
- [ ] New files include appropriate licenses/headers where required.
- [ ] Commit messages follow Conventional Commits.

Once reviewed and merged, GitHub Actions will produce updated wheels, gems, N-API bundles, CLI binaries, and Docker images.

Thanks again for contributing!
