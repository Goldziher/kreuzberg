---
type: Playbook
title: Chunking Embeddings
description: Chunking, embeddings, and RAG pipeline integration
x-ai-rulez:
  kind: skill
  id: chunking-embeddings
  metadata:
    priority: critical
    name: chunking-embeddings
---

# Chunking & Embeddings

**Text splitting, ONNX/static embedding generation, RAG pipeline integration**

Locations: `crates/xberg/src/chunking/` and `crates/xberg/src/embeddings/` (both directories,
not single files).

## Chunking

`ExtractionConfig.chunking: Option<ChunkingConfig>` drives it. The standalone entry points are
`chunking::chunk_text(text, &ChunkingConfig, page_boundaries) -> Result<ChunkingResult>`
(`chunking/core.rs`) and `chunking::rag::chunk_for_rag(text, &ChunkingConfig)`
(`chunking/rag.rs`), which upgrades `ChunkerType::Text` to `Markdown` and fills each chunk's
`heading_path`.

`ChunkingResult { chunks: Vec<Chunk>, chunk_count: usize }`. `Chunk` carries `content`,
`chunk_type`, `metadata`, and the optional vectors `embedding`, `sparse_embedding`,
`late_interaction` (`types/extraction.rs`).

### `ChunkerType` — there is no strategy enum beyond this

`Text` (default), `Markdown`, `Yaml`, `Semantic` (`core/config/processing.rs`).
`Semantic` splits at embedding-based topic shifts when an `EmbeddingConfig` is present, and
falls back to a structural-boundary heuristic otherwise — `topic_threshold` has no effect on
the fallback path.

### `ChunkingConfig` fields and their serde wire names

| Field | Wire name (config file) | Default |
| --- | --- | --- |
| `max_characters` | `max_chars` (alias `max_characters`) | 1000 |
| `overlap` | `max_overlap` (alias `overlap`) | 200 |
| `trim` | `trim` | true |
| `chunker_type` | `chunker_type` | `Text` |
| `preset` | `preset` | none |

The renames are load-bearing: a config file that writes `max_characters` works only via the
alias, and a typo'd key is silently ignored (see `config-loading-precedence`).

### Presets set chunk size AND the embedding model

`ChunkingConfig.preset` resolves through `resolve_preset()`, which is
`#[cfg(feature = "embeddings")]`-gated — **without that feature it is a no-op and the preset
name does nothing**. A preset overrides `max_characters` and `overlap` and, if no embedding
config was given, selects the model.

| Preset | chunk_size | overlap | dims | backend |
| --- | --- | --- | --- | --- |
| `fast` | 512 | 50 | 384 | ONNX |
| `balanced` | 1024 | 100 | 768 | ONNX |
| `quality` | 2000 | 200 | 1024 | ONNX |
| `multilingual` | 1024 | 100 | 768 | ONNX |
| `gte-modernbert-base` | 1024 | 100 | 768 | ONNX |
| `lightweight` | 512 | 50 | 256 | static (model2vec) |
| `arctic-embed-m-v2.0` | 1024 | 100 | 768 | ONNX |
| `qwen3-embedding-0.6b` | 2000 | 200 | 1024 | ONNX |

Source of truth: `EMBEDDING_PRESETS` in `crates/xberg/src/embeddings/mod.rs`.

## Embeddings

There is no `TextEmbeddingManager`, no `embed_chunks()`, no `ChunkWithEmbedding`, no
`RagDocument`, and **no fastembed dependency** — do not write code against any of those.

Model selection is `EmbeddingModelType`, a tagged enum (`core/config/processing.rs`):
`Preset { name }` (recommended), `Custom { … }` (HuggingFace ONNX), `Llm { … }`,
`Plugin { … }`.

Two defaults disagree and both are live: `EmbeddingModelType::default()` is the
`gte-modernbert-base` preset (what bindings and `#[serde(default)]` get), while
`EmbeddingConfig::default()` names `balanced` via `default_balanced_embedding_model()`. Read
the constructor you are actually going through before assuming which model runs.

`EmbeddingConfig` defaults: `normalize = true`, `batch_size = 32`,
`max_embed_duration_secs = Some(60)`, `max_sequence_length = None` (falls back to 512, capped
at the model's own `model_max_length`).

### Feature gating

```toml
embeddings = ["onnx-runtime", "dep:ndarray", "chunking", "tokio-runtime", "embedding-presets"]
```

`ort-bundled` (the default ORT linkage) **downloads** ONNX Runtime at build time — no system
install, no `ORT_DYLIB_PATH`. That variable matters only under `ort-dynamic`.

`static-embeddings` is the pure-Rust model2vec path and the only dense embedder available on
`no-ort-target` (WASM/Android). `embedding-presets` carries preset metadata alone and is
WASM-safe.

## Critical Rules

1. **Chunk before embedding** — vectors are attached per chunk, not per document.
2. **A preset without the `embeddings` feature is inert** — `resolve_preset()` is compiled out.
3. **Write serde wire names in config files** — `max_chars`/`max_overlap`, not the Rust field names.
4. **Degrade, don't fail** — a build without ORT should skip embeddings, not error.
5. **Normalize for cosine similarity** — `EmbeddingConfig.normalize` defaults to true; leave it on.

## Related Skills

- **extraction-pipeline-patterns** — text extraction preceding chunking
- **config-loading-precedence** — how `ChunkingConfig` is resolved and why typos are silent
- **feature-flag-policy** — `embeddings` vs `static-embeddings` vs `embedding-presets`
