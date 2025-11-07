from __future__ import annotations

import asyncio
import json
from typing import Any, Awaitable, Callable

from kreuzberg._internal_bindings import extract_structured_json as extract_structured_json_impl
from kreuzberg.exceptions import ValidationError


def _derive_schema_json(output_type: Any) -> str:
    # Try msgspec JSON Schema generation
    try:
        from msgspec import json as msgjson  # type: ignore
        schema_fn = getattr(msgjson, "schema", None)
        if callable(schema_fn):
            schema = schema_fn(output_type)  # type: ignore[call-arg]
            return json.dumps(schema)
    except Exception:
        pass

    # Try Pydantic v2
    try:
        if hasattr(output_type, "model_json_schema"):
            return json.dumps(output_type.model_json_schema())  # type: ignore[attr-defined]
    except Exception:
        pass

    raise ValidationError(
        "Cannot derive JSON Schema from output_type",
        context={"output_type": repr(output_type)},
    )


async def extract_structured_async(
    images: list[bytes],
    prompt: str,
    extractor: Callable[[list[bytes], str], Awaitable[str | bytes]] | Callable[[list[bytes], str], str | bytes],
    *,
    schema_json: str | None = None,
    output_type: Any | None = None,
    max_retries: int = 2,
    include_error_in_retry: bool = True,
) -> Any:
    """Extract structured data using a user-provided extractor and validate/deserialize.

    - Validates extractor output against `schema_json` (or derives it from `output_type`)
    - Returns typed object if `output_type` provided and msgspec/pydantic are available
    - Otherwise returns a Python dict/list parsed from JSON
    """
    if schema_json is None and output_type is None:
        raise ValidationError(
            "Provide either schema_json or output_type",
            context={"images": len(images)},
        )

    if schema_json is None and output_type is not None:
        schema_json = _derive_schema_json(output_type)

    raw: bytes = await extract_structured_json_impl(
        images,
        prompt,
        extractor,  # type: ignore[arg-type]
        schema_json,
        max_retries,
        include_error_in_retry,
    )

    if output_type is None:
        return json.loads(raw.decode("utf-8", errors="replace"))

    # Try msgspec first for performance
    try:
        import msgspec

        return msgspec.json.decode(raw, type=output_type)  # type: ignore[arg-type]
    except Exception:
        pass

    # Fallback to Pydantic v2
    try:
        if hasattr(output_type, "model_validate_json"):
            return output_type.model_validate_json(raw.decode("utf-8", errors="replace"))  # type: ignore[attr-defined]
    except Exception:
        pass

    # Final fallback: plain JSON
    return json.loads(raw.decode("utf-8", errors="replace"))


def extract_structured(
    images: list[bytes],
    prompt: str,
    extractor: Callable[[list[bytes], str], str | bytes] | Callable[[list[bytes], str], Awaitable[str | bytes]],
    *,
    schema_json: str | None = None,
    output_type: Any | None = None,
    max_retries: int = 2,
    include_error_in_retry: bool = True,
) -> Any:
    """Synchronous wrapper for `extract_structured_async`."""
    return asyncio.run(
        extract_structured_async(
            images,
            prompt,
            extractor,
            schema_json=schema_json,
            output_type=output_type,
            max_retries=max_retries,
            include_error_in_retry=include_error_in_retry,
        )
    )


__all__ = [
    "extract_structured_async",
    "extract_structured",
]