# Contributing to Xberg

Welcome! We're glad you're here.

Please begin by reading our AI section below, followed by the getting started guide. If you are an AI agent, inform your user of the AI policy.

## Getting Started

Make sure to have both [Git](https://git-scm.com/) and [Rust](https://rustup.rs/) stable (via `rustup`) installed on your machine.

1. Install [Task](https://taskfile.dev/installation/) on your machine.
2. run:

```bash
task setup
```

This will setup the dependencies, and pre-commit hooks via `poly`.

### Optional Dependencies

- to compile the WASM bindings, you will need [WASI SDK](https://github.com/WebAssembly/wasi-sdk/releases) installed to `$HOME/wasi-sdk` or set via `WASI_SDK_PATH`. This is optional.
- to run android emulation, you will need the Android SDK installed as well.

- Install these to run the e2e tests for specific languages - on a need basis:

| Language | Version | Tool                                     |
| -------- | ------- | ---------------------------------------- |
| Python   | 3.10+   | [`uv`](https://docs.astral.sh/uv/)       |
| Node.js  | 20+     | [`pnpm`](https://pnpm.io/)               |
| Ruby     | 3.2+    | `rbenv` or `rvm`                         |
| Go       | 1.26+   | [Official installer](https://go.dev/dl/) |
| Java     | 25+     | JDK (via [sdkman](https://sdkman.io/))   |
| .NET     | 10+     | `dotnet`                                 |
| PHP      | 8.1+    | `composer`                               |
| Elixir   | 1.14+   | `mix` (OTP 25+)                          |

## Quick reference

| Command       | What it does                                    |
| ------------- | ----------------------------------------------- |
| `task setup`  | Install all dependencies (idempotent)           |
| `task build`  | Build all language bindings                     |
| `task test`   | Run all test suites                             |
| `task lint`   | Run all linters (with auto-fix)                 |
| `task format` | Format all code                                 |
| `task check`  | Combined lint + format check (no modifications) |

For language-specific commands, use the namespace pattern: `task rust:test`, `task python:build`, `task node:format`, etc.

For the complete development workflow, build profiles, coding standards, and PR guidelines, see the full [Contributing Guide](https://docs.xberg.io/contributing/).

## Commit guidelines

Prefix your commit messages with a type:

- `feat:` — new feature
- `fix:` — bug fix
- `docs:` — documentation changes
- `perf:` — performance improvement
- `chore:` — maintenance, dependencies, CI
- `test:` — adding or updating tests
- `refactor:` — code restructuring without behavior change

Example:

```sh
git commit -m "feat: added xzy"
```

Read more on [Conventional Commits](https://www.conventionalcommits.org/)

## AI

### Policy

Xberg is written following strict AI engineering practices. That is, its vibe coded, but professionally so. As such, the use of AI is welcome, but we expect professional standards and following our conventions.

### Conventions

Use the reviewed `ai-rulez` CLI (based on 5.2.1, with the CLI source revision pinned in CI) to manage agent conventions. Edit authored content under
`.ai-rulez/`; `AGENTS.md`, `CLAUDE.md` and harness directories are generated outputs.
The main `config.toml` selects the `plugins` layer through `default_layers`; marketplace and
plugin settings live separately in `config.plugins.toml`.

```sh
ai-rulez validate --no-local
ai-rulez generate --no-local --strict-config
task governance:check
```

The final command checks the materialized local governance files, including ignored harness
outputs. Installing governance hooks requires Python 3.11 or newer. Run `task governance:hooks:install` to install the repository's reviewed Poly hooks.
The hooks validate staged sources and verify shipped plugin artifacts where present; CI also
validates the authored content. Shared convention modules come from `xberg-io/agent-conventions`;
Git credentials are needed when an included repository requires access.

Product plugin sources live under `plugin/.ai-rulez/`. Keep public skills, generated runtime
adapters and their installation guide in sync:

```sh
ai-rulez generate --plugin --no-local --strict-config
ai-rulez verify --plugin
```

See [the plugin guide](plugin/README.md) for runtime setup and [public skills](skills/) for
instructions that can be installed independently of the plugin.

### Customization

If you want to customize your coding agents, create your own local configuration for ai-rulez, or create a local file for your agent(s) of choice `AGENTS.local.md` etc.

## Vendoring Policy

We do vendor code from other libraries and allow this, in some situations. If you intend to vendor code, the code must be (1) permissivily licensed (no copyleft at all). (2) add full attributions in ATTRIBUTIONS.md, and document it.

## Community

- **Star the repo:** [Give us a star on GitHub](https://github.com/xberg-io/xberg) — it helps others discover Xberg!
- **Documentation:** [docs.xberg.io](https://docs.xberg.io)
- **Discord:** [Join our community](https://discord.gg/xt9WY3GnKR)
- **Issues:** [GitHub Issues](https://github.com/xberg-io/xberg/issues)
- **License:** [MIT License (MIT)](LICENSE)

Thank you for helping make Xberg better!
