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

1. **Install pre-commit hooks**:

    ```bash
    pre-commit install && pre-commit install --hook-type commit-msg
    ```

## Development

### Commands

All commands run through `uv run`:

```bash
# Testing
uv run pytest                      # Run all tests
uv run pytest tests/foo_test.py    # Run specific test
uv run pytest --cov                # With coverage (must be ≥95%)

# Code quality
uv run ruff format                 # Format code
uv run ruff check                  # Lint
uv run ruff check --fix            # Auto-fix issues
uv run mypy                        # Type check

# Pre-commit
uv run pre-commit run --all-files  # Run all checks manually

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

- Python 3.9-3.13 supported
- System dependencies (optional): Tesseract, Pandoc
- Pre-commit runs automatically on commit
- Join our [Discord](https://discord.gg/pXxagNK2zN) for help

## License

Contributions are licensed under MIT.
