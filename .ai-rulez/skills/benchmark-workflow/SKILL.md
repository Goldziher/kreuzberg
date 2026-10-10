---
type: Playbook
title: Benchmark Workflow
description: Run, diagnose, or change Xberg extraction benchmarks, quality scoring, benchmark fixtures, artifact contracts, and independently sourced ground truth. Load for the Benchmarks workflow or benchmark-harness work, not ordinary unit tests.
x-ai-rulez:
  kind: skill
  id: benchmark-workflow
  metadata:
    name: benchmark-workflow
---

# Benchmark workflow

The benchmark system lives in `tools/benchmark-harness/`; the GitHub workflow is
`.github/workflows/benchmarks.yaml`. The workflow is dispatch-only, so it does not run on push or gate merges. Treat
a result as evidence for its exact commit SHA and inputs, not for newer local work.

## Ground-truth integrity

- Never use Xberg's own extractor output as benchmark ground truth. Use an independent source and record it in the
  fixture's `ground_truth.source` field (`manual`, `vision`, `pdf_text_layer`, `pandoc`, `python-docx`, and similar).
- Before blaming ground truth for a score, render or otherwise inspect the source document. If the derived `.md` or
  `.txt` disagrees with the source, fix the ground truth; if it agrees, investigate the extractor or metric.
- Use the fixture schema in `tools/benchmark-harness/README.md`, the generator at
  `tools/benchmark-harness/scripts/generate_markdown_gt.py`, and the harness `validate-gt` command implemented in
  `tools/benchmark-harness/src/validate_gt.rs`. Do not replace these with an ad-hoc conversion pipeline.
- A quality claim requires the same corpus, config, renderer, cache state, and metric on control and experiment.
  Disable or invalidate extraction and OCR caches before A/B runs whose output behavior changed.

## Diagnosing runs

- Separate infrastructure failures from extraction or quality failures. A missing backend library, absent fixture,
  malformed artifact, or runner setup error does not describe extractor quality.
- Inspect the per-adapter artifacts before the aggregate job. Aggregate contract failures may be caused by missing or
  unexpectedly named artifacts even when individual adapters ran.
- Compare accepted OCR pages before raw word counts. Rejected OCR pages contribute neither text nor structured
  paragraphs.
- Measure headings and lists using Markdown output. Plain output normalizes away list markers and cannot distinguish
  detection from rendering.
- Do not quote a coverage, latency, or quality threshold unless the workflow or harness currently enforces it.

When changing harness behavior, add focused tests for the report or artifact contract and run the task that exercises
the affected adapter before dispatching the remote workflow.
