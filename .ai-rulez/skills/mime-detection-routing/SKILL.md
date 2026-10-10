---
type: Playbook
title: Mime Detection Routing
description: MIME type detection and extractor routing in core/mime.rs — the FORMATS registry that EXT_TO_MIME and SUPPORTED_MIME_TYPES are derived from, the path-based and bytes-based detection functions, priority-based registry selection, wildcard MIME families, and the real procedure for adding a format. Load when adding a format, wiring an extractor to a MIME type, or debugging why a file routes to the wrong (or no) extractor.
x-ai-rulez:
  kind: skill
  id: mime-detection-routing
  metadata:
    name: mime-detection-routing
---

# MIME Detection & Routing

## Detection Flow

```text
Policy -> content and/or extension evidence -> validate_mime_type -> registry.get(mime) -> extractor
```

## Key Functions

| Function | Location | Behaviour |
| --- | --- | --- |
| `detect_mime_type(path, check_exists: bool)` | `core/mime.rs` | **Path-based only** — never reads bytes. Lowercased extension → `EXT_TO_MIME`, then tree-sitter extension detection (feature `tree-sitter`), then `mime_guess::from_path`. `check_exists` gates a file-existence check, not content inspection. |
| `detect_mime_type_from_bytes(bytes)` | `core/mime.rs` | Magic-number detection via the `infer` crate. The only content-sniffing entry point. |
| `validate_mime_type(mime)` | `core/mime.rs` | Parses the media type, matches its case-insensitive essence against `SUPPORTED_MIME_TYPES`, and returns the registered MIME spelling. Parameters such as `charset` do not affect extractor routing. It does **not** consult the extractor registry. |

## The FORMATS registry is the single source of truth

`FORMATS: &[FormatEntry { extensions, mime_type, aliases }]` in `core/mime.rs`.
`EXT_TO_MIME` and `SUPPORTED_MIME_TYPES` are `LazyLock`s **derived** from it by iteration —
there is no `m.insert` call site to add to, and hand-editing either is impossible.

The full registry publishes 106 formats, 140 unique extensions, and 53 aliases, verified by
`scripts/sync_supported_counts.py verify`. The published count constants describe that static
registry; runtime availability is its intersection with registered extractors. Extension lookup is
case-insensitive (the extension is lowercased before the map hit).

## Registry Selection

```rust
let registry = get_document_extractor_registry();          // plugins/registry/mod.rs
let guard = registry.read()?;
let extractor: Arc<dyn DocumentExtractor> = guard.get(mime_type)?;  // Result, not Option
```

`DocumentExtractorRegistry::get` (`plugins/registry/extractor.rs`) returns the
highest-`priority()` extractor for the MIME type, and returns `Err` — not `None` — when none
matches.

## Wildcard Support

An extractor may register a family: `"image/*"` matches `image/png`, `image/jpeg`, and so on
(prefix match on a registered type ending in `/*`).

## Adding a New Format

1. Add one `FormatEntry` to `FORMATS` in `crates/xberg/src/core/mime.rs`. `EXT_TO_MIME` and
   `SUPPORTED_MIME_TYPES` update automatically.
2. Run `scripts/sync_supported_counts.py sync` to update published count claims, then run its
   `verify` command.
3. Implement `InternalDocumentExtractor` (not `DocumentExtractor` — see
   `plugin-architecture-patterns`) with `supported_mime_types()` returning the MIME.
4. Register in `crates/xberg/src/extractors/mod.rs::register_default_extractors()`.

## Critical Rules

1. Call `validate_mime_type()` before extraction — but do not treat it as proof an extractor exists.
2. Extension lookup is case-insensitive.
3. `detect_mime_type` inspects no content. Extraction defaults to `PreferContent`, which performs
   bounded content inspection and falls back to a supported extension. Use `ContentOnly` when the
   filename must be ignored; use `TrustExtension` only for trusted sources.
4. A specific explicit MIME type is authoritative. `application/octet-stream` is the exception: it
   is a generic placeholder and triggers policy-based detection.
5. Never edit `EXT_TO_MIME` or `SUPPORTED_MIME_TYPES` — edit `FORMATS`.
