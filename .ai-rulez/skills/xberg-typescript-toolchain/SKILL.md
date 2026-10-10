---
type: Playbook
title: Xberg Typescript Toolchain
description: Work on Xberg TypeScript or JavaScript packages with the repository's actual poly, pnpm, npm, Vitest, napi-rs, wasm-pack, and integration-package boundaries. Load for TS/JS tooling or package changes, not Rust-only binding generation.
x-ai-rulez:
  kind: skill
  id: xberg-typescript-toolchain
  metadata:
    name: xberg-typescript-toolchain
---

# Xberg TypeScript toolchain

These repository-specific facts override generic TypeScript conventions where they differ.

- Lint and format with `poly lint .` and `poly fmt .`. Poly embeds oxlint and oxfmt; do not install or invoke oxlint,
  oxfmt, Prettier, ESLint, or Biome directly, and do not add per-package lint scripts.
- Root `tsconfig.json` enables strict checking options and is a project-reference root for `e2e/node` and `e2e/wasm`.
  Confirm that the package being changed is included before treating a root typecheck as evidence.
- Testing uses Vitest with `@vitest/coverage-v8`. No coverage threshold is enforced unless one is added to current
  configuration.
- No runtime schema-validator dependency is guaranteed. Use existing boundary guards or propose a dependency
  explicitly rather than assuming Zod is available.
- The pnpm workspace uses the root lockfile. Packages under `integrations/node/` are outside that workspace and use
  their own npm lockfiles. Determine ownership before installing or updating dependencies.
- `integrations/node/{langchain,llamaindex}-xberg` use tsup. The Node binding is built by napi-rs and the WASM binding
  by wasm-pack; do not add an application bundler to either binding.
- Prefer `workspace:*` for new dependencies between packages that are actually in the pnpm workspace.
- JavaScript dependency auditing is manual unless a current workflow says otherwise; do not claim an audit ran merely
  because Rust or poly checks passed.
