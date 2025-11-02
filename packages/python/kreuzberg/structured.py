"""Structured extraction stubs and configuration proposal.

This module contains a lightweight Python-facing stub for the proposed
structured extraction feature. It is intended as a discussion and review
artifact for a draft PR and *not* a production implementation.

Goals:
- Describe the intended Python surface for structured extraction
- Provide a dataclass stub that can be used in tests and docs
- Attempt to import optional dependencies (msgspec, pydantic, litellm)
  and provide helpful errors if they are missing

When the full implementation is developed, the runtime will:
- Use LiteLLM (vision-enabled models) to extract JSON matching the
  provided schema (msgspec.Struct or Pydantic v2 BaseModel)
- Validate the output and retry with error feedback according to config

"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Dict, Optional, Type

try:
    import msgspec  # type: ignore
except Exception:  # pragma: no cover - optional dependency
    msgspec = None  # type: ignore

try:
    import pydantic  # type: ignore
    from pydantic import BaseModel as PydanticBaseModel  # type: ignore
except Exception:  # pragma: no cover - optional dependency
    pydantic = None  # type: ignore
    PydanticBaseModel = None  # type: ignore

try:
    import litellm  # type: ignore
except Exception:  # pragma: no cover - optional dependency
    litellm = None  # type: ignore

from kreuzberg.exceptions import MissingDependencyError


@dataclass
class StructuredExtractionConfig:
    """Proposed Python-level configuration for structured extraction.

    Notes:
    - This is a lightweight Python stub. The canonical `ExtractionConfig`
      struct lives in the Rust bindings. When implementing, we should either
      extend the Rust `ExtractionConfig` or add a thin Python wrapper that
      maps these fields into the rust side.

    Fields:
        output_type: A schema type to validate the extraction to. This should
            be either a `msgspec.Struct` subclass or a Pydantic v2 `BaseModel`.
        extraction_model: LiteLLM model identifier (string) for vision model.
        extraction_model_config: Model-specific options passed through to LiteLLM.
        max_extraction_retries: Number of times to retry on validation failures.
        include_error_in_retry: Include validation error details when prompting for retry.
    """

    output_type: Optional[Type[Any]] = None
    extraction_model: Optional[str] = None
    extraction_model_config: Optional[Dict[str, Any]] = None
    max_extraction_retries: int = 2
    include_error_in_retry: bool = True

    def validate_dependencies(self) -> None:
        """Raise a helpful MissingDependencyError when optional deps are missing.

        This helper is used by the higher-level structured extraction flow to
        fail early and provide an install suggestion.
        """
        if self.output_type is None:
            return

        if msgspec is None and pydantic is None:
            raise MissingDependencyError.create_for_package(
                dependency_group="structured",
                functionality="structured extraction (msgspec or pydantic)",
                package_name="msgspec or pydantic",
            )

        if self.extraction_model and litellm is None:
            raise MissingDependencyError.create_for_package(
                dependency_group="structured",
                functionality="LiteLLM vision model integration",
                package_name="litellm",
            )


__all__ = ["StructuredExtractionConfig"]
