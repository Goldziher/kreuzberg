---
type: Playbook
title: Plugin Architecture Patterns
description: Design, implement, or diagnose Xberg plugin traits, typed registries, priority collisions, lifecycle, native extractors, and Alef-generated Python plugin bridges. Load for plugin-system work, not ordinary extractor parsing.
x-ai-rulez:
  kind: skill
  id: plugin-architecture-patterns
  metadata:
    priority: critical
    name: plugin-architecture-patterns
---

# Plugin Architecture & Registration

## Plugin Types

| Type | Trait | Location |
| --- | --- | --- |
| Document extractor (binding-facing) | `DocumentExtractor: Plugin` | `plugins/extractor/trait.rs` |
| Document extractor (in-crate) | `InternalDocumentExtractor: Plugin` | `plugins/extractor/trait.rs` |
| OCR backend | `OcrBackend: Plugin` | `plugins/ocr.rs` (a file, not a directory) |
| Post processor | `PostProcessor: Plugin` | `plugins/processor/trait.rs` |
| Validator | `Validator: Plugin` | `plugins/validator/trait.rs` |
| Embedding backend | `EmbeddingBackend: Plugin` | `plugins/embedding.rs` |
| Reranker backend | `RerankerBackend: Plugin` | `plugins/reranker.rs` |
| Tokenizer backend | `TokenizerBackend: Plugin` | `plugins/tokenizer.rs` |
| Renderer | `Renderer: Plugin` | `plugins/renderer.rs` |

`Plugin` (`plugins/traits.rs`) is `Send + Sync` and requires `name()`; `version()`,
`initialize()`, `shutdown()`, `description()`, and `author()` have defaults. There is no
`'static` trait bound; registry-owned `Arc<dyn Trait>` supplies the necessary lifetime.

## Native Rust extractors implement `InternalDocumentExtractor`

`DocumentExtractor` is the binding-facing surface. In-crate extractors implement
`InternalDocumentExtractor` and get `DocumentExtractor` from a blanket impl. Implementing
`DocumentExtractor` directly in this crate is the wrong layer.

```rust
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl InternalDocumentExtractor for MyExtractor {
    async fn extract_content(&self, content: &[u8], mime_type: &str, config: &ExtractionConfig)
        -> Result<InternalDocument> { /* ... */ }

    fn supported_mime_types(&self) -> &[&str] { &["application/x-custom"] }
    fn priority(&self) -> i32 { 50 }
}
```

`extract_path` has a default that reads the file and delegates to `extract_content` (and
errors without `tokio-runtime`).

Always use the two-arm `cfg_attr` form for `async_trait`. A bare `#[async_trait]` does not
match the trait declaration on `wasm32`.

The public trait has exactly four items — `extract`, `supported_mime_types`, `priority`,
`can_handle`. There is no `as_sync_extractor`; writing one is a compile error. WASM sync
support is the separate `SyncExtractor` trait — see `wasm-constraints`.

## Priority System

| Range | Use |
| --- | --- |
| 0-25 | Fallback/low-quality |
| 26-49 | Alternative extractors |
| **50** | **Default (built-in)** |
| 51-75 | Premium/enhanced |
| 76-100 | Specialized/high-priority |

The registry selects the **highest priority** extractor for each MIME type. The ranges are
conventions over an unclamped `i32`; negative and values above 100 are representable. Equal
MIME and priority is a collision: the later registration replaces the earlier entry and
warns. Give competing plugins distinct priorities.

## Registration

```rust
// crates/xberg/src/extractors/mod.rs -> register_default_extractors()
let registry = get_document_extractor_registry();
let mut registry = registry.write();
registry.register(Arc::new(MyExtractor::new()))?;
```

Feature-gate optional formats:

```rust
#[cfg(feature = "office")]
{
    registry.register(Arc::new(DocxExtractor::new()))?;
    registry.register(Arc::new(PptxExtractor::new()))?;
}
```

## PostProcessor Pattern

```rust
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl PostProcessor for MyProcessor {
    async fn process(&self, result: &mut ExtractedDocument, config: &ExtractionConfig)
        -> Result<()> {
        result.content = process_content(&result.content);
        Ok(())
    }
    fn processing_stage(&self) -> ProcessingStage { ProcessingStage::Middle }
}
```

The enum is `ProcessingStage` and the accessor is `processing_stage()`. Stages:
`Early` (default) → `Middle` → `Late`. `process` takes `&mut ExtractedDocument`, not an owned
result.

## Critical Rules

1. All plugins **MUST be `Send + Sync`** — `Plugin` requires it.
2. In-crate extractors implement `InternalDocumentExtractor`, never `DocumentExtractor`.
3. Use the two-arm `cfg_attr` `async_trait` form on every plugin trait impl.
4. Feature-gate optional formats with `#[cfg(feature = "...")]` at the registration site.
5. Initialization is lazy via `ensure_initialized()` (`extractors/mod.rs`), called before first extraction.
6. Plugin names are kebab-case (e.g. `"pdf-extractor"`).
7. A new extractor struct needs `#[cfg_attr(alef, alef(skip))]` or the binding regen aborts — see
   `alef-generated-bindings`.

## Registry and lifecycle invariants

- Eight plugin types have eight process-global typed registries in `plugins/registry/mod.rs`.
  There is no universal `PluginRegistry`.
- Registries use `Arc<parking_lot::RwLock<_>>`. Their guards are not poisoned and
  `.read()`/`.write()` return guards directly.
- Extractor lookup is `HashMap<mime, BTreeMap<priority, entry>>`: exact MIME lookup is
  constant-time on the outer map; wildcard-family lookup scans registered MIME keys.
- Registration calls `initialize()` and rejects a plugin whose initialization fails.
  Registries support register, remove, clear, and `shutdown_all`; there is no hot reload.
- All eight plugin types can be registered from language bindings. Plugin interfaces are
  public APIs, so breaking changes follow the public compatibility policy.
- Return errors rather than panicking. Test lifecycle, collision/replacement, concurrent
  access, and failure paths with test doubles; use real backends for integration coverage.
  No dispatch-overhead benchmark exists unless one is explicitly added.

## Alef-generated Python bridge

The Python bridge is generated into `crates/xberg-py/src/lib.rs`; there is no hand-written
`plugins.rs`. Change Alef/configuration and regenerate rather than editing the bridge.

- PyO3 0.29 uses `Python::attach`. Async host calls enter Python from
  `tokio::task::spawn_blocking` and propagate the caller's `contextvars` context.
- Cache frequently accessed host data such as plugin names in Rust fields so infallible
  methods do not need repeated GIL acquisition. Do not assume `allow_threads` is in use.
- Every trait method return crosses the bridge through native extraction or JSON fallback.
  Crossing types therefore need `Serialize + Deserialize + Default`, including unit enums.
- Host exceptions become `XbergError::Other` with plugin and method context; the original
  Python exception type and traceback are not retained. Infallible methods can only warn and
  return `Default::default()`, so a default may indicate bridge failure rather than real data.
- Rust-side extractor plugin failures may use `XbergError::Plugin`, which is fallback-eligible;
  do not assume Python bridge errors have the same fallback behavior.
- Validate the Python protocol at registration. Do not quote GIL overhead without a current
  benchmark.
