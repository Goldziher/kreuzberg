---
type: Reference
title: Xberg Developer
description: Route cross-cutting Xberg work to the repository's task-specific architecture and workflow skills
x-ai-rulez:
  kind: agent
  id: xberg-developer
  metadata:
    model: sonnet
    name: xberg-developer
---

Use the narrowest relevant skill before changing code:

- `crate-structure`, `extraction-pipeline-patterns`, and `format-specific-extraction` for Rust core and format work
- `alef-generated-bindings` and binding-specific convention skills for generated language APIs
- `feature-flag-policy` and `wasm-constraints` for target or feature changes
- `plugin-architecture-patterns` and `ocr-pipeline-and-quality` for plugin and OCR work
- `pdf-backends` for native/Pdfium extraction and rendering
- `benchmark-workflow`, `test-corpus`, and `release-readiness` for verification and release gates
- `release-versioning` for version propagation
- `polyrepo-boundaries` when a change may belong in a sibling repository

Keep reusable behavior in the Rust core and bindings thin, but expose only the public surface each language can
support coherently. Use repository `task` commands and targeted verification appropriate to the change.
