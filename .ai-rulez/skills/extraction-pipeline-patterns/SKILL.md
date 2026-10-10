---
type: Playbook
title: Extraction Pipeline Patterns
description: Change or diagnose Xberg's core extraction orchestration, cache semantics, extractor fallback, post-processing, concurrency defaults, or format-wide quality invariants. Load for pipeline work, not a single parser's syntax.
x-ai-rulez:
  kind: skill
  id: extraction-pipeline-patterns
  metadata:
    priority: critical
    name: extraction-pipeline-patterns
---

# Extraction Pipeline Patterns

**Format detection → extractor routing → post-processing, across 106 formats / 140 file extensions**

The full-registry counts are verified against published claims by
`scripts/sync_supported_counts.py verify`. Runtime `SUPPORTED_FORMAT_COUNT` and
`SUPPORTED_EXTENSION_COUNT` values are derived from the full static `FORMATS` registry.

## Layout

- `crates/xberg/src/core/pipeline/` — orchestration (`mod.rs`, `cache.rs`, `execution.rs`,
  `features.rs`, `format.rs`, `initialization.rs`, `page_markers.rs`)
- `crates/xberg/src/core/mime.rs`, `core/formats.rs` — detection and the `FORMATS` registry
- `crates/xberg/src/extractors/` — one module per format, each implementing
  `InternalDocumentExtractor`
- `crates/xberg/src/extraction/` — shared parsing/rendering helpers used by those extractors
- `crates/xberg/src/core/config/`, `core/config_validation/` — both directories, not files

## Flow

1. **Detect** — MIME from extension via `EXT_TO_MIME`, or from bytes via
   `detect_mime_type_from_bytes`; validate against `SUPPORTED_MIME_TYPES`.
2. **Route** — registry returns the highest-`priority()` extractor registered for that MIME.
3. **Extract** — the extractor produces an `InternalDocument`.
4. **Post-process** — `core::pipeline::run_pipeline(doc, config)` (async) or
   `run_pipeline_sync` (WASM) runs validators, quality processing, chunking and hooks, and
   returns `ExtractedDocument`. Every extraction path goes through it.

## Extractor modules

- Office: DOCX, PPTX, PPT, DOC, XLSX/XLS, ODT, ODP, iWork, HWP/HWPX, and WordPerfect under
  `extractors/{docx,pptx,ppt,doc,excel,odt,odp,hwp,hwpx,wordperfect}.rs` and `extractors/iwork/`.
- Markup: Markdown, text, RST, Org, RTF, AsciiDoc, Typst, and Djot under
  `extractors/{markdown,text,rst,orgmode,asciidoc,typst}.rs` and `extractors/{rtf,djot_format}/`.
- Academic: LaTeX, BibTeX, JATS, Jupyter, DocBook, EPUB, and FictionBook under
  `extractors/{bibtex,jupyter,docbook,fictionbook}.rs` and `extractors/{latex,jats,epub}/`.
- PDF: text, encrypted-document, and OCR-fallback handling under `extractors/pdf/`.
- Images: PNG, JPEG, TIFF, WebP, HEIC, SVG, and QR under `extractors/` and `extraction/`.
- Web: HTML, XHTML, XML, and MDX under `extractors/` and `extraction/html/`.
- Email: EML, MSG, and PST under `extractors/` and `extraction/email.rs`.
- Archives: ZIP, TAR, GZIP, and 7z under `extractors/archive.rs` and `extraction/archive/`.
- Structured: JSON, GeoJSON, YAML, TOML, CSV, DBF, SQLite, and GeoPackage under `extractors/`.

## Fallback strategies

- **Password-protected PDFs** — try the configured password, then the secondary list; on
  failure report `is_encrypted` in metadata rather than erroring out.
- **OCR fallback** — a PDF page with no extractable text routes to the OCR pipeline;
  `config.force_ocr` and `config.force_ocr_pages` force it.
- **Nested archives** — recursive with a depth limit from `SecurityLimits`.
- **Corrupted input** — emit what parsed and attach the error location; never panic.

The cross-extractor fallback chain runs only for `UnsupportedFormat` and `Plugin` errors as
defined by `is_extractor_fallback_eligible`. Parsing, IO, OCR, and validation errors abort the
chain. A successful fallback records an `extractor-fallback` processing warning.

## Cache and concurrency

- Extraction keys are `<cache_version_tag>-<content_hash>-<config_hash>`, never path-based.
  The tag hashes `CARGO_PKG_VERSION`, `CACHE_SCHEMA_VERSION`, `env!("XBERG_BUILD_ID")` and the
  debug/release bit (`cache/version.rs`); `XBERG_BUILD_ID` is the commit SHA plus a hash of any
  uncommitted diff (`build.rs`), so it IS a build fingerprint.
- Two builds from different commits therefore get different entries with no schema bump needed.
  The exception is a build that cannot reach git metadata and gets `XBERG_BUILD_ID = ""` -- a
  crates.io/docs.rs tarball (safe: immutable source per version) or a Docker build (`.dockerignore`
  excludes `**/.git/`, and nothing passes the `--build-arg` escape hatch). Bump
  `CACHE_SCHEMA_VERSION` when behavior can change and those builds must not collide. For A/B or
  revert checks, disable the cache or export distinct `XBERG_BUILD_ID` values per arm, so the
  experiment cannot replay the control.
- Configuration that affects output belongs in the config hash. Check the cache before
  extraction so a hit skips processing.
- Default batch concurrency uses host CPU count capped by any detected Linux cgroup quota via
  `core/config/concurrency.rs::resolve_thread_budget`.
- `core/io.rs::read_file_async` currently reads the whole file with `tokio::fs::read`; there
  is no `AsyncRead` extraction surface. Treat streaming as an open gap.
- Cache hit and miss OTel counters exist, but no hit-rate target is computed or enforced.

## Plugin integration

`crates/xberg/src/plugins/`. Registry selection is by **`priority()`, highest wins** — not by
registration order. Register above 50 to override a built-in. See
`plugin-architecture-patterns`.

## Features

All features live in `crates/xberg/Cargo.toml`; there is no `FEATURE_MATRIX.md`.

| Group | Features |
| --- | --- |
| OCR | `ocr`, `ocr-wasm`, `paddle-ocr`, `paddle-ocr-tract`, `sceptre-ocr`, `candle-vlm-ocr` |
| Formats | `pdf`, `office`, `excel`, `html`, `xml`, `email`, `archives`, and format-specific flags |
| AI/ML | `embeddings`, `static-embeddings`, layout, keywords, language detection, and NER flags |
| Server | `api` (Axum), `mcp`, `otel`, `prometheus`, `tokio-runtime` |
| Aggregates | `formats`, `analysis`, `services`, `full`, and platform target groups |

Bindings are separate crates, not features of `crates/xberg`. The one mutually-exclusive pair
is `ort-bundled` / `ort-dynamic`. WASM excludes ORT-backed `embeddings`, but **does** carry
`ocr-wasm`, `keywords` and `static-embeddings`. Full detail in `feature-flag-policy`.

## Critical Rules

1. **Detect before routing** — never dispatch on a caller-supplied extension alone.
2. **Post-processing is mandatory** — all results flow through `run_pipeline` / `run_pipeline_sync`.
3. **Selection is by priority, not registration order** — the registry returns the highest `priority()` for a MIME
   type.
4. **Do not claim streaming** — current file extraction reads the whole input into memory.
5. **Apply `SecurityLimits` to user content** — archive size, compression ratio, file count, nesting depth.
6. **Fail gracefully** — malformed input returns partial content plus error context, never a panic.

## Verification

Test the changed format categories and both success and failure paths. No coverage percentage
is an enforced contract. The format headline test is the enforced count; update its constants
and listed copy together with `FORMATS`. Use `benchmark-workflow` for performance or quality
claims and `test-corpus` for bucket-backed fixtures.

## Related Skills

- **mime-detection-routing** — the `FORMATS` registry and how to add a format
- **plugin-architecture-patterns** — which trait an extractor actually implements
- **format-specific-extraction** — per-format workflows and helpers
- **chunking-embeddings** — post-extraction text splitting
