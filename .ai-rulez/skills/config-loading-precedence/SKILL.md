---
type: Playbook
title: Config Loading Precedence
description: How Xberg resolves configuration — CLI-mode and server/MCP-mode precedence orders, project config walk-up, user config fallback, field-level inline JSON merge (merge_json_into_config), the ExtractionOverrides CLI layer, and the two mechanisms that make a config change silently do nothing. Load when adding a config flag or env var, changing config precedence, or debugging why a setting is or isn't taking effect.
x-ai-rulez:
  kind: skill
  id: config-loading-precedence
  metadata:
    name: config-loading-precedence
---

# Configuration Loading & Precedence

## CLI Mode Precedence (highest to lowest)

1. Individual CLI flags (`--ocr`, `--output-format`, `--chunk`)
2. Inline JSON config (`--config-json` or `--config-json-base64`)
3. Config file (`--config path.toml`)
4. Auto-discovered config (`xberg.{toml,yaml,yml,json}` in cwd/parents, then the user config dir)
5. Default values

## Server/MCP Mode Precedence

1. CLI arguments (`--host`, `--port`)
2. Environment variables (`XBERG_HOST`, `XBERG_PORT`)
3. Config file `[server]` section
4. Defaults (`127.0.0.1:8000`)

## Config File Discovery

`ExtractionConfig::discover()` (`core/config/extraction/loaders.rs`) uses the same deterministic
format precedence at every level:

1. Walks the current directory and its parents, probing `xberg.toml`, `xberg.yaml`,
   `xberg.yml`, then `xberg.json` in each directory. The first hit wins.
2. If that finds nothing, falls back to the per-user global config directory
   (`dirs::config_dir()/xberg`) and probes four basenames in a fixed order:
   `xberg.toml`, `xberg.yaml`, `xberg.yml`, `xberg.json`.

## Inline JSON Config

Field-level merge (not whole-object replacement):

```rust
fn merge_json_into_config(base: &ExtractionConfig, json: Value) -> Result<ExtractionConfig> {
    let mut config_json = serde_json::to_value(base)?;
    // Merge fields from json into config_json
    serde_json::from_value(merged)?
}
```

Use `--config-json-base64` for shell escaping.

## Config File Formats

**TOML** (`xberg.toml`):

```toml
use_cache = true
[ocr]
backend = "tesseract"
languages = ["eng", "deu"]
[security_limits]
max_archive_size = 524288000
```

**YAML** and **JSON** follow equivalent structure.

## CLI Flag Overrides

`crates/xberg-cli/src/commands/overrides.rs`: the `ExtractionOverrides` struct's `validate()`
runs first, then `apply(self, config: &mut ExtractionConfig)` lays individual CLI flags over
the merged config. There is no `apply_extraction_overrides()` and no `commands.rs` —
`commands/` is a directory.

Inline JSON enters through `apply_json_overrides(config, config_json, config_json_base64)`
(`crates/xberg-cli/src/input.rs`), which delegates to `merge_json_into_config`.

## Two silent no-op mechanisms

**Duplicate `Default` impls.** `TesseractConfig` is defined twice — public
(`types/formats.rs`, binding-friendly types) and internal (`ocr/types.rs`, engine-side types)
— bridged by a `From` impl, each with its own `Default`. Changing one default fixes only the
routes that materialise that struct. Before changing any config default, grep the **bare**
type name (a `pub use module::*;` makes a qualified path unsearchable) and check for a second
definition.

**Unknown keys are ignored.** `#[serde(deny_unknown_fields)]` is on exactly two structs:
`ExtractionConfig` (`core/config/extraction/core.rs`) and `UrlExtractionConfig`
(`core/config/extraction/types.rs`). Every nested config silently ignores a typo'd key, so a
wrong setting parses clean, does nothing, and the test still passes. Write config fixtures
against the serde **wire** names (`ChunkingConfig` declares `max_characters` but renames to
`max_chars`), and assert the parsed config differs from the default rather than that it parsed.

## Critical Rules

1. CLI flags always win over config file
2. JSON merge is field-level, not whole-object
3. Auto-discovery probes TOML, YAML, YML, then JSON in each directory before walking to its parent
4. `--config-json-base64` for shell-safe JSON passing
5. Server config uses `[server]` section + extraction config
