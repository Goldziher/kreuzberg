from __future__ import annotations

from collections.abc import Mapping
from pathlib import Path
from typing import Any

import pytest

from kreuzberg import (
    ChunkingConfig,
    ExtractionConfig,
    ImageExtractionConfig,
    LanguageDetectionConfig,
    OcrConfig,
    PdfConfig,
    PostProcessorConfig,
    TokenReductionConfig,
)

_WORKSPACE_ROOT = Path(__file__).resolve().parent.parent.parent.parent
_TEST_DOCUMENTS = _WORKSPACE_ROOT / "test_documents"


def resolve_document(relative: str) -> Path:
    """Return absolute path to a document in test_documents."""

    return _TEST_DOCUMENTS / relative


def build_config(config: dict[str, Any] | None) -> ExtractionConfig:
    """Construct an ExtractionConfig from a plain dictionary."""

    if not config:
        return ExtractionConfig()

    kwargs: dict[str, Any] = {}

    for key in ("use_cache", "enable_quality_processing", "force_ocr"):
        if key in config:
            kwargs[key] = config[key]

    if (ocr_data := config.get("ocr")) is not None:
        kwargs["ocr"] = OcrConfig(**ocr_data)

    if (chunking_data := config.get("chunking")) is not None:
        kwargs["chunking"] = ChunkingConfig(**chunking_data)

    if (images_data := config.get("images")) is not None:
        kwargs["images"] = ImageExtractionConfig(**images_data)

    if (pdf_options := config.get("pdf_options")) is not None:
        kwargs["pdf_options"] = PdfConfig(**pdf_options)

    if (token_reduction := config.get("token_reduction")) is not None:
        kwargs["token_reduction"] = TokenReductionConfig(**token_reduction)

    if (language_detection := config.get("language_detection")) is not None:
        kwargs["language_detection"] = LanguageDetectionConfig(**language_detection)

    if (postprocessor := config.get("postprocessor")) is not None:
        kwargs["postprocessor"] = PostProcessorConfig(**postprocessor)

    return ExtractionConfig(**kwargs)


def assert_expected_mime(result: Any, expected: list[str]) -> None:
    if not expected:
        return
    if not any(token in result.mime_type for token in expected):
        pytest.fail(f"Expected MIME {result.mime_type!r} to match one of {expected!r}")


def assert_min_content_length(result: Any, minimum: int) -> None:
    if len(result.content) < minimum:
        pytest.fail(
            f"Expected content length >= {minimum}, got {len(result.content)}"
        )


def assert_max_content_length(result: Any, maximum: int) -> None:
    if len(result.content) > maximum:
        pytest.fail(
            f"Expected content length <= {maximum}, got {len(result.content)}"
        )


def assert_content_contains_any(result: Any, snippets: list[str]) -> None:
    if not snippets:
        return
    lowered = result.content.lower()
    preview = result.content[:160]
    if not any(snippet.lower() in lowered for snippet in snippets):
        pytest.fail(
            f"Expected content to contain any of {snippets!r}. Preview: {preview!r}"
        )


def assert_content_contains_all(result: Any, snippets: list[str]) -> None:
    if not snippets:
        return
    lowered = result.content.lower()
    missing = [snippet for snippet in snippets if snippet.lower() not in lowered]
    if missing:
        pytest.fail(
            f"Expected content to contain all snippets {snippets!r}. Missing {missing!r}"
        )


def assert_table_count(result: Any, minimum: int | None, maximum: int | None) -> None:
    count = len(getattr(result, "tables", []) or [])
    if minimum is not None and count < minimum:
        pytest.fail(f"Expected at least {minimum} tables, found {count}")
    if maximum is not None and count > maximum:
        pytest.fail(f"Expected at most {maximum} tables, found {count}")


def assert_detected_languages(
    result: Any, expected: list[str], min_confidence: float | None
) -> None:
    if not expected:
        return
    languages = result.detected_languages
    if languages is None:
        pytest.fail("Expected detected languages but field is None")

    missing = [lang for lang in expected if lang not in languages]
    if missing:
        pytest.fail(f"Expected languages {expected!r}, missing {missing!r}")

    if min_confidence is not None:
        confidence = (
            result.metadata.get("confidence")
            if isinstance(result.metadata, Mapping)
            else None
        )
        if confidence is not None and confidence < min_confidence:
            pytest.fail(
                f"Expected confidence >= {min_confidence}, got {confidence}"
            )


def assert_metadata_expectation(result: Any, path: str, expectation: dict[str, Any]) -> None:
    value = _lookup_path(result.metadata, path)
    if value is None:
        pytest.fail(f"Metadata path '{path}' missing in {result.metadata!r}")

    if "eq" in expectation and not _values_equal(value, expectation["eq"]):
        pytest.fail(
            f"Expected metadata '{path}' == {expectation['eq']!r}, got {value!r}"
        )

    if "gte" in expectation:
        actual = float(value)
        if actual < float(expectation["gte"]):
            pytest.fail(
                f"Expected metadata '{path}' >= {expectation['gte']}, got {actual}"
            )

    if "lte" in expectation:
        actual = float(value)
        if actual > float(expectation["lte"]):
            pytest.fail(
                f"Expected metadata '{path}' <= {expectation['lte']}, got {actual}"
            )

    if "contains" in expectation:
        expected_values = expectation["contains"]
        if isinstance(value, str) and isinstance(expected_values, str):
            if expected_values not in value:
                pytest.fail(
                    f"Expected metadata '{path}' string to contain {expected_values!r}"
                )
        elif isinstance(value, (list, tuple, set)):
            missing = [item for item in expected_values if item not in value]
            if missing:
                pytest.fail(
                    f"Expected metadata '{path}' to contain {expected_values!r}, missing {missing!r}"
                )
        else:
            pytest.fail(
                f"Unsupported contains expectation for metadata '{path}': {value!r}"
            )

    if expectation.get("exists") is False:
        pytest.fail("exists=False not supported for metadata expectations")


def _lookup_path(metadata: Mapping[str, Any], path: str) -> Any:
    current: Any = metadata
    for segment in path.split("."):
        if not isinstance(current, Mapping) or segment not in current:
            return None
        current = current[segment]
    return current


def _values_equal(lhs: Any, rhs: Any) -> bool:
    if isinstance(lhs, str) and isinstance(rhs, str):
        return lhs == rhs
    if isinstance(lhs, (int, float)) and isinstance(rhs, (int, float)):
        return float(lhs) == float(rhs)
    if isinstance(lhs, bool) and isinstance(rhs, bool):
        return lhs is rhs
    return bool(lhs == rhs)
