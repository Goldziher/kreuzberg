---
type: Playbook
title: Pdf Backends
description: Change or diagnose Xberg PDF extraction, native/Pdfium backend selection, PDF rendering sessions, encrypted documents, OCR fallback, or backend-specific capability gaps. Load for PDF engine work, not generic image OCR.
x-ai-rulez:
  kind: skill
  id: pdf-backends
  metadata:
    name: pdf-backends
---

# PDF backends

`PdfConfig.backend` selects `PdfBackend::Native` (default) or `PdfBackend::Pdfium`. Backend choice is explicit: never
silently return native output when Pdfium was requested.

## Native backend

- The native engine is `xberg_native_pdf` and supports the full Xberg PDF pipeline: text and structure extraction,
  page rendering/OCR fallback, tables, annotations, images, attachments, and richer metadata.
- `config.force_ocr` forces the whole document; `config.force_ocr_pages` selects pages. Otherwise, pages without usable
  native text can enter the OCR stage inside `PdfExtractor`; OCR is not a second PDF extractor in the registry.
- `PdfRenderSession` in `pdf/render.rs` opens document bytes once and exposes page count and page rendering. Use it for
  repeated page renders instead of reopening the same document for each page. It is a Rust primitive and need not be
  exposed by every binding.

## Pdfium backend

- Pdfium requires the `pdf-pdfium` Cargo feature. Selecting it without that feature must return an actionable error.
- Runtime dynamic loading honors `PDFIUM_DYNAMIC_LIB_PATH` as a directory containing the platform library. Build-time
  static and dynamic link variables are not interchangeable with runtime discovery.
- Pdfium binding is process-global and operations are serialized by the engine lock. Preserve that invariant when
  changing concurrency.
- The Pdfium implementation intentionally has a narrower capability surface: text, page count, and Info-dictionary
  metadata. It warns about omitted native capabilities rather than pretending parity.

## Verification

Test backend selection separately from document semantics. A missing Pdfium runtime is infrastructure failure unless
the job promises to provision it; when `XBERG_REQUIRE_PDFIUM` is set, skipping because the library or fixture is
absent is a failure. Use corpus fixtures through the `test-corpus` skill and include Pdfium artifacts in benchmark
aggregation when the adapter is enabled.
