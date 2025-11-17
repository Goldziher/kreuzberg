# Contributing to tesseract-rs

Thank you for your interest in contributing to tesseract-rs! This document provides guidelines and instructions for contributing.

## Commit Message Format

We use [Conventional Commits](https://www.conventionalcommits.org/) for our commit messages. This leads to more readable messages that are easy to follow when looking through the project history.

### Commit Message Structure

```text
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

### Types

- **feat**: A new feature
- **fix**: A bug fix
- **docs**: Documentation only changes
- **style**: Changes that do not affect the meaning of the code (white-space, formatting, etc)
- **refactor**: A code change that neither fixes a bug nor adds a feature
- **perf**: A code change that improves performance
- **test**: Adding missing tests or correcting existing tests
- **build**: Changes that affect the build system or external dependencies
- **ci**: Changes to our CI configuration files and scripts
- **chore**: Other changes that don't modify src or test files
- **revert**: Reverts a previous commit

### Examples

```text
feat: add support for async OCR operations

fix: resolve Windows build issues with HOME environment variable

docs: update README with Windows installation instructions

test: add unit tests for error handling

ci: add commitlint to PR validation
```

### Scope

The scope should be the name of the module affected (as perceived by the person reading the changelog generated from commit messages).

Examples:

- `build`
- `api`
- `error`
- `tests`

### Subject

The subject contains a succinct description of the change:

- Use the imperative, present tense: "change" not "changed" nor "changes"
- Don't capitalize the first letter
- No dot (.) at the end

### Body

The body should include the motivation for the change and contrast this with previous behavior.

### Footer

The footer should contain any information about Breaking Changes and is also the place to reference GitHub issues that this commit closes.

## Setting Up Commit Hooks

We use [Prek](https://github.com/j178/prek) to manage pre-commit and commit-msg hooks. Prek gives us the same validation locally that CI performs.

1. Install [uv](https://docs.astral.sh/uv/) if you don't already have it:
   ```bash
   curl -LsSf https://astral.sh/uv/install.sh | sh
   ```

2. Install Prek and set up git hooks:
   ```bash
   uvx prek install
   ```

This will install a `commit-msg` hook enforcing Conventional Commits and a `pre-commit` hook that runs the same checks as `prek run --all-files` (formatting, clippy, Cargo checks, etc.).

If checks fail locally, fix the issues and re-run `uvx prek run --all-files`.

## Pull Request Process

1. Fork the repository and create your branch from `master`
2. If you've added code that should be tested, add tests
3. Ensure the test suite passes with `cargo test`
4. Make sure your code follows the Rust style guide with `cargo fmt`
5. Ensure there are no clippy warnings with `cargo clippy`
6. Update the README.md with details of changes if applicable
7. Create a Pull Request with a clear title and description

## Development Setup

### Prerequisites

Make sure the following tools are available before you start:

- Rust 1.85.0 or newer (the project targets the 2024 edition)
- A C++ compiler (e.g. clang, gcc, or MSVC)
- CMake (required by the build script)
- Internet connectivity for downloading Tesseract and Leptonica sources plus training data
- [uv](https://docs.astral.sh/uv/) for running Prek and managing Python tools
- Optional: [sccache](https://github.com/mozilla/sccache). Export `RUSTC_WRAPPER=sccache` locally if you want to cache compiler outputs.

### Initial Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/cafercangundogdu/tesseract-rs.git
   cd tesseract-rs
   ```

2. Install uv (if not already installed):
   ```bash
   curl -LsSf https://astral.sh/uv/install.sh | sh
   ```

3. Set up git hooks with Prek:
   ```bash
   uvx prek install
   ```

4. Build the project:
   ```bash
   cargo build
   ```

5. Run the test suite:
   ```bash
   cargo test
   ```

## Code Style

- Follow the official [Rust Style Guide](https://github.com/rust-dev-tools/fmt-rfcs/blob/master/guide/guide.md)
- Use `cargo fmt` before committing
- Address all `cargo clippy` warnings
- Write descriptive variable and function names
- Add comments for complex logic
- Keep functions small and focused

## Testing

- Write unit tests for new functionality
- Ensure all tests pass before submitting a PR
- Add integration tests for significant features
- Test on Windows, macOS, and Linux if possible

## Questions?

Feel free to open an issue if you have any questions or need clarification on anything!
