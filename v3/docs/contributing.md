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
    uv sync --all-packages --all-extras --all-groups
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

- Python 3.10-3.14 supported (note: EasyOCR, PaddleOCR, and entity extraction extras remain unavailable on 3.14 until upstream wheels support it)
- System dependencies (optional): Tesseract, Pandoc
- Prek runs automatically on commit
- Join our [Discord](https://discord.gg/pXxagNK2zN) for help

## License

Contributions are licensed under MIT.
