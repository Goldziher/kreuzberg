---
title: "Draft: Structured extraction using vision models"
labels: enhancement
assignees: []
---

# Draft PR: Structured extraction using vision models

This draft PR proposes adding structured data extraction powered by LiteLLM-compatible vision models. It's intentionally a design + scaffold so maintainers can comment on the approach before a full implementation is developed against the v4 branch.

## High-level description

- Allow users to pass an output schema (either `msgspec.Struct` or Pydantic v2 `BaseModel`) and have the library extract structured data from documents using vision-capable LLMs.
- Integrate with LiteLLM for vision models and allow pass-through model options.
- Validate extracted output against the provided schema and retry with error feedback when validation fails.
- Expose configuration via the (extended) `ExtractionConfig` and environment variables.
- Provide helpful exceptions and diagnostics on failure.

## Checklist

- [x] Add exception classes: `ExtractionError`, `ExtractionValidationError` (stubbed / implemented in Python)
- [ ] Extend `ExtractionConfig` with the following fields (proposal / stub included):
  - `output_type`
  - `extraction_model`
  - `extraction_model_config`
  - `max_extraction_retries`
  - `include_error_in_retry`
- [ ] Implement structured extraction module (Python glue + runtime integration with Rust core where appropriate)
- [ ] Add dependency group: `structured = ["msgspec>=0.18.0", "pydantic>=2.0.0", "litellm>=1.0.0"]`
- [ ] Tests: unit tests for validation + retry logic (mocked LiteLLM), integration tests, support both msgspec and pydantic
- [ ] Documentation: API reference, examples, error handling docs, supported models list

## What this draft includes

- An exceptions update (already included in this branch) with `ExtractionError` and `ExtractionValidationError`.
- A lightweight Python stub module at `packages/python/kreuzberg/structured.py` documenting the intended config fields and showing a minimal `StructuredExtractionConfig` dataclass. This is intentionally not wired into the Rust bindings yet — it's a proposal for the Python surface and tests can be built around it.

## Usage example

See the tiny example usage guide (draft) added for review: `docs/guides/structured-extraction.md`.
This contains a short Pydantic example and a hypothetical `extract_structured_sync` usage pattern.

## Why a draft

The v4 work is ongoing and implementation details (Rust binding surface, config layout) may need to change. Opening a draft PR lets maintainers comment on overall API and design while I iterate on the implementation in small, reviewable steps.

## Next steps (once accepted for iteration)

1. Add the optional dependency group to `packages/python/pyproject.toml`.
2. Implement the runtime integration that calls LiteLLM vision models (with an adapter interface so tests can mock it).
3. Add schema parsing/validation for `msgspec.Struct` and Pydantic v2 `BaseModel`.
4. Wire into `ExtractionConfig` (either via Rust-binding changes or a Python wrapper) and support env var configuration.
5. Add comprehensive tests and documentation.

---

Maintainers: tag me on this draft for feedback; I can follow up with a small implementation PR that is easier to rework for v4.
