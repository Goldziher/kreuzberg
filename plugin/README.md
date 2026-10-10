# xberg

Extract text, tables, metadata, and images from 107 document formats — PDF, Office, images with OCR, HTML, email, archives, academic — using the local `xberg` CLI in your agent.

<!-- ~keep TODO: add screenshot -->

## Install

### From the marketplace (recommended)

Pending review for official Claude marketplace.

Self-host:

```text
/plugin marketplace add xberg-io/xberg
/plugin install xberg@xberg
```

### Pi

From a checkout of this repository, run:

```bash
pi install ./plugin
```

Once a plugin release includes Pi support, use the shared OpenCode/Pi npm package:

```bash
pi install npm:@xberg-io/opencode-xberg
```

Pi discovers skills from `.pi/skills` and prompt templates from `.pi/prompts`.
Use `/xberg-extract` and `/xberg-ocr` to start a task.
Install the CLI separately using the instructions below. The Pi package does not
register MCP servers, hooks, or executable extensions. The MCP integration
sections below apply to clients with MCP support.

Git installs use the repository root's `package.json` resource paths. GitHub
hosts the checkout resources; the existing npm plugin release publishes the same
Pi content alongside the OpenCode adapter. No separate Pi release job is needed.

### Binary requirement

Install the `xberg` CLI:

```bash
brew install xberg-io/tap/xberg
```

Or run without a persistent install:

```bash
npx @xberg-io/xberg-cli --help
uvx --from xberg-cli xberg --help
```

Or download a prebuilt binary from the [latest GitHub release](https://github.com/xberg-io/xberg/releases/latest), or install the CLI from crates.io:

```bash
cargo install xberg-cli --features mcp
```

The Python (`xberg`) and Node (`@xberg-io/xberg`) packages are language SDKs/bindings, not the CLI. The prebuilt CLI binaries (brew, GitHub release) include the MCP server; a source build enables it with `--features mcp` (`mcp` is not in the CLI's default feature set).

OCR ships with Tesseract by default. Install language packs for non-English documents:

```bash
brew install tesseract-lang        # macOS
sudo apt install tesseract-ocr-*   # Debian/Ubuntu
```

## Skills shipped

| Skill | Trigger |
|-------|---------|
| **xberg** | Extract text, tables, metadata, and images from 107 document formats (PDF, Office, images, HTML, email, archives, academic) using Xberg. Use when writing code that calls Xberg APIs in Python, Node.js/TypeScript, Rust, or CLI. Covers installation, extraction (sync/async), configuration (OCR, chunking, output format), batch processing, error handling, and plugins. |
| **extracting-with-ocr** | Use when extracting text from scanned PDFs, photographed pages, or images that have no embedded text layer. Covers OCR backends, language packs, force-OCR, and performance tuning. |
| **extracting-tables** | Use when extracting tabular data from PDFs, spreadsheets, or images. Covers layout-aware table detection, table model selection, output formats (markdown / JSON cells), and known limits. |
| **chunking** | Use when splitting extracted text into chunks for LLM context windows or RAG ingestion. Covers chunk size, overlap, markdown/yaml/semantic chunkers, tokenizer-based sizing, and the standalone `chunk` command. |
| **extracting-keywords** | Use when extracting keywords (YAKE/RAKE), detecting document language, or generating embeddings for RAG and search. Covers the keyword config, `--detect-language`, and the standalone `embed` command. |
| **batch-extraction** | Use when extracting from many files at once with shared config, bounded parallelism, per-file overrides, and error recovery. Covers the `batch` command, `--file-configs`, `--max-concurrent`, and output layout. |
| **picking-a-format** | Use when choosing an output format for extracted documents — text, markdown, djot, html, or JSON. Maps consumer (LLM, parser, archive) to the right `--format` / `--content-format` pair. |

**Reference materials** (linked from the `xberg` skill):

| Reference | Content |
|-----------|---------|
| **CLI Reference** | All commands, flags, config precedence, exit codes |
| **Configuration Reference** | TOML/YAML/JSON formats, auto-discovery, env vars, full schema |
| **Supported Formats** | Format families, extensions, capabilities, and authoritative discovery commands |
| **Python API Reference** | All functions, config classes, plugin protocols, exact signatures |
| **Node.js API Reference** | All functions, TypeScript interfaces, worker pool APIs |
| **Rust API Reference** | All functions with feature gates, structs, Cargo.toml examples |
| **Advanced Features** | Plugins, embeddings, MCP server, API server, security limits |
| **Other Language Bindings** | Go, Ruby, Java, C#, PHP, Elixir, WASM, Docker |

## MCP tools

Run `xberg mcp` to start the MCP server over stdio. The server exposes 9 tools:

**Extraction:** `extract`, `extract_batch`, `detect_mime_type`

**Cache:** `cache_stats`, `cache_clear`, `cache_manifest`, `cache_warm`

**Metadata:** `list_formats`, `get_version`

The `extract` and `extract_batch` tools accept an optional `config` object to override defaults and a `response_format` of `json` (default) or `toon`. There is no separate `extract_bytes` tool — `extract`/`extract_batch` take an `input` object whose `kind` is `bytes` or `uri`. The MCP server is gated behind the CLI's `mcp` build feature.

## Configuration

Xberg auto-discovers `xberg.toml` from the current directory upward. Set config via:

1. **Environment variable**: `XBERG_CONFIG_JSON='{"output_format":"markdown"}'`
2. **Config file** (TOML): `xberg.toml` in cwd or a parent directory.
3. **CLI flag**: `xberg extract doc.pdf --content-format markdown`

See `skills/xberg/references/configuration.md` for the full schema and precedence rules.

## Examples

Extract a PDF to plain text and print it:

```text
xberg extract document.pdf
```

Extract with markdown formatting for LLM context:

```text
xberg extract report.pdf --content-format markdown
```

Extract tables from a spreadsheet as JSON:

```text
xberg extract data.xlsx --format json
```

## Versioning

The plugin version tracks the xberg core release (`Cargo.toml`). See [CHANGELOG.md](../CHANGELOG.md) for release notes.

## Maintaining the plugin

Edit `plugin/.ai-rulez` sources, then run these commands from the repository root
with a Pi-capable ai-rulez v5 binary installed:

```bash
task plugin:generate
task plugin:verify
task plugin:test
```

CI builds the generator from a pinned source revision and verifies the generated
bundles, OpenCode tools, and actual npm archive before publication. The shared
npm package publishes both OpenCode and Pi resources; Hermes uses its existing
PyPI release job. GitHub hosts the remaining plugin bundles and marketplace indexes.

## License

MIT. The skill content uses Elastic-2.0 references to the upstream [xberg](https://github.com/xberg-io/xberg) repository.

## See also

- **Marketplace**: this repo — `/plugin marketplace add xberg-io/xberg`
- **Upstream**: [xberg-io/xberg](https://github.com/xberg-io/xberg)
- **Related plugins**: [crawlberg](https://github.com/xberg-io/crawlberg)
