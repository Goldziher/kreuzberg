# CI Workflow Script Mapping

This document maps each extracted script to its corresponding GitHub Actions workflow step.

## ci-docker.yaml

| Step Name | Script | Shell | Arguments |
|-----------|--------|-------|-----------|
| Free up disk space | `docker/free-disk-space.sh` | bash | None |
| Build Docker image | `docker/build-image.sh` | bash | `<variant>` (core\|full) |
| Test Docker image size | `docker/check-image-size.sh` | bash | `<variant>` (core\|full) |
| Save Docker image as artifact | `docker/save-image.sh` | bash | `<variant>` `[output-dir]` |
| Collect Docker logs on failure | `docker/collect-logs.sh` | bash | `[log-dir]` |
| Clean up Docker resources | `docker/cleanup.sh` | bash | `<variant>` (core\|full) |
| Summary | `docker/summary.sh` | bash | `<variant>` `[results-file]` |

**Usage Example:**
```yaml
- name: Free up disk space
  run: ./scripts/ci/docker/free-disk-space.sh

- name: Build Docker image
  run: ./scripts/ci/docker/build-image.sh ${{ matrix.variant }}

- name: Test Docker image size
  run: ./scripts/ci/docker/check-image-size.sh ${{ matrix.variant }}
```

---

## ci-go.yaml

| Step Name | Script | Shell | Arguments |
|-----------|--------|-------|-----------|
| Build FFI library | `go/build-ffi.ps1` | pwsh | None (detects platform) |
| Build Go bindings | `go/build-bindings.ps1` | pwsh | None (detects platform) |
| Reorganize FFI libraries for Windows GNU target | `go/reorganize-libraries.ps1` | pwsh | None |
| Run Go tests | `go/run-tests.sh` | bash/pwsh | None (detects platform) |

**Usage Example:**
```yaml
- name: Build FFI library
  shell: pwsh
  run: & ./scripts/ci/go/build-ffi.ps1

- name: Run Go tests
  shell: bash
  run: ./scripts/ci/go/run-tests.sh
```

---

## ci-java.yaml

| Step Name | Script | Shell | Arguments |
|-----------|--------|-------|-----------|
| Build Java bindings | `java/build-java.sh` | bash | None |
| Run Java tests | `java/run-tests.sh` | bash | None |

**Usage Example:**
```yaml
- name: Build Java bindings
  shell: bash
  run: ./scripts/ci/java/build-java.sh

- name: Run Java tests
  run: ./scripts/ci/java/run-tests.sh
```

---

## ci-node.yaml

| Step Name | Script | Shell | Arguments |
|-----------|--------|-------|-----------|
| Build Node bindings | `node/build-napi.sh` | bash | `<target>` (e.g., x86_64-unknown-linux-gnu) |
| Unpack and install Node bindings | `node/unpack-bindings.sh` | bash | None |
| Run TypeScript tests | `node/setup-library-paths.sh` + test command | bash | None (sourced) |
| Run E2E tests | `node/setup-library-paths.sh` + task command | bash | None (sourced) |

**Usage Example:**
```yaml
- name: Build Node bindings
  shell: bash
  run: ./scripts/ci/node/build-napi.sh ${{ matrix.target }}

- name: Unpack and install Node bindings
  shell: bash
  run: ./scripts/ci/node/unpack-bindings.sh

- name: Run TypeScript tests
  shell: bash
  run: |
    source ./scripts/ci/node/setup-library-paths.sh
    cd packages/typescript
    pnpm test
```

---

## ci-python.yaml

| Step Name | Script | Shell | Arguments |
|-----------|--------|-------|-----------|
| Clean previous wheel artifacts | `python/clean-artifacts.sh` | bash | None |
| Smoke test wheel | `python/smoke-test-wheel.sh` | bash | None |
| Install wheel | `python/install-wheel.sh` | bash | None |
| Run Python tests | `python/run-tests.sh` | bash | `<coverage>` (true\|false) `[pytest-args]` |
| Setup library paths (pre-test) | `python/setup-library-paths.sh` | bash | None (sourced) |

**Usage Example:**
```yaml
- name: Clean previous wheel artifacts
  shell: bash
  run: ./scripts/ci/python/clean-artifacts.sh

- name: Smoke test wheel
  shell: bash
  run: ./scripts/ci/python/smoke-test-wheel.sh

- name: Install wheel
  shell: bash
  run: ./scripts/ci/python/install-wheel.sh

- name: Run Python tests
  shell: bash
  run: |
    source ./scripts/ci/python/setup-library-paths.sh
    ./scripts/ci/python/run-tests.sh ${{ matrix.coverage }}
```

---

## ci-ruby.yaml

| Step Name | Script | Shell | Arguments |
|-----------|--------|-------|-----------|
| Install Ruby deps | `ruby/install-ruby-deps.sh` (Unix) or `install-ruby-deps.ps1` (Windows) | bash/pwsh | None |
| Vendor kreuzberg core crate | `ruby/vendor-kreuzberg-core.sh` | bash | None |
| Configure bindgen compatibility headers (Windows) | `ruby/configure-bindgen-windows.ps1` | pwsh | None |
| Configure Tesseract build environment (Windows) | `ruby/configure-tesseract-windows.ps1` | pwsh | None |
| Build Ruby gem | `ruby/build-gem.sh` | bash | None |
| Install gem | `ruby/install-gem.sh` | bash | None |
| Build local native extension | `ruby/compile-extension.sh` | bash | None |
| Run Ruby tests | `ruby/run-tests.sh` | bash | None |

**Usage Example:**
```yaml
- name: Install Ruby deps
  if: runner.os != 'Windows'
  shell: bash
  run: ./scripts/ci/ruby/install-ruby-deps.sh

- name: Install Ruby deps (Windows)
  if: runner.os == 'Windows'
  shell: pwsh
  run: & ./scripts/ci/ruby/install-ruby-deps.ps1

- name: Vendor kreuzberg core crate
  shell: bash
  run: ./scripts/ci/ruby/vendor-kreuzberg-core.sh

- name: Build Ruby gem
  shell: bash
  run: ./scripts/ci/ruby/build-gem.sh

- name: Run Ruby tests
  run: ./scripts/ci/ruby/run-tests.sh
```

---

## ci-rust.yaml

| Step Name | Script | Shell | Arguments |
|-----------|--------|-------|-----------|
| Configure bindgen compatibility headers (Windows) | `rust/configure-bindgen-windows.ps1` | pwsh | None |
| Run unit tests | `rust/run-unit-tests.sh` | bash | None |
| Package CLI (Unix) | `rust/package-cli-unix.sh` | bash | `<target>` |
| Package CLI (Windows) | `rust/package-cli-windows.ps1` | pwsh | `<target>` |
| Extract and test CLI (Unix) | `rust/test-cli-unix.sh` | bash | `<target>` |
| Extract and test CLI (Windows) | `rust/test-cli-windows.ps1` | pwsh | `<target>` |

**Usage Example:**
```yaml
- name: Run unit tests
  shell: bash
  run: ./scripts/ci/rust/run-unit-tests.sh

- name: Package CLI (Unix)
  if: matrix.archive == 'tar.gz'
  shell: bash
  run: ./scripts/ci/rust/package-cli-unix.sh ${{ matrix.target }}

- name: Package CLI (Windows)
  if: matrix.archive == 'zip'
  shell: pwsh
  run: & ./scripts/ci/rust/package-cli-windows.ps1 -Target "${{ matrix.target }}"

- name: Extract and test CLI (Unix)
  if: matrix.archive == 'tar.gz'
  shell: bash
  run: ./scripts/ci/rust/test-cli-unix.sh ${{ matrix.target }}
```

---

## ci-validate.yaml

| Step Name | Script | Shell | Arguments |
|-----------|--------|-------|-----------|
| Run lint | `validate/run-lint.sh` | bash | None |

**Usage Example:**
```yaml
- name: Run lint
  run: ./scripts/ci/validate/run-lint.sh
  shell: bash
```

---

## ci-csharp.yaml

| Step Name | Script | Shell | Arguments |
|-----------|--------|-------|-----------|
| Build C# bindings | `csharp/build-csharp.sh` | bash | None |
| Run C# tests | `csharp/run-tests.sh` | bash | None |

**Usage Example:**
```yaml
- name: Build C# bindings
  shell: bash
  run: ./scripts/ci/csharp/build-csharp.sh

- name: Run C# tests
  shell: bash
  env:
    KREUZBERG_FFI_DIR: ${{ github.workspace }}/target/release
  run: ./scripts/ci/csharp/run-tests.sh
```

---

## Shared/Reusable Scripts

These scripts are designed to be sourced/called from multiple workflows:

### setup-library-paths.sh
- **Used in**: Python, Node/TypeScript workflows
- **Purpose**: Setup PDFium and ONNX Runtime library paths based on OS
- **Usage**: Source before running tests
  ```bash
  source ./scripts/ci/python/setup-library-paths.sh
  # or
  source ./scripts/ci/node/setup-library-paths.sh
  ```

### configure-bindgen-windows.ps1
- **Used in**: Ruby and Rust workflows (Windows only)
- **Purpose**: Configure bindgen compatibility headers for MSVC
- **Usage**: Call before builds
  ```powershell
  & ./scripts/ci/ruby/configure-bindgen-windows.ps1
  & ./scripts/ci/rust/configure-bindgen-windows.ps1
  ```

### configure-tesseract-windows.ps1
- **Used in**: Ruby workflow (Windows only)
- **Purpose**: Setup temporary directory and Tesseract environment
- **Usage**: Call before build steps
  ```powershell
  & ./scripts/ci/ruby/configure-tesseract-windows.ps1
  ```

---

## Notes

1. All bash scripts use `#!/usr/bin/env bash` shebang for portability
2. PowerShell scripts use `Set-StrictMode` and `$ErrorActionPreference` for error handling
3. Scripts are self-documenting with headers and usage information
4. Arguments are validated with usage messages on missing parameters
5. Library path scripts are designed to be sourced, not executed directly
6. Windows-specific scripts (.ps1) should use `pwsh` shell in workflow
7. Unix scripts (.sh) should use `bash` shell in workflow
