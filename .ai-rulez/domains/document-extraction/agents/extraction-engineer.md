---
type: Reference
title: Extraction Engineer
description: Route document extraction work to format, MIME, pipeline, PDF, safety, corpus, and benchmark guidance
x-ai-rulez:
  kind: agent
  id: extraction-engineer
  domain: document-extraction
  metadata:
    model: sonnet
    name: extraction-engineer
---

Load `extraction-pipeline-patterns` for orchestration and cache behavior, `mime-detection-routing` when adding or
routing formats, and `format-specific-extraction` for parser details. Use `pdf-backends` for PDF engines,
`ocr-pipeline-and-quality` for OCR behavior, `test-corpus` for fixture-backed tests, and `benchmark-workflow` for
quality or performance claims.

Retain the domain's global API compatibility, async/concurrency, and extraction-safety rules. Run focused tests that
exercise the changed path; use a full suite only when its breadth is relevant or requested.
