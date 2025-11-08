"""Tests for Python validator plugin registration and execution.

This module tests the full lifecycle of Python-based validators:
- Registration via register_validator()
- Execution during extraction (before postprocessing)
- Validation error handling
- Unregistration and clearing
- Priority ordering
"""

from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING, Any

import pytest

from kreuzberg import (
    ValidationError,
    clear_validators,
    extract_file,
    extract_file_sync,
    register_validator,
    unregister_validator,
)

if TYPE_CHECKING:
    from collections.abc import Generator

TEST_DOCS_DIR = Path(__file__).parent.parent.parent.parent.parent / "test_documents"
SIMPLE_TEXT_FILE = TEST_DOCS_DIR / "text" / "contract.txt"


class PassValidator:
    """Simple validator that always passes."""

    def name(self) -> str:
        return "pass_validator"

    def validate(self, result: dict[str, Any]) -> None:
        assert isinstance(result, dict)
        assert "content" in result

    def priority(self) -> int:
        return 50

    def should_validate(self, result: dict[str, Any]) -> bool:
        return True


class ContentLengthValidator:
    """Validator that checks minimum content length."""

    def __init__(self, min_length: int = 10) -> None:
        self.min_length = min_length

    def name(self) -> str:
        return "content_length_validator"

    def validate(self, result: dict[str, Any]) -> None:
        content = result.get("content", "")
        if len(content) < self.min_length:
            msg = f"Content too short: {len(content)} < {self.min_length}"
            raise ValidationError(msg)

    def priority(self) -> int:
        return 50

    def should_validate(self, result: dict[str, Any]) -> bool:
        return True


class KeywordValidator:
    """Validator that requires specific keywords."""

    def __init__(self, required_keywords: list[str]) -> None:
        self.required_keywords = required_keywords

    def name(self) -> str:
        return "keyword_validator"

    def validate(self, result: dict[str, Any]) -> None:
        content = result.get("content", "")
        content_lower = content.lower()
        missing = [kw for kw in self.required_keywords if kw.lower() not in content_lower]
        if missing:
            msg = f"Missing required keywords: {missing}"
            raise ValidationError(msg)

    def priority(self) -> int:
        return 50

    def should_validate(self, result: dict[str, Any]) -> bool:
        return True


class PriorityValidator:
    """Validator with custom priority."""

    def __init__(self, priority_value: int, execution_tracker: list[str]) -> None:
        self.priority_value = priority_value
        self.execution_tracker = execution_tracker

    def name(self) -> str:
        return f"priority_{self.priority_value}"

    def priority(self) -> int:
        return self.priority_value

    def validate(self, result: dict[str, Any]) -> None:
        self.execution_tracker.append(self.name())

    def should_validate(self, result: dict[str, Any]) -> bool:
        return True


class FailFastValidator:
    """Validator that always fails."""

    def name(self) -> str:
        return "fail_fast_validator"

    def validate(self, result: dict[str, Any]) -> None:
        msg = "Validation failed intentionally"
        raise ValidationError(msg)

    def priority(self) -> int:
        return 50

    def should_validate(self, result: dict[str, Any]) -> bool:
        return True


class ConditionalValidator:
    """Validator that only validates certain conditions."""

    def name(self) -> str:
        return "conditional_validator"

    def should_validate(self, result: dict[str, Any]) -> bool:
        content = result.get("content", "")
        return len(content) > 100

    def validate(self, result: dict[str, Any]) -> None:
        content = result.get("content", "")
        if "test" not in content.lower():
            msg = "Content must contain 'test'"
            raise ValidationError(msg)

    def priority(self) -> int:
        return 50


@pytest.fixture(autouse=True)
def _cleanup_validators() -> Generator[None, None, None]:
    """Cleanup all validators before and after each test."""
    clear_validators()
    yield
    clear_validators()


def test_register_simple_validator() -> None:
    """Test registering a Python validator that passes."""
    validator = PassValidator()
    register_validator(validator)

    result = extract_file_sync(str(SIMPLE_TEXT_FILE))
    assert result.content is not None


@pytest.mark.asyncio
async def test_validator_async_extraction() -> None:
    """Test validator is called during async extraction."""
    validator = PassValidator()
    register_validator(validator)

    result = await extract_file(str(SIMPLE_TEXT_FILE))
    assert result.content is not None


def test_validator_receives_extraction_result() -> None:
    """Test validator receives valid ExtractionResult."""

    class InspectorValidator:
        def name(self) -> str:
            return "inspector"

        def validate(self, result: dict[str, Any]) -> None:
            assert isinstance(result, dict)
            assert "content" in result
            assert "metadata" in result
            assert "mime_type" in result
            assert "tables" in result

        def priority(self) -> int:
            return 50

        def should_validate(self, result: dict[str, Any]) -> bool:
            return True

    register_validator(InspectorValidator())
    result = extract_file_sync(str(SIMPLE_TEXT_FILE))
    assert result is not None


def test_validator_can_raise_validation_error() -> None:
    """Test validator can raise ValidationError to fail extraction."""
    validator = ContentLengthValidator(min_length=1000000)
    register_validator(validator)

    with pytest.raises((ValidationError, Exception)) as exc_info:
        extract_file_sync(str(SIMPLE_TEXT_FILE))

    assert "too short" in str(exc_info.value).lower() or "validation" in str(exc_info.value).lower()


def test_validator_passes_validation() -> None:
    """Test validator allows extraction when validation passes."""
    validator = ContentLengthValidator(min_length=1)
    register_validator(validator)

    result = extract_file_sync(str(SIMPLE_TEXT_FILE))
    assert result.content is not None


def test_multiple_validators_priority_order() -> None:
    """Test multiple validators execute in priority order (high to low)."""
    execution_order: list[str] = []

    high_priority = PriorityValidator(100, execution_order)
    medium_priority = PriorityValidator(50, execution_order)
    low_priority = PriorityValidator(10, execution_order)

    register_validator(low_priority)
    register_validator(high_priority)
    register_validator(medium_priority)

    extract_file_sync(str(SIMPLE_TEXT_FILE))

    assert execution_order == ["priority_100", "priority_50", "priority_10"]


def test_validator_fail_fast() -> None:
    """Test validation stops on first failure (fail-fast)."""
    executed_validators: list[str] = []

    class FirstValidator:
        def name(self) -> str:
            return "first_validator"

        def priority(self) -> int:
            return 100

        def validate(self, result: dict[str, Any]) -> None:
            executed_validators.append("first")
            msg = "First validator failed"
            raise ValidationError(msg)

        def should_validate(self, result: dict[str, Any]) -> bool:
            return True

    class SecondValidator:
        def name(self) -> str:
            return "second_validator"

        def priority(self) -> int:
            return 50

        def validate(self, result: dict[str, Any]) -> None:
            executed_validators.append("second")

        def should_validate(self, result: dict[str, Any]) -> bool:
            return True

    register_validator(FirstValidator())
    register_validator(SecondValidator())

    with pytest.raises((ValidationError, Exception)):
        extract_file_sync(str(SIMPLE_TEXT_FILE))

    assert "first" in executed_validators
    assert "second" not in executed_validators


def test_unregister_validator() -> None:
    """Test unregistering a validator by name."""
    validator = FailFastValidator()
    register_validator(validator)

    with pytest.raises((ValidationError, Exception)):
        extract_file_sync(str(SIMPLE_TEXT_FILE))

    unregister_validator("fail_fast_validator")
    result = extract_file_sync(str(SIMPLE_TEXT_FILE))
    assert result.content is not None


def test_clear_all_validators() -> None:
    """Test clearing all validators."""
    register_validator(FailFastValidator())
    register_validator(ContentLengthValidator(min_length=1000000))

    with pytest.raises((ValidationError, Exception)):
        extract_file_sync(str(SIMPLE_TEXT_FILE))

    clear_validators()
    result = extract_file_sync(str(SIMPLE_TEXT_FILE))
    assert result.content is not None


def test_validator_with_keyword_check() -> None:
    """Test validator that checks for required keywords."""
    validator = KeywordValidator(["required_keyword_xyz"])
    register_validator(validator)

    with pytest.raises((ValidationError, Exception)) as exc_info:
        extract_file_sync(str(SIMPLE_TEXT_FILE))

    assert "keyword" in str(exc_info.value).lower()


def test_validator_conditional_execution() -> None:
    """Test validator with should_validate method."""
    short_file = SIMPLE_TEXT_FILE

    validator = ConditionalValidator()
    register_validator(validator)

    result = extract_file_sync(str(short_file))
    assert result is not None


def test_multiple_validators_all_pass() -> None:
    """Test multiple validators when all pass."""

    class LengthValidator:
        def name(self) -> str:
            return "length_val"

        def validate(self, result: dict[str, Any]) -> None:
            content = result.get("content", "")
            if len(content) < 1:
                msg = "Content too short"
                raise ValidationError(msg)

        def priority(self) -> int:
            return 50

        def should_validate(self, result: dict[str, Any]) -> bool:
            return True

    class TypeValidator:
        def name(self) -> str:
            return "type_val"

        def validate(self, result: dict[str, Any]) -> None:
            content = result.get("content", "")
            if not isinstance(content, str):
                msg = "Content must be string"
                raise ValidationError(msg)

        def priority(self) -> int:
            return 50

        def should_validate(self, result: dict[str, Any]) -> bool:
            return True

    register_validator(LengthValidator())
    register_validator(TypeValidator())

    result = extract_file_sync(str(SIMPLE_TEXT_FILE))
    assert result.content is not None


def test_validator_with_real_pdf() -> None:
    """Test validator with real PDF extraction."""
    pdf_file = TEST_DOCS_DIR / "pdfs_with_tables" / "tiny.pdf"
    if not pdf_file.exists():
        pytest.skip("Test PDF not found")

    validator = PassValidator()
    register_validator(validator)

    result = extract_file_sync(str(pdf_file))
    assert result.content is not None


@pytest.mark.asyncio
async def test_async_validator() -> None:
    """Test async validator execution."""

    class AsyncValidator:
        def name(self) -> str:
            return "async_validator"

        def validate(self, result: dict[str, Any]) -> None:
            content = result.get("content", "")
            if len(content) < 1:
                msg = "Content too short"
                raise ValidationError(msg)

        def priority(self) -> int:
            return 50

        def should_validate(self, result: dict[str, Any]) -> bool:
            return True

    register_validator(AsyncValidator())

    result = await extract_file(str(SIMPLE_TEXT_FILE))
    assert result.content is not None


def test_validator_error_message_propagation() -> None:
    """Test validation error messages are properly propagated."""

    class CustomMessageValidator:
        def name(self) -> str:
            return "custom_message"

        def validate(self, result: dict[str, Any]) -> None:
            msg = "This is a custom validation error message with unique identifier 12345"
            raise ValidationError(msg)

        def priority(self) -> int:
            return 50

        def should_validate(self, result: dict[str, Any]) -> bool:
            return True

    register_validator(CustomMessageValidator())

    with pytest.raises(Exception, match=r"(?i)(12345|custom)"):
        extract_file_sync(str(SIMPLE_TEXT_FILE))


def test_validator_with_metadata_check() -> None:
    """Test validator that checks metadata."""

    class MetadataValidator:
        def name(self) -> str:
            return "metadata_validator"

        def validate(self, result: dict[str, Any]) -> None:
            mime_type = result.get("mime_type")
            if not mime_type:
                msg = "Missing mime_type"
                raise ValidationError(msg)

        def priority(self) -> int:
            return 50

        def should_validate(self, result: dict[str, Any]) -> bool:
            return True

    register_validator(MetadataValidator())

    result = extract_file_sync(str(SIMPLE_TEXT_FILE))
    assert result.mime_type is not None


def test_validator_priority_default() -> None:
    """Test validators without priority method use default priority."""
    execution_order: list[str] = []

    class NoPriorityValidator:
        def name(self) -> str:
            return "no_priority"

        def validate(self, result: dict[str, Any]) -> None:
            execution_order.append("no_priority")

        def priority(self) -> int:
            return 50

        def should_validate(self, result: dict[str, Any]) -> bool:
            return True

    class HighPriorityValidator:
        def name(self) -> str:
            return "high_priority"

        def priority(self) -> int:
            return 100

        def validate(self, result: dict[str, Any]) -> None:
            execution_order.append("high_priority")

        def should_validate(self, result: dict[str, Any]) -> bool:
            return True

    register_validator(NoPriorityValidator())
    register_validator(HighPriorityValidator())

    extract_file_sync(str(SIMPLE_TEXT_FILE))

    assert execution_order[0] == "high_priority"


@pytest.mark.asyncio
async def test_concurrent_validation() -> None:
    """Test validators work correctly with concurrent extractions."""
    import asyncio

    call_count = 0

    class CountingValidator:
        def name(self) -> str:
            return "counting"

        def validate(self, result: dict[str, Any]) -> None:
            nonlocal call_count
            call_count += 1

        def priority(self) -> int:
            return 50

        def should_validate(self, result: dict[str, Any]) -> bool:
            return True

    register_validator(CountingValidator())

    tasks = [extract_file(str(SIMPLE_TEXT_FILE)) for _ in range(5)]
    results = await asyncio.gather(*tasks)

    assert len(results) == 5
    assert call_count == 5
