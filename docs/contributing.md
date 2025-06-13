# Contributing to Kreuzberg

Thank you for considering contributing to Kreuzberg! This document provides guidelines and instructions for contributing to the project.

## Development Setup

1. Clone the repository:

    ```bash
    git clone https://github.com/Goldziher/kreuzberg.git
    cd kreuzberg
    ```

1. Create and activate a virtual environment:

    ```bash
    python -m venv .venv
    source .venv/bin/activate  # On Windows: .venv\Scripts\activate
    ```

1. Install development dependencies:
    Install uv

    ```bash
    pip install uv
    ```

    Then install dependencies:

    ```bash
    uv pip install -e ".[all]"
    ```

## Code Style

```bash
pre-commit run --all-files
```

## Running Tests

```bash
pytest
```
