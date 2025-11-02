## Structured extraction (draft)

This guide shows a tiny example of the proposed structured extraction API and usage. The API surface is a draft and may change as the feature is implemented and reviewed.

Overview
- Define a schema using Pydantic v2 `BaseModel` or `msgspec.Struct`.
- Provide a `StructuredExtractionConfig` (or set fields on a proposed `ExtractionConfig`).
- Call the structured extraction helper which uses a LiteLLM-compatible vision model to extract JSON matching the schema, validates it, and retries on validation failures.

Example (proposed Python usage)

```py
from pydantic import BaseModel

from kreuzberg.structured import StructuredExtractionConfig
from kreuzberg.exceptions import ExtractionValidationError, ExtractionError

# NOTE: extract_structured_sync is a proposed helper shown here for
# documentation purposes. The real implementation may be exposed via
# the Rust bindings' ExtractionConfig or a Python wrapper.


class Invoice(BaseModel):
    invoice_number: str
    date: str
    total: float


cfg = StructuredExtractionConfig(
    output_type=Invoice,
    extraction_model="litellm/vision-base",
    extraction_model_config={"device": "cpu"},
    max_extraction_retries=2,
    include_error_in_retry=True,
)


def extract_structured_sync(path: str, config: StructuredExtractionConfig):
    """Hypothetical wrapper shown for docs.

    The real function will live in the public API and may accept the
    schema via `ExtractionConfig` or as a separate argument.
    """
    raise NotImplementedError("This is a docs-only example; implementation pending")


try:
    result = extract_structured_sync("examples/invoice.pdf", cfg)
    print("Parsed structured result:", result)
except ExtractionValidationError as e:
    # Validation failed after retries; show details
    print("Validation failed:", e)
except ExtractionError as e:
    # General extraction failure (model, IO, etc.)
    print("Extraction error:", e)

```

Further notes
- The example intentionally shows a small, focused flow. The final
  implementation will include an adapter to LiteLLM vision models, schema
  parsing for both `msgspec.Struct` and Pydantic v2 `BaseModel`, and a
  configurable retry loop that can include validation errors in retry prompts.
