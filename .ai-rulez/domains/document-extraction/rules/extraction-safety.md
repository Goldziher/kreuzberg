---
type: Decision
title: Extraction Safety
x-ai-rulez:
  kind: rule
  id: extraction-safety
  domain: document-extraction
  metadata:
    priority: critical
---

- Always use `SecurityLimits` to cap archive size, compression ratio, file count, and nesting depth for user content. Use `ZipBombValidator` for archive extraction.
- `SecurityLimits` also carries `max_entity_length`, `max_content_size`, `max_xml_depth`, `max_table_cells`, and `max_pages` (`extractors/security.rs`). `max_pages` defaults to `None` and is the only guard on per-page OCR/layout work — byte limits do not bound page count.
- Validate MIME type before extraction — never trust file extensions alone (`core/mime.rs::validate_mime_type`)
- The extractor chain falls back ONLY for fallback-eligible failures: `is_extractor_fallback_eligible` (`core/extractor/file.rs`) matches `UnsupportedFormat` and `Plugin` and nothing else. A `Parsing`, `Io`, `Ocr`, or `Validation` error aborts the chain by design — do not look for a second extractor's attempt when debugging one. A successful fallback records an `"extractor-fallback"` `ProcessingWarning` naming the extractor that ran.
- Preserve partial results on failure — return what was extracted with error context. Applied unevenly: honoured in the HWP and PPTX container paths, explicitly not in `enrich.rs` (which drops the partial result on any error).
- Errors carry a message and an optional `#[source]` chain — that is all. `XbergError` has no `suggestion` field; put the remedy in the message text.
- Never expose internal file paths or system details in error messages returned to users
