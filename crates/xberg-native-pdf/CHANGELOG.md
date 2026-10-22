# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

`xberg-pdf-oxide` is a fork of [`yfedoseev/pdf_oxide`](https://github.com/yfedoseev/pdf_oxide)
(PDFOxide), licensed MIT. Upstream's pre-1.0.0 history — the overwhelming
majority of this crate's code — is not reproduced here; it lives in this repository's git history
and in the upstream project itself.

## [Unreleased]

### Changed

- **(pdf): Native font metrics retain their 16-bit glyph-count API with the updated font parser.**
  `TrueTypeFont::num_glyphs()` caps larger upstream counts at 65,535; width-table work stays bounded to that range.
  Font ascenders and descenders use normalized above/below-baseline signs.

## [1.0.1] - 2026-08-20

Vendoring-preparation release. Everything here is in service of moving this crate into the
`xberg` workspace as a path member, where each rough edge would become permanent.

This release carries breaking changes under a patch version deliberately. The crate is consumed
only by `xberg`, which has not yet shipped a release depending on it, so no published consumer can
be broken. Later releases follow semver normally.

### Breaking

- **`ObjectRef.gen` is now `ObjectRef.generation`.** `gen` is a reserved keyword in edition 2024.
  The serde key changes with it, from `"gen"` to `"generation"`.
- **The crate is edition 2024** and its MSRV stays 1.92.
- **`ttf-parser` is replaced by fontations (`skrifa`).** `xberg-ttf-parser` leaves the dependency
  graph entirely. The crate now contains no `unsafe` at all and carries `#![forbid(unsafe_code)]`,
  which is what lets a vendored member adopt the workspace lint set.
- **Ten of the twelve Cargo features are gone**, and their code is unconditional: `rendering`,
  `system-fonts`, `jpeg2000`, `icc`, `icc-qcms`, `legacy-crypto`, `logging`, `test-support`,
  `parallel`, and the `default` set. Only `cjk-render-fallback` and `cjk-form-fonts` survive,
  because they embed multi-megabyte fonts via `include_bytes!` and the consumer's wasm bundle sits
  inside a hard CDN size budget.
- **`ParallelExtractor` and `extract_all_text_parallel` are removed**, along with the `rayon`
  dependency. Nothing called them.
- **`NoOpBackend` and the `NoOpCmykRetarget` / `NoOpSrgbToCmykTransform` / `NoOpSrgbTransform`
  aliases are removed.** `ActiveIccBackend` is permanently `QcmsBackend`.
- **`log` is replaced by `tracing`.** 683 call sites across 50 files. `log` and `env_logger` are no
  longer direct dependencies. Consumers that were filtering this crate's `log` output must switch
  to a `tracing` subscriber; note that `fontdb` and `tiny-skia` still emit through `log`, so a
  `tracing_log::LogTracer` bridge is needed to see those.
- **`CertificateEncryption::build` now returns an error.** It previously returned `Ok` while
  emitting a `/Recipients` array of empty strings — a genuinely AES-encrypted document whose file
  key was never wrapped for any recipient, and therefore permanently undecryptable by everyone
  including the certificate holder. Public-key encryption is not implemented; it now says so
  instead of silently destroying data. Use password-based encryption.
- **`fontdb` is vendored** into `src/vendor/fontdb`, parsing through `skrifa` rather than
  `ttf-parser`. `fontdb`, `slotmap` and `memmap2` all leave the dependency graph. The point is the
  `unsafe`: fontdb's memory-mapped `Source::SharedFile` was its only unsafe code and nothing here
  ever constructed it. Family-name selection now keys off skrifa's BCP-47 language tags rather than
  `ttf_parser::Language` — equivalent for real-world English name records, not byte-identical.
- **The never-wired redaction pruners are removed**: `redaction::font_scrub`,
  `redaction::image_prune`, `redaction::path_prune`, and from `redaction::classify` the
  `Classification` enum, `classify` and `transform_bbox`. All were fully unit-tested and called by
  nothing. `Classification` also leaves the crate-root re-exports. Redaction removes text and
  scrubs the catalog; it does **not** subset embedded fonts, resample image samples under a region,
  or clip vector paths crossing one, and the module now says so rather than implying coverage it
  never had.
- **`lexer` and `parser` are crate-internal.** Neither was named by any consumer; they are the
  tokenizer and object reader the higher-level modules sit on. `lexer::tokens` is removed.
- **`parse_tounicode_cmap` can now fail.** It previously returned `Ok` for any input, so a corrupt
  /ToUnicode stream yielded an empty CMap indistinguishable from a font that legitimately maps
  nothing. A non-empty stream carrying none of `begincmap`, `beginbfchar`, `beginbfrange`,
  `begincodespacerange` or `beginnotdefrange` is now an error. A zero-length stream stays
  legitimately empty; every other malformation warns and keeps parsing.
- **`compute_owner_hash_r5` is removed** and `compute_owner_password_hash` now rejects revisions
  5 and above. Its output omitted `U[0..48]` and so did not conform to ISO 32000-2 Algorithm 8.
  The document-writing path never used it and is unaffected; callers wanting R5/R6 should use
  `compute_u_and_ue` and `compute_o_and_oe`.

### Added

- `#[tracing::instrument]` spans on the public entry points, with stable names: `pdf.open`,
  `pdf.from_bytes`, `pdf.extract_text`, `pdf.extract_spans`, `pdf.render_page`,
  `pdf.extract_fields`. Byte buffers are skipped; only cheap scalar fields are recorded. Span
  names and field keys are semver-relevant.

### Fixed

- **A rotated table's grid is now reported on the table's own axes.** Ruled-table detection buckets
  spans by physical page position, which is correct, but it then always labelled the top-to-bottom
  axis "rows" and the left-to-right axis "columns". A table drawn sideways therefore came out
  transposed, with what should have been one column spread across a header row. The finished table
  is now re-oriented when a strict majority of its spans agree on a 90, 180 or 270 degree quadrant;
  a tie or a disagreement is treated as upright, and the upright path is unchanged. Borderless
  tables, which are grouped by text clustering rather than ruling lines, are not yet covered.
- **Image masks now resolve indirect `/Width`, `/Height` and `/BitsPerComponent`.** ISO 32000-1
  §7.3.10 lets any dictionary entry be an indirect reference, and some scanner producers write the
  stencil dimensions that way. The renderer read them with a plain `as_integer()`, which returns
  nothing for a reference, so the mask was rejected as `ImageMask missing /Height` and the region
  painted blank — starving OCR of the page it was supposed to read. The non-mask image path already
  resolved these; the mask path and the separation-plate renderer now use the same helper.
- **Cross-document font-cache poisoning.** `font_identity_hash_cheap` folded every font attribute
  except the vertical-writing metrics, so two documents whose fonts differed only in `/W2` or
  `/DW2` collided in the global cache and the second silently inherited the first's vertical
  advances.
- **Form XObject glyph loss under skrifa.** skrifa's CFF outline path requires an `hmtx` table and
  reports a font as having no outline source without one. PDF-embedded CFF subsets routinely omit
  `hmtx` because advances live in the PDF `/W` array, so every glyph of such a font vanished;
  `hmtx` is now synthesised when a font genuinely has no outline source.
- **`Pdf::to_bytes` could return another document's bytes.** It serialised through
  `$TMPDIR/pdf_oxide_temp_<pid>.pdf`, a path keyed only on the process id, so two threads in one
  process wrote and read back the same file. The caller received a valid-looking PDF with the wrong
  content — undetectable downstream. It now uses the editor's in-memory `save_to_bytes`, the same
  `write_full_to_writer` path `save()` takes, which additionally works on wasm32 where `std::fs`
  cannot. `flatten_to_images` had the same collision on its scratch directory, plus a
  `remove_dir_all` that deleted a concurrent caller's pages mid-run.
- **A failed ToUnicode CMap parse is memoized.** `LazyCMap::get` sits on the per-character decode
  path and did not record failure, so a broken stream was re-parsed once per character rather than
  once per font. The resulting warning now fires once and names the font it belongs to; a document
  with thirty fonts previously gave no way to tell which lost its mapping.
- Unit and integration tests no longer build fixed paths under `std::env::temp_dir()`, so
  concurrent runs stop truncating each other's files.
- Repeated per-string warnings during decryption are aggregated to one event per object.

### Changed

- Logging levels were re-derived rather than mechanically renamed. Corruption signals a caller can
  act on — xref reconstruction, Catalog synthesis, structure-tree fallback to geometry, unsupported
  shading types, font substitution — were promoted to WARN. Events that fire per glyph, per
  operator or per path were demoted to TRACE regardless of severity, because a warning repeated
  thousands of times per page carries no information.
- `subsetter` builds with `default-features = false`, collapsing a duplicate fontations stack
  (`skrifa` 0.42 / `read-fonts` 0.39) that shadowed the crate's own. `jpeg-decoder` likewise, which
  is what removed `rayon` from the graph.

### Removed

- Dead code and inert configuration: the never-compiled `pipeline::input_parsers` (1,550 LOC), the
  unreferenced `extractors::debug_span_merging` (469 LOC), duplicated bundled fonts, and advisory
  ignores for crates this fork no longer has.
- Roughly 70 tests whose inputs existed only on a private machine, and which therefore either
  reported `ok` without asserting anything or could never run at all.
- Tracker numbers, stale pre-1.0 version citations, upstream marks, and the names of people and
  organisations used as fixture provenance, throughout the source and tests. Spec citations,
  producer names attached to a documented deviation, and reference implementations used as
  correctness baselines are kept, since a reader can still resolve those.
- The CodeQL workflow, and the `release-small` profile, which nothing referenced.


## [1.0.0] - 2026-08-18

This is the first release under the `xberg-pdf-oxide` name: a reduced fork of PDFOxide, stripped
to the Rust library that [`xberg`](https://github.com/xberg-io/xberg) consumes.

### Breaking

- **The Markdown, HTML and Office output converters are removed.** `PdfDocument`'s
  `to_markdown`, `to_html`, `to_plain_text`, `to_docx`, `to_pptx` and `to_xlsx` families (with their
  `_all`, `_bytes` and `_flow` variants), `PdfBuilder::to_markdown`/`to_html`, the
  `pipeline::converters` module and `parallel::extract_all_markdown_parallel` go with them, as does
  the `office_oxide` dependency. Extraction, rendering, table detection, forms, XFA and the
  reading-order pipeline are unaffected — use `extract_text`, `extract_tables` and
  `pipeline::TextPipeline` directly. `converters::ConversionOptions`, `TextPostProcessor` and the
  whitespace helpers remain, since extraction depends on them.
- **The crate and library name changed from `pdf_oxide` to `xberg-pdf-oxide` / `xberg_pdf_oxide`.**
  Every import path moves (`use pdf_oxide::...` → `use xberg_pdf_oxide::...`), and anything derived
  from `CARGO_PKG_NAME` — the `NAME` constant, `log` targets, the `/Creator` and XMP
  `CreatorTool`/`Producer` fields a written document carries, and the CBOM tool component — now
  reports `xberg-pdf-oxide` instead of `pdf_oxide`.
- The language bindings, C FFI layer, CLI binaries, and the Python and WASM modules are removed.
  This is now a Rust-only library crate.
- The OCR, digital-signature, `html_css` (HTML/CSS-to-PDF), PDF/A compliance, and hybrid extraction
  subsystems are removed, along with the unused barcode-writer stub and debug modules.

### Fixed

- Embedded PNGs are encoded with adaptive per-scanline filtering instead of `NoFilter`. Deflate
  alone cannot exploit a smooth gradient, so a 256×256 RGB ramp encoded to 196,947 bytes from
  196,608 raw samples — larger than its uncompressed input; the same image is now 761 bytes with
  byte-identical decoded pixels. Reported upstream by ajbufort.
- Numerals keep left-to-right order inside a right-to-left tagged table cell (UAX #9 class EN/AN);
  only the surrounding script reverses. A figure a producer split across marked-content runs was
  read back reversed — `47.500` became `7.5004`, altering the value rather than its spacing.
  Reported upstream by yfedoseev.
- The Form XObject `/BBox` clip (ISO 32000-1 §8.10.1) now prunes the character layer as well as
  spans, so `extract_chars` no longer reports out-of-BBox text that `extract_text` correctly drops.
  Reported upstream by yfedoseev.

### Added

- Depends on `xberg-ttf-parser` in place of upstream `ttf-parser`: a superset with zero API
  removals that additionally fixes glyphs silently dropped when a CFF font uses the deprecated
  `dotsection` operator (`i`, `j`, `.`, `:`, `;`, `!`, `?` in affected fonts).
- `XBERG_PDF_OXIDE_MAX_DECOMPRESS_MB` controls the Flate decompression bomb guard, falling back to
  the pre-rename `PDF_OXIDE_MAX_DECOMPRESS_MB` when unset, so existing deployments keep working
  across the rename.

### Changed

- The declared MSRV is now `1.92`, matching what the dependency graph actually requires
  (`hayro-jpeg2000`, pulled in by the `jpeg2000` feature that `rendering` enables); the prior
  `1.88` was already false for that feature combination.
- Dependencies upgraded to their latest versions, including major-version bumps, `fontdb` to
  0.24 and `harfrust` to 0.13.

[Unreleased]: https://github.com/xberg-io/xberg-pdf-oxide/compare/v1.0.1...HEAD
[1.0.1]: https://github.com/xberg-io/xberg-pdf-oxide/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/xberg-io/xberg-pdf-oxide/releases/tag/v1.0.0
