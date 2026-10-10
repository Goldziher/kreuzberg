---
type: Playbook
title: Wasm Constraints
description: WASM build constraints for the crates/xberg-wasm crate — the wasm-target feature set, no-tokio sync-only internal APIs, the crate-private SyncExtractor trait, the 2 MB HTML size limit, size-optimized build config (opt-level="z"), and the async-wrapper/sync-internal API pattern. Load when building for wasm32, adding or modifying a WASM-compatible extractor, or debugging WASM build/runtime failures.
x-ai-rulez:
  kind: skill
  id: wasm-constraints
  metadata:
    name: wasm-constraints
---

# WASM Build Constraints

## Overview

WASM target lives in `crates/xberg-wasm/`, built with wasm-bindgen over sync-only internal
APIs. Note that `crates/xberg-wasm/src/lib.rs` is Alef-generated — do not hand-edit it.

## Feature Flags

```toml
# crates/xberg/Cargo.toml
wasm-target = [
    "no-ort-target",
    "excel-wasm",
    "ocr-wasm",
    "layout-tract",
    "auto-rotate-tract",
    "ner-candle-wasm",
]
```

RT-DETR layout detection and PP-LCNet document orientation run through the pure-Rust `tract`
engine; weights are streamed in from JS, never fetched by Rust (`hf-hub`/`reqwest` are
native-only). Deliberately **no** tree-sitter: the 371-language grammar pack pushes the
browser `.wasm` past jsDelivr's 50 MB per-file cap.

## Critical Constraints

### 1. No Tokio Runtime

All operations must be synchronous internally. Use `#[cfg(not(feature = "tokio-runtime"))]`
paths.

### 2. Internal Sync Extractor Required

Every WASM-compatible built-in extractor must implement `SyncExtractor`
(`crates/xberg/src/extractors/mod.rs`). It is `pub(crate)`, so only in-crate extractors can
implement it — out-of-crate plugins cannot. This is not part of the public API; public callers
still use `extract` / `extract_batch`.

```rust
impl SyncExtractor for MyExtractor {
    fn extract_sync(&self, content: &[u8], mime_type: &str, config: &ExtractionConfig)
        -> Result<InternalDocument> { /* sync implementation */ }
}
```

There is no `as_sync_extractor()` method on `DocumentExtractor` — do not write one.

### 3. HTML Size Limit

```rust
// crates/xberg/src/extraction/html/stack_management.rs
pub const MAX_HTML_SIZE_BYTES: usize = 2 * 1024 * 1024;  // 2 MB — stack constraint
```

## Build Config

```toml
# crates/xberg-wasm/Cargo.toml
[lib]
crate-type = ["cdylib"]

# root Cargo.toml
[profile.release.package.xberg-wasm]
opt-level = "z"   # codegen-units = 1 comes from the global [profile.release]
```

## API Pattern

The generated surface exposes `async` wasm-bindgen functions over sync internals:

```rust
#[wasm_bindgen]
pub async fn extract(input: JsValue, config: JsValue) -> Result<WasmExtractionResult, JsValue>
```

Functions can be `async` for JS ergonomics; extraction underneath is synchronous.

## Critical Rules

1. **No tokio** — all operations synchronous.
2. **Implement `SyncExtractor`** for every WASM-compatible in-crate extractor.
3. **HTML capped at `MAX_HTML_SIZE_BYTES` (2 MB)** due to stack constraints.
4. **Size optimization** via `opt-level = "z"` on the package profile only.
5. **Gate WASM-specific code** with `#[cfg(target_arch = "wasm32")]`, and use the two-arm `cfg_attr` `async_trait` form on any plugin trait impl.
