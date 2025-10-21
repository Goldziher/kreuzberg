# Contributing to Kreuzberg

Thank you for contributing to Kreuzberg!

## Setup

1. **Install uv** (fast Python package manager):

    ```bash
    curl -LsSf https://astral.sh/uv/install.sh | sh
    ```

1. **Clone and install**:

    ```bash
    git clone https://github.com/Goldziher/kreuzberg.git
    cd kreuzberg
    uv sync --all-extras --dev
    ```

1. **Install prek and hooks**:

    ```bash
    # Install prek using uv (recommended)
    uv tool install prek

    # Install git hooks
    prek install && prek install --hook-type commit-msg
    ```

## Development

### Commands

All commands run through `uv run`:

```bash
# Testing
uv run pytest                      # Run all tests
uv run pytest tests/foo_test.py    # Run specific test
uv run pytest --cov                # With coverage (must be ≥85%)

# Code quality
uv run ruff format                 # Format code
uv run ruff check                  # Lint
uv run ruff check --fix            # Auto-fix issues
uv run mypy                        # Type check

# Prek
prek run --all-files  # Run all checks manually

# Documentation
uv run mkdocs serve                # Serve docs locally
```

### Updating Pdfium Versions

Kreuzberg uses [pdfium](https://pdfium.googlesource.com/pdfium/) for PDF rendering. The versions are controlled via environment variables in the CI workflow:

1. **Check for new releases**:
   - Standard builds: [bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries/releases)
   - WASM builds: [paulocoutinhox/pdfium-lib](https://github.com/paulocoutinhox/pdfium-lib/releases)

2. **Update versions in `.github/workflows/ci.yaml`**:
   ```yaml
   env:
     PDFIUM_VERSION: "7455"           # Update this
     PDFIUM_WASM_VERSION: "7469"      # Update this
   ```

3. **Test locally**:
   ```bash
   # Set env vars and test build
   export PDFIUM_VERSION="7455"
   export PDFIUM_WASM_VERSION="7469"
   cargo build --release
   ```

4. **Fallback versions**: If env vars are not set, `crates/kreuzberg/build.rs` will attempt to fetch the latest version from GitHub API, or fall back to hardcoded versions.

### Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

- `feat: add new feature`
- `fix: resolve issue with X`
- `docs: update README`
- `test: add tests for Y`

## Pull Requests

1. Fork the repo
1. Create a feature branch
1. Make changes (tests, code, docs)
1. Ensure all checks pass
1. Submit PR with clear description

## Notes

- Python 3.10-3.13 supported
- System dependencies (optional): Tesseract, Pandoc
- Prek runs automatically on commit
- Join our [Discord](https://discord.gg/pXxagNK2zN) for help

## License

Contributions are licensed under MIT.
