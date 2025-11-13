# Kreuzberg CI/CD Documentation

This document describes the CI/CD infrastructure for Kreuzberg, including workflows, composite actions, and best practices.

## Table of Contents

- [Overview](#overview)
- [Workflows](#workflows)
- [Composite Actions](#composite-actions)
- [Build Matrix](#build-matrix)
- [Testing Strategy](#testing-strategy)
- [Release Process](#release-process)

## Overview

Kreuzberg uses a multi-tiered CI/CD approach:

1. **CI Workflow** - Fast feedback on every PR/push
2. **Build Artifacts Workflow** - Full build matrix validation (legacy, being phased out)
3. **Publish Workflow** - Cross-platform builds and release

### Design Principles

- **Composite Actions**: Reusable build and test logic across workflows
- **Multi-Platform**: Linux, macOS, Windows support
- **Language Bindings**: Python, Node.js, Ruby, Rust, CLI
- **Fast Feedback**: Native builds in CI, cross-compilation in publish
- **Installation Validation**: Smoke tests verify all installation paths

## Workflows

### 1. CI (`ci.yaml`)

**Triggers**: Pull requests, pushes to main

**Purpose**: Fast feedback loop for developers

**Jobs**:
- `validate` - Pre-commit checks, linting, formatting
- `ts-test` - TypeScript tests across Node versions
- `py-test` - Python tests across Python versions
- `ruby-test` - Ruby tests across Ruby versions
- `rust-unit` - Rust unit tests
- `rust-e2e` - Rust end-to-end tests
- `build-cli` - Build CLI binaries (native platforms)
- `build-node` - Build Node.js bindings (native platforms)
- `build-ruby` - Build Ruby gems (native platforms)
- `smoke` - Smoke test all packages from source

**Build Strategy**: Native builds only (no cross-compilation)
- Linux: x86_64-unknown-linux-gnu
- macOS: aarch64-apple-darwin
- Windows: x86_64-pc-windows-msvc

**Duration**: ~15-30 minutes

### 2. Build Artifacts (`build-artifacts.yaml`)

**Triggers**: Pushes to main, tags

**Purpose**: Full build matrix validation (legacy)

**Jobs**:
- `build-wheels` - Python wheels (all platforms including ARM64, musl)
- `build-sdist` - Python source distribution
- `build-napi-bindings` - Node.js bindings (6 targets)
- `build-ruby-gems` - Ruby gems (4 platforms)
- `build-cli-binaries` - CLI binaries (3 targets)
- `smoke-tests` - Validate built artifacts
- `summarize` - Build summary report

**Build Strategy**: Full cross-compilation matrix
- Python: linux-x86_64, linux-aarch64, macos-universal, windows-amd64
- Node.js: 6 targets (gnu/musl/darwin/windows × arch)
- Ruby: linux-x86_64, linux-aarch64, macos-arm64, windows-x64
- CLI: linux-gnu, macos-aarch64, windows-msvc

**Duration**: ~60-180 minutes

**Status**: Being phased out in favor of CI workflow for quick validation

### 3. Publish (`publish.yaml`)

**Triggers**: Tags (v*), manual workflow dispatch

**Purpose**: Build and publish releases

**Jobs**:
- `prepare` - Extract version, validate tag
- `python-wheels` - Build Python wheels (all platforms)
- `python-sdist` - Build Python source distribution
- `node-bindings` - Build Node.js bindings (6 targets)
- `cli-binaries` - Build CLI binaries (5 targets including musl, aarch64)
- `ruby-gem` - Build Ruby gems (3 platforms)
- `cargo-packages` - Package Rust crates
- `release-smoke` - Multi-platform smoke tests (3 OS)
- `publish-docker` - Build and push Docker images
- `create-release` - Create GitHub release
- `publish-crates` - Publish to crates.io
- `publish-pypi` - Publish to PyPI
- `publish-rubygems` - Publish to RubyGems
- `publish-node` - Publish to npm
- `publish-homebrew` - Update Homebrew formula

**Build Strategy**: Full cross-compilation for all supported targets

**Duration**: ~120-240 minutes

## Composite Actions

Located in `.github/actions/`, these provide reusable build and test logic.

### Build Actions

#### `build-cli`
Builds and packages CLI binary for a specified target.

**Inputs**:
- `target` (required): Rust target triple
- `use-cross` (optional): Whether to use cross for cross-compilation

**Outputs**:
- `archive-path`: Path to packaged .tar.gz or .zip

**Usage**:
```yaml
- uses: ./.github/actions/build-cli
  with:
    target: x86_64-unknown-linux-gnu
    use-cross: "false"
```

#### `build-node`
Builds NAPI-RS Node.js bindings for a specified target.

**Inputs**:
- `target` (required): Rust target triple
- `use-cross` (optional): Use cross for cross-compilation
- `pack-tarball` (optional): Create npm tarball

**Outputs**:
- `binding-path`: Path to .node file
- `tarball`: Path to npm package tarball (if pack-tarball=true)

#### `build-ruby-unix`
Builds Ruby gem on Unix platforms (Linux/macOS).

**Inputs**:
- `platform` (required): Ruby platform (x86_64-linux, arm64-darwin)
- `build-source-gem` (optional): Also build source gem

**Outputs**:
- `gem-path`: Path to built .gem file

#### `build-ruby-windows`
Builds Ruby gem on Windows using MSYS2.

**Inputs**:
- `platform` (required): Ruby platform (x64-mingw32)

**Outputs**:
- `gem-path`: Path to built .gem file

### Smoke Test Actions

Smoke tests validate that packages can be installed and used.

#### `smoke-python`
Tests Python wheel installation.

**Inputs**:
- `wheel-path` (optional): Path to wheel file or directory. Empty = install from source

**Usage**:
```yaml
- uses: ./.github/actions/smoke-python
  with:
    wheel-path: artifacts/python  # Or "" for source install
```

#### `smoke-node`
Tests Node.js package installation.

**Inputs**:
- `package-tarball` (optional): Path to .tgz file. Empty = install from source

#### `smoke-ruby`
Tests Ruby gem installation.

**Inputs**:
- `gem-path` (optional): Path to .gem file or directory. Empty = install from source

#### `smoke-cli`
Tests CLI binary execution.

**Inputs**:
- `cli-artifacts-dir` (required): Directory containing CLI archives

#### `smoke-rust`
Tests Rust crate as a dependency.

**Inputs**:
- `source-path` (optional): Path to crate source

## Build Matrix

### Python Wheels

| Platform | Target | Runner | Manylinux |
|----------|--------|--------|-----------|
| Linux x86_64 | x86_64-unknown-linux-gnu | ubuntu-latest | auto |
| Linux ARM64 | aarch64-unknown-linux-gnu | ubuntu-latest | 2_28 |
| macOS Universal | universal2-apple-darwin | macos-14 | - |
| Windows x64 | x86_64-pc-windows-msvc | windows-latest | - |

### Node.js Bindings

| Target | Runner | Notes |
|--------|--------|-------|
| x86_64-unknown-linux-gnu | ubuntu-latest | Native |
| x86_64-unknown-linux-musl | ubuntu-latest | Alpine Linux |
| aarch64-apple-darwin | macos-14 | Apple Silicon |
| x86_64-apple-darwin | macos-13 | Intel Mac |
| x86_64-pc-windows-msvc | windows-latest | Native |
| aarch64-pc-windows-msvc | windows-latest | ARM64 Windows |

### Ruby Gems

| Platform | Target | Runner |
|----------|--------|--------|
| Linux x86_64 | x86_64-linux | ubuntu-latest |
| Linux ARM64 | aarch64-linux | ubuntu-latest |
| macOS ARM64 | arm64-darwin | macos-14 |
| Windows x64 | x64-mingw32 | windows-latest |

### CLI Binaries

| Target | Runner | Notes |
|--------|--------|-------|
| x86_64-unknown-linux-gnu | ubuntu-latest | Standard Linux |
| x86_64-unknown-linux-musl | ubuntu-latest | Static binary |
| aarch64-unknown-linux-gnu | ubuntu-latest | ARM64 Linux |
| aarch64-apple-darwin | macos-14 | Apple Silicon |
| x86_64-pc-windows-msvc | windows-latest | Windows |

## Testing Strategy

### Unit Tests
- **Rust**: `cargo test` on Linux and macOS
- **Python**: pytest with coverage
- **TypeScript**: vitest
- **Ruby**: RSpec

### Integration Tests
- Rust E2E tests with test documents
- Python integration tests
- TypeScript integration tests

### Smoke Tests
Validate installation paths users will actually use:

**Python**:
```bash
python -m venv .venv
source .venv/bin/activate
pip install kreuzberg-*.whl
python -c "import kreuzberg; print(kreuzberg.__version__)"
```

**Node.js**:
```bash
pnpm install kreuzberg-node.tgz
node -e "const k = require('@kreuzberg/node'); console.log('ok')"
```

**Ruby**:
```bash
gem install kreuzberg-*.gem
ruby -e "require 'kreuzberg'; puts Kreuzberg::VERSION"
```

**CLI**:
```bash
./kreuzberg --version
```

**Rust**:
```bash
cargo new test-project
cd test-project
# Add kreuzberg dependency
cargo build
```

### Test Coverage
- Target: 95% for Rust core
- Python: Covered by pytest
- TypeScript: Covered by vitest
- Ruby: Covered by RSpec

## Release Process

### 1. Prepare Release

1. Update version in `Cargo.toml` (workspace root)
2. Run `task sync-versions` to propagate to all packages
3. Update CHANGELOG.md
4. Commit: `git commit -m "chore: prepare v4.0.0 release"`

### 2. Create Tag

```bash
git tag -a v4.0.0 -m "Release v4.0.0"
git push origin v4.0.0
```

### 3. Automated Build

The `publish.yaml` workflow triggers automatically:
- Builds all artifacts
- Runs multi-platform smoke tests
- Creates GitHub release with artifacts
- Publishes to registries (if not dry-run)

### 4. Manual Publish (if needed)

```bash
# Trigger manually with specific ref
gh workflow run publish.yaml \
  -f tag=v4.0.0 \
  -f dry_run=false \
  -f ref=main
```

### 5. Verify

Check published packages:
- **PyPI**: https://pypi.org/project/kreuzberg/
- **npm**: https://www.npmjs.com/package/@kreuzberg/node
- **RubyGems**: https://rubygems.org/gems/kreuzberg
- **crates.io**: https://crates.io/crates/kreuzberg
- **Homebrew**: https://github.com/Goldziher/homebrew-tap

## Troubleshooting

### Failed Builds

**Linux ARM64 wheels fail with "file in wrong format"**:
- Tesseract cache may contain x86 libraries
- Solution: Cache cleanup step in workflow removes stale cache

**macOS universal wheels fail**:
- Ensure both x86_64 and aarch64 targets are installed
- Check MACOSX_DEPLOYMENT_TARGET is set correctly

**Windows MSYS2 build fails**:
- Ruby requires MSYS2 UCRT64 environment
- Check BINDGEN_EXTRA_CLANG_ARGS is set for bindgen

### Smoke Test Failures

**Python wheel import error**:
- Check wheel was built for correct platform
- Verify manylinux compatibility

**Node.js binding not found**:
- Check .node file is in correct npm package structure
- Verify NAPI-RS artifacts command ran successfully

**Ruby gem load error**:
- Check native extension compiled for correct platform
- Verify bundler configuration

### CI Performance

**Slow CI runs**:
- Check if caching is working (Rust, Python, Node.js)
- Consider reducing test matrix for faster feedback
- Use native builds in CI, cross-compilation only in publish

**Out of disk space**:
- Maximize build space action is used on Linux
- Clean up caches between jobs
- Remove unnecessary dependencies

## Contributing

When adding new build targets or platforms:

1. Add to appropriate workflow matrix
2. Update composite actions if needed
3. Add smoke test coverage
4. Update this documentation
5. Test locally with `act` if possible

When modifying workflows:

1. Test with workflow_dispatch trigger first
2. Check all dependent jobs
3. Verify artifact paths
4. Update documentation

## Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [NAPI-RS Documentation](https://napi.rs/)
- [Maturin Documentation](https://www.maturin.rs/)
- [Magnus (Ruby) Documentation](https://github.com/matsadler/magnus)
- [Cross Compilation Guide](https://rust-lang.github.io/rustup/cross-compilation.html)
