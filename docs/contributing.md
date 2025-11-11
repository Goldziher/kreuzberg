# Contributing Guide

Thanks for helping improve Kreuzberg! This guide summarizes the workflow, tooling, and expectations for contributions across all language bindings.

## Prerequisites

- Rust toolchain (`rustup`) with the stable channel.
- Python 3.10+ (we use `uv` for virtualenv and dependency management).
- Node.js 20+ with `pnpm`.
- Ruby 3.3+ via `rbenv` (preferred) or `rvm`.
- Homebrew (macOS) or system equivalents for Tesseract/Pdfium dependencies.

Install all project dependencies in one shot:

```bash
task setup
```

This runs language-specific installers (Python `uv sync`, `pnpm install`, `bundle install`) and builds the Rust workspace.

## Development Workflow

1. **Create a branch** off `main` with a descriptive name (e.g., `feat/python-config-alias`).
2. **Make changes** with small, focused commits. Code should compile on all supported platforms.
3. **Run tests/lint** for the areas you touched:
   - `task lint` – cross-language linters (cargo clippy, Ruff, Rubocop, Biome/Oxlint, Mypy).
   - `task dev:test` – full test matrix (Rust + Python + Ruby + TypeScript).
   - Language-specific shortcuts: `task python:test`, `task typescript:test`, `task ruby:test`, `task rust:test`.
4. **Write/Update docs** when adding features. User-facing content lives under `docs/` and must be referenced in `mkdocs.yaml`.
5. **Ensure conventional commits** (`feat: ...`, `fix: ...`, `docs: ...`). The pre-commit hook checks commit messages.
6. **Create a pull request** with a clear summary, screenshots/logs if relevant, and a checklist of tests you ran.

## Coding Standards

- **Rust**: edition 2024, no `unwrap` in production paths, document all public items, add `SAFETY` comments for unsafe blocks.
- **Python**: dataclasses use `frozen=True`, `slots=True`; function-based pytest tests; follow Ruff/Mypy rules.
- **TypeScript**: maintain strict types, avoid `any`, keep bindings in `packages/typescript/src` and tests under `tests/binding|smoke|cli`.
- **Ruby**: no global state outside `Kreuzberg` module, keep native bridge panic-free, follow Rubocop defaults.
- **Testing strategy**: Only language-specific smoke/binding tests live in each package; shared behavior belongs to the `e2e/` fixtures (Python, Ruby, TypeScript, Rust runners). When adding a new feature, update the relevant fixture and regenerate via `task e2e:<lang>:generate`.

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
