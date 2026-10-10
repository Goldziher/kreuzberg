---
type: Playbook
title: Ocr Pipeline And Quality
description: Change or evaluate Xberg OCR backends, preprocessing, caching, page acceptance, geometry, hOCR structure, table reconstruction, or cross-backend quality. Load for OCR behavior and A/B quality work, not ordinary PDF text extraction.
x-ai-rulez:
  kind: skill
  id: ocr-pipeline-and-quality
  metadata:
    name: ocr-pipeline-and-quality
---

# OCR pipeline and quality

OCR flows through preprocessing, backend execution, structured conversion, page acceptance, and caching. Backend
outputs are not interchangeable measurements.

## Backends and execution

- All backends implement `OcrBackend`. Tesseract is the default and is bound through the in-repo
  `crates/xberg-tesseract` C FFI crate; there is no `leptess` dependency.
- `OcrBackendType` is `Tesseract | PaddleOCR | Candle | Custom`. Sceptre is selected by name through `Custom`, not a
  dedicated enum variant.
- Run blocking OCR work through `tokio::task::spawn_blocking`, minimize runtime/FFI lock duration, and respect backend
  resource limits.
- Check `PageOrientationHandling` before assuming a backend handles rotated input.
- Validate ISO 639 language codes and required tessdata before execution. Language detection runs after extraction
  and does not choose traineddata automatically.

## Configuration and cache

- Public `types::formats::TesseractConfig` and internal `ocr::types::TesseractConfig` have independent defaults. Change
  both and keep their synchronization test passing.
- The OCR cache key combines a build-identity tag, image hash, backend, config hash, and output format. The config
  hash includes `TESSERACT_RESULT_SCHEMA_VERSION` and the ordered Tesseract variable set and carries no build identity
  of its own, but it is not the whole key: `OcrCache::generate_cache_key` prefixes `cache_version_tag()`, which hashes
  `CARGO_PKG_VERSION`, `CACHE_SCHEMA_VERSION`, `env!("XBERG_BUILD_ID")` and the debug/release bit. Two builds from
  different commits therefore never share entries.
- Bump `TESSERACT_RESULT_SCHEMA_VERSION` when unchanged image/config inputs can produce different output, or disable
  the cache for an A/B or revert check.
- Default preprocessing is 300 DPI, deskew, and Otsu binarization. Auto-rotation, denoise, contrast enhancement, and
  color inversion are off unless configured. Native PSM defaults to 3; WASM defaults to 6.

## Quality invariants

- Query `confidence_semantics()` before interpreting confidence. Never threshold `Uncalibrated` output using a
  Tesseract-derived scale.
- Tesseract font size is typography from hOCR `x_fsize`; Sceptre/Paddle font size is a geometric detection-box proxy.
  Do not compare or threshold them as the same quantity.
- `hocr_font_info=1` is required for Tesseract typography; without it font sizes fall back to 12 pt. Sceptre/Paddle do
  not provide hOCR style fractions.
- `accept_or_reject_ocr_page` can discard an entire page and its structured paragraphs. Compare accepted pages before
  word counts. A missing dictionary-invalid ratio is unknown, not zero.
- Precision is the scarce resource on the current benchmark corpus. Require an independently grounded F1 A/B for
  recall-oriented rewrites; use the `benchmark-workflow` skill.
- Measure structural output such as headings and lists in Markdown, not Plain.

## Tables and geometry

- OCR table detection clusters word bounding boxes; it does not detect ruled lines or re-OCR cells.
- Detect rows, merge words into cell tokens, then detect columns on the merged tokens. Use
  `reconstruct_table_with_columns`; detecting columns on raw words creates spurious columns for multi-word cells.
- The PDF OCR raster is normalized to MediaBox-oriented user space. OCR segments carry `rotation_degrees: 0.0`; code
  reasoning about `/Rotate` pages must carry page rotation explicitly.
- Preserve word-level bounding boxes, confidence, and reading order through hOCR parsing and validate a reconstructed
  grid before emitting Markdown.
