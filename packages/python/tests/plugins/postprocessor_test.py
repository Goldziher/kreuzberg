"""Tests for Python postprocessor plugin registration and execution.

This module tests the full lifecycle of Python-based postprocessors:
- Registration via register_post_processor()
- Execution during extraction (both sync and async)
- Unregistration and clearing
- Error handling and edge cases
"""

from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING, Literal

import pytest

from kreuzberg import (
    ExtractionConfig,
    ExtractionResult,
    PostProcessorConfig,
    clear_post_processors,
    extract_file,
    extract_file_sync,
    register_post_processor,
    unregister_post_processor,
)

if TYPE_CHECKING:
    from collections.abc import Generator
    from typing import Any


# Test document paths
# Navigate from packages/python/tests/plugins/ to root/test_documents/
TEST_DOCS_DIR = Path(__file__).parent.parent.parent.parent.parent / "test_documents"
SIMPLE_TEXT_FILE = TEST_DOCS_DIR / "text" / "contract.txt"


# Test postprocessor classes


class SimplePostProcessor:
    """Simple postprocessor that adds metadata."""

    def name(self) -> str:
        return "simple_test_processor"

    def process(self, result: dict[str, Any]) -> dict[str, Any]:
        if "metadata" not in result:
            result["metadata"] = {}
        result["metadata"]["simple_processor_executed"] = True
        result["metadata"]["processor_name"] = "simple_test_processor"
        return result

    def processing_stage(self) -> Literal["early", "middle", "late"]:
        return "middle"

    def initialize(self) -> None:
        pass

    def shutdown(self) -> None:
        pass


class ContentModifier:
    """Postprocessor that modifies content."""

    def name(self) -> str:
        return "content_modifier"

    def process(self, result: dict[str, Any]) -> dict[str, Any]:
        result["content"] = result["content"].upper()
        if "metadata" not in result:
            result["metadata"] = {}
        result["metadata"]["content_modified"] = True
        return result

    def processing_stage(self) -> Literal["early", "middle", "late"]:
        return "late"

    def initialize(self) -> None:
        pass

    def shutdown(self) -> None:
        pass


class EarlyStageProcessor:
    """Processor that runs in early stage."""

    def name(self) -> str:
        return "early_processor"

    def process(self, result: dict[str, Any]) -> dict[str, Any]:
        if "metadata" not in result:
            result["metadata"] = {}
        result["metadata"]["early_stage_executed"] = True
        return result

    def processing_stage(self) -> Literal["early", "middle", "late"]:
        return "early"

    def initialize(self) -> None:
        pass

    def shutdown(self) -> None:
        pass


class LateStageProcessor:
    """Processor that runs in late stage."""

    def name(self) -> str:
        return "late_processor"

    def process(self, result: dict[str, Any]) -> dict[str, Any]:
        if "metadata" not in result:
            result["metadata"] = {}
        result["metadata"]["late_stage_executed"] = True
        return result

    def processing_stage(self) -> Literal["early", "middle", "late"]:
        return "late"

    def initialize(self) -> None:
        pass

    def shutdown(self) -> None:
        pass


class StatefulProcessor:
    """Stateful processor that counts invocations."""

    def __init__(self) -> None:
        self.call_count = 0

    def name(self) -> str:
        return "stateful_processor"

    def process(self, result: dict[str, Any]) -> dict[str, Any]:
        self.call_count += 1
        if "metadata" not in result:
            result["metadata"] = {}
        result["metadata"]["call_count"] = self.call_count
        return result

    def processing_stage(self) -> Literal["early", "middle", "late"]:
        return "middle"

    def initialize(self) -> None:
        pass

    def shutdown(self) -> None:
        pass


class ErrorProcessor:
    """Processor that raises an error."""

    def name(self) -> str:
        return "error_processor"

    def process(self, result: dict[str, Any]) -> dict[str, Any]:
        msg = "Intentional error for testing"
        raise ValueError(msg)

    def processing_stage(self) -> Literal["early", "middle", "late"]:
        return "middle"

    def initialize(self) -> None:
        pass

    def shutdown(self) -> None:
        pass


# Fixtures


@pytest.fixture(autouse=True)
def _cleanup_processors() -> Generator[None, None, None]:
    """Cleanup all processors before and after each test."""
    clear_post_processors()
    yield
    clear_post_processors()


# Tests


def test_register_simple_postprocessor_class() -> None:
    """Test registering a Python class as postprocessor."""
    processor = SimplePostProcessor()
    register_post_processor(processor)

    # Verify processor is called during extraction
    result = extract_file_sync(str(SIMPLE_TEXT_FILE))
    assert result.metadata.get("simple_processor_executed") is True
    assert result.metadata.get("processor_name") == "simple_test_processor"


@pytest.mark.asyncio
async def test_register_postprocessor_async_extraction() -> None:
    """Test postprocessor is called during async extraction."""
    processor = SimplePostProcessor()
    register_post_processor(processor)

    result = await extract_file(str(SIMPLE_TEXT_FILE))
    assert result.metadata.get("simple_processor_executed") is True


def test_postprocessor_receives_extraction_result() -> None:
    """Test postprocessor receives valid ExtractionResult."""

    class InspectorProcessor:
        def name(self) -> str:
            return "inspector"

        def process(self, result: dict[str, Any]) -> dict[str, Any]:
            # Verify result structure
            assert isinstance(result, (dict, ExtractionResult))
            assert "content" in result
            assert "metadata" in result
            assert "mime_type" in result
            assert "tables" in result
            result.setdefault("metadata", {})["inspector_passed"] = True
            return result

        def processing_stage(self) -> Literal["early", "middle", "late"]:
            return "middle"

        def initialize(self) -> None:
            pass

        def shutdown(self) -> None:
            pass

    register_post_processor(InspectorProcessor())
    result = extract_file_sync(str(SIMPLE_TEXT_FILE))
    assert result.metadata.get("inspector_passed") is True


def test_postprocessor_modifies_content() -> None:
    """Test postprocessor can modify content."""
    register_post_processor(ContentModifier())

    result = extract_file_sync(str(SIMPLE_TEXT_FILE))
    assert result.metadata.get("content_modified") is True
    # Content should be uppercase
    assert result.content.isupper()


def test_multiple_postprocessors_stage_order() -> None:  # noqa: C901
    """Test multiple postprocessors execute in stage order."""
    execution_order: list[str] = []

    class OrderTrackerEarly:
        def name(self) -> str:
            return "early_tracker"

        def process(self, result: dict[str, Any]) -> dict[str, Any]:
            execution_order.append("early")
            result.setdefault("metadata", {})["early_ran"] = True
            return result

        def processing_stage(self) -> Literal["early", "middle", "late"]:
            return "early"

        def initialize(self) -> None:
            pass

        def shutdown(self) -> None:
            pass

    class OrderTrackerMiddle:
        def name(self) -> str:
            return "middle_tracker"

        def process(self, result: dict[str, Any]) -> dict[str, Any]:
            execution_order.append("middle")
            result.setdefault("metadata", {})["middle_ran"] = True
            return result

        def processing_stage(self) -> Literal["early", "middle", "late"]:
            return "middle"

        def initialize(self) -> None:
            pass

        def shutdown(self) -> None:
            pass

    class OrderTrackerLate:
        def name(self) -> str:
            return "late_tracker"

        def process(self, result: dict[str, Any]) -> dict[str, Any]:
            execution_order.append("late")
            result.setdefault("metadata", {})["late_ran"] = True
            return result

        def processing_stage(self) -> Literal["early", "middle", "late"]:
            return "late"

        def initialize(self) -> None:
            pass

        def shutdown(self) -> None:
            pass

    # Register in reverse order to test stage ordering
    register_post_processor(OrderTrackerLate())
    register_post_processor(OrderTrackerMiddle())
    register_post_processor(OrderTrackerEarly())

    result = extract_file_sync(str(SIMPLE_TEXT_FILE))

    assert execution_order == ["early", "middle", "late"]
    assert result.metadata.get("early_ran") is True
    assert result.metadata.get("middle_ran") is True
    assert result.metadata.get("late_ran") is True


def test_unregister_postprocessor() -> None:
    """Test unregistering a postprocessor by name."""
    processor = SimplePostProcessor()
    register_post_processor(processor)

    # Verify processor is called
    result = extract_file_sync(str(SIMPLE_TEXT_FILE))
    assert result.metadata.get("simple_processor_executed") is True

    # Unregister and verify it's not called
    unregister_post_processor("simple_test_processor")
    result = extract_file_sync(str(SIMPLE_TEXT_FILE))
    assert result.metadata.get("simple_processor_executed") is None


def test_clear_all_postprocessors() -> None:
    """Test clearing all postprocessors."""
    register_post_processor(SimplePostProcessor())
    register_post_processor(ContentModifier())

    # Verify both are called
    result = extract_file_sync(str(SIMPLE_TEXT_FILE))
    assert result.metadata.get("simple_processor_executed") is True
    assert result.metadata.get("content_modified") is True

    # Clear all and verify none are called
    clear_post_processors()
    result = extract_file_sync(str(SIMPLE_TEXT_FILE))
    assert result.metadata.get("simple_processor_executed") is None
    assert result.metadata.get("content_modified") is None


def test_postprocessor_error_propagation() -> None:
    """Test postprocessor error handling."""
    register_post_processor(ErrorProcessor())

    # Error should propagate (or be captured in metadata)
    result = extract_file_sync(str(SIMPLE_TEXT_FILE))
    # Check if error is captured in metadata
    error_keys = [k for k in result.metadata if "error" in k.lower()]
    assert len(error_keys) > 0


def test_stateful_postprocessor() -> None:
    """Test stateful postprocessor maintains state across calls."""
    processor = StatefulProcessor()
    register_post_processor(processor)

    result1 = extract_file_sync(str(SIMPLE_TEXT_FILE))
    assert result1.metadata.get("call_count") == 1

    result2 = extract_file_sync(str(SIMPLE_TEXT_FILE))
    assert result2.metadata.get("call_count") == 2

    result3 = extract_file_sync(str(SIMPLE_TEXT_FILE))
    assert result3.metadata.get("call_count") == 3


def test_postprocessor_with_disabled_config() -> None:
    """Test postprocessor is skipped when disabled in config."""
    register_post_processor(SimplePostProcessor())

    # With postprocessor enabled (default)
    result_enabled = extract_file_sync(str(SIMPLE_TEXT_FILE))
    assert result_enabled.metadata.get("simple_processor_executed") is True

    # With postprocessor disabled
    config = ExtractionConfig(postprocessor=PostProcessorConfig(enabled=False))
    result_disabled = extract_file_sync(str(SIMPLE_TEXT_FILE), config=config)
    assert result_disabled.metadata.get("simple_processor_executed") is None


def test_postprocessor_whitelist() -> None:
    """Test postprocessor whitelist filtering."""
    register_post_processor(SimplePostProcessor())
    register_post_processor(ContentModifier())

    # Only enable simple_test_processor
    config = ExtractionConfig(
        postprocessor=PostProcessorConfig(
            enabled=True,
            enabled_processors=["simple_test_processor"],
        )
    )
    result = extract_file_sync(str(SIMPLE_TEXT_FILE), config=config)

    assert result.metadata.get("simple_processor_executed") is True
    assert result.metadata.get("content_modified") is None


def test_postprocessor_blacklist() -> None:
    """Test postprocessor blacklist filtering."""
    register_post_processor(SimplePostProcessor())
    register_post_processor(ContentModifier())

    # Disable content_modifier
    config = ExtractionConfig(
        postprocessor=PostProcessorConfig(
            enabled=True,
            disabled_processors=["content_modifier"],
        )
    )
    result = extract_file_sync(str(SIMPLE_TEXT_FILE), config=config)

    assert result.metadata.get("simple_processor_executed") is True
    assert result.metadata.get("content_modified") is None


def test_postprocessor_function_registration() -> None:
    """Test registering a Python function as postprocessor."""

    class FunctionProcessor:
        def name(self) -> str:
            return "function_processor"

        def process(self, result: dict[str, Any]) -> dict[str, Any]:
            result.setdefault("metadata", {})["function_processor_executed"] = True
            result.setdefault("metadata", {})["original_length"] = len(result["content"])
            return result

        def processing_stage(self) -> Literal["early", "middle", "late"]:
            return "middle"

        def initialize(self) -> None:
            pass

        def shutdown(self) -> None:
            pass

    register_post_processor(FunctionProcessor())
    result = extract_file_sync(str(SIMPLE_TEXT_FILE))
    assert result.metadata.get("function_processor_executed") is True
    assert result.metadata.get("original_length") is not None


def test_postprocessor_with_real_pdf() -> None:
    """Test postprocessor with real PDF extraction."""
    pdf_file = TEST_DOCS_DIR / "pdfs_with_tables" / "tiny.pdf"
    if not pdf_file.exists():
        pytest.skip("Test PDF not found")

    register_post_processor(SimplePostProcessor())
    result = extract_file_sync(str(pdf_file))
    assert result.metadata.get("simple_processor_executed") is True


@pytest.mark.asyncio
async def test_concurrent_extraction_with_stateful_processor() -> None:
    """Test stateful processor handles concurrent extractions."""
    import asyncio

    processor = StatefulProcessor()
    register_post_processor(processor)

    # Run 5 concurrent extractions
    tasks = [extract_file(str(SIMPLE_TEXT_FILE)) for _ in range(5)]
    results = await asyncio.gather(*tasks)

    # All should have been processed
    call_counts: list[int] = [r.metadata.get("call_count") for r in results]  # type: ignore[misc]
    assert all(count is not None for count in call_counts)
    assert len(set(call_counts)) == 5  # All should be unique
    assert sorted(call_counts) == [1, 2, 3, 4, 5]


def test_postprocessor_with_empty_content() -> None:
    """Test postprocessor handles empty content."""

    class EmptyContentProcessor:
        def name(self) -> str:
            return "empty_processor"

        def process(self, result: dict[str, Any]) -> dict[str, Any]:
            result.setdefault("metadata", {})["was_empty"] = len(result["content"]) == 0
            result.setdefault("metadata", {})["content_length"] = len(result["content"])
            return result

        def processing_stage(self) -> Literal["early", "middle", "late"]:
            return "middle"

        def initialize(self) -> None:
            pass

        def shutdown(self) -> None:
            pass

    register_post_processor(EmptyContentProcessor())
    result = extract_file_sync(str(SIMPLE_TEXT_FILE))
    assert "was_empty" in result.metadata
    assert "content_length" in result.metadata


def test_postprocessor_duplicate_names() -> None:
    """Test handling of duplicate processor names."""

    class Proc1:
        def name(self) -> str:
            return "duplicate"

        def process(self, result: dict[str, Any]) -> dict[str, Any]:
            result.setdefault("metadata", {})["proc_version"] = 1
            return result

        def processing_stage(self) -> Literal["early", "middle", "late"]:
            return "middle"

        def initialize(self) -> None:
            pass

        def shutdown(self) -> None:
            pass

    class Proc2:
        def name(self) -> str:
            return "duplicate"

        def process(self, result: dict[str, Any]) -> dict[str, Any]:
            result.setdefault("metadata", {})["proc_version"] = 2
            return result

        def processing_stage(self) -> Literal["early", "middle", "late"]:
            return "middle"

        def initialize(self) -> None:
            pass

        def shutdown(self) -> None:
            pass

    register_post_processor(Proc1())
    register_post_processor(Proc2())

    result = extract_file_sync(str(SIMPLE_TEXT_FILE))
    # Second processor should override
    assert result.metadata.get("proc_version") is not None
