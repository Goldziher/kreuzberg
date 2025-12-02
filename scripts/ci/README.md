# CI Workflow Scripts

This directory contains extracted scripts from GitHub Actions CI workflows, organized by workflow type.

## Overview

- **Total Scripts**: 39 (25 Bash + 14 PowerShell)
- **Documentation**: See `SCRIPT_MAPPING.md` for detailed workflow-to-script mapping
- **All Scripts**: Production-ready with proper error handling and documentation

## Directory Structure

```
scripts/ci/
├── README.md               ← This file
├── SCRIPT_MAPPING.md       ← Detailed workflow-to-script mapping guide
├── docker/                 ← Docker image build and test scripts
├── go/                     ← Go bindings scripts
├── java/                   ← Java bindings scripts
├── node/                   ← Node/TypeScript NAPI scripts
├── python/                 ← Python wheel build scripts
├── ruby/                   ← Ruby gem build scripts
├── rust/                   ← Rust core and CLI scripts
├── csharp/                 ← C# bindings scripts
└── validate/               ← Validation and linting scripts
```

## Quick Start

### Running a Script

**Bash scripts:**
```bash
./scripts/ci/docker/build-image.sh core
./scripts/ci/python/run-tests.sh true
```

**PowerShell scripts:**
```powershell
& ./scripts/ci/go/build-ffi.ps1
& ./scripts/ci/rust/package-cli-windows.ps1 -Target "x86_64-pc-windows-msvc"
```

### Sourcing Scripts

For library path setup scripts:
```bash
source ./scripts/ci/python/setup-library-paths.sh
./scripts/ci/python/run-tests.sh true
```

## Scripts by Workflow

### Docker (`docker/`)
- `free-disk-space.sh` - Clean up CI disk space
- `build-image.sh` - Build Docker image variant
- `check-image-size.sh` - Validate image size constraints
- `save-image.sh` - Save Docker image as tar.gz artifact
- `collect-logs.sh` - Collect container logs on failure
- `cleanup.sh` - Clean up Docker resources
- `summary.sh` - Print test summary

### Go (`go/`)
- `build-ffi.ps1` - Build FFI library (Windows/Unix)
- `build-bindings.ps1` - Build Go bindings with CGO
- `reorganize-libraries.ps1` - Reorganize FFI libraries for Windows
- `run-tests.sh` - Run Go tests with library paths

### Java (`java/`)
- `build-java.sh` - Build Java bindings with Maven
- `run-tests.sh` - Run Java tests with Maven

### Node/TypeScript (`node/`)
- `build-napi.sh` - Build NAPI bindings with artifact collection
- `unpack-bindings.sh` - Unpack and install bindings from tarball
- `setup-library-paths.sh` - Configure PDFium and ONNX Runtime paths

### Python (`python/`)
- `clean-artifacts.sh` - Clean previous wheel artifacts
- `smoke-test-wheel.sh` - Test wheel installation
- `install-wheel.sh` - Install platform-specific wheel
- `run-tests.sh` - Run tests with optional coverage
- `setup-library-paths.sh` - Configure PDFium and ONNX Runtime paths

### Ruby (`ruby/`)
- `install-ruby-deps.sh` - Install bundle dependencies (Unix)
- `install-ruby-deps.ps1` - Install bundle dependencies (Windows)
- `vendor-kreuzberg-core.sh` - Vendor core crate for packaging
- `configure-bindgen-windows.ps1` - Configure bindgen headers (Windows)
- `configure-tesseract-windows.ps1` - Configure Tesseract (Windows)
- `build-gem.sh` - Build Ruby gem
- `install-gem.sh` - Install built gem
- `compile-extension.sh` - Compile native extension
- `run-tests.sh` - Run RSpec tests

### Rust (`rust/`)
- `configure-bindgen-windows.ps1` - Configure bindgen headers (Windows)
- `run-unit-tests.sh` - Run Rust unit tests
- `package-cli-unix.sh` - Package CLI as tar.gz (Unix)
- `package-cli-windows.ps1` - Package CLI as zip (Windows)
- `test-cli-unix.sh` - Test CLI binary (Unix)
- `test-cli-windows.ps1` - Test CLI binary (Windows)

### C# (`csharp/`)
- `build-csharp.sh` - Build C# bindings with dotnet
- `run-tests.sh` - Run C# tests with dotnet

### Validate (`validate/`)
- `run-lint.sh` - Run all linting and validation checks via Task

## Features

### Error Handling
- All Bash scripts use `set -euo pipefail`
- All PowerShell scripts use `Set-StrictMode` and error action preferences
- Proper exit codes and error messages
- Usage information for incorrect arguments

### Documentation
- Every script has a descriptive header
- Purpose and usage clearly stated
- Which CI workflow step uses it
- Argument documentation

### Platform Support
- Windows-specific operations via PowerShell (.ps1)
- Unix operations via Bash (.sh)
- Cross-platform scripts detect OS and adjust behavior
- Library path setup scripts handle Windows/Linux/macOS

### Reusability
- `setup-library-paths.sh` shared by Python and Node
- `configure-bindgen-windows.ps1` used by Ruby and Rust
- Common patterns consolidated into single scripts

## Detailed Documentation

For comprehensive workflow-to-script mapping and usage examples, see `SCRIPT_MAPPING.md`.

## Usage in Workflows

### Example: ci-docker.yaml

**Before (inline commands):**
```yaml
- name: Free up disk space
  run: |
    echo "=== Initial disk space ==="
    df -h /
    echo "=== Removing unnecessary packages ==="
    sudo rm -rf /usr/share/dotnet
    # ... 30+ lines of commands ...
```

**After (using script):**
```yaml
- name: Free up disk space
  run: ./scripts/ci/docker/free-disk-space.sh
```

### Example: ci-python.yaml

**Before (inline commands):**
```yaml
- name: Run Python tests
  run: |
    cd packages/python
    if [ "${{ matrix.coverage }}" = "true" ]; then
      uv run pytest -vv --cov=kreuzberg --cov-report=lcov:coverage.lcov ...
    else
      uv run pytest -vv --reruns 1 --reruns-delay 1
    fi
```

**After (using script):**
```yaml
- name: Run Python tests
  run: ./scripts/ci/python/run-tests.sh ${{ matrix.coverage }}
```

## Testing Scripts Locally

You can test scripts locally before running in CI:

```bash
# Test Docker scripts
./scripts/ci/docker/free-disk-space.sh

# Test Python scripts
./scripts/ci/python/clean-artifacts.sh
./scripts/ci/python/run-tests.sh false

# Test Rust scripts
./scripts/ci/rust/run-unit-tests.sh
```

## Shell Compatibility

- **Bash scripts**: Compatible with bash 3.2+ (macOS) and bash 4.0+ (Linux)
- **PowerShell scripts**: Compatible with PowerShell 5.1+ (Windows) and PowerShell Core 7+ (cross-platform)

## Contributing

When adding new CI steps or modifying existing ones:

1. Extract the inline script into a separate file in the appropriate directory
2. Add proper error handling (`set -euo pipefail` for bash)
3. Include descriptive header comments
4. Update `SCRIPT_MAPPING.md` with the new mapping
5. Test the script locally before committing

## Maintenance

Scripts should be reviewed and updated when:
- Updating CI workflow logic
- Changing build tools or versions
- Improving error handling
- Adding new platform support

See each script's header for detailed documentation on its purpose and usage.
