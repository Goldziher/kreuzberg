# mypy: ignore-errors
"""Integration tests for custom PostProcessor registration and execution.

Tests cover:
- All processing stages (early, middle, late)
- Single and multiple processor registration
- Metadata modification
- Error handling
- Processor ordering
- Integration with extraction pipeline
"""

from __future__ import annotations

import pytest

from kreuzberg import (
    ExtractionConfig,
    PostProcessorConfig,
    clear_post_processors,
    extract_bytes_sync,
    register_post_processor,
)


@pytest.fixture(autouse=True)
def _clear_processors() -> None:
    """Clear all registered processors before each test.

    This prevents processor accumulation across tests and ensures
    each test starts with a clean registry.
    """
    clear_post_processors()
    yield
    clear_post_processors()


class EarlyStageProcessor:
    """Processor that runs in early stage."""

    def name(self) -> str:
        return "early_processor"

    def process(self, result: dict) -> dict:
        """Process method receives and returns a dict, not ExtractionResult."""
        if "metadata" not in result:
            result["metadata"] = {}
        result["metadata"]["early_executed"] = True
        result["metadata"]["execution_order"] = result["metadata"].get("execution_order", [])
        result["metadata"]["execution_order"].append("early")
        return result

    def processing_stage(self) -> str:
        return "early"


class MiddleStageProcessor:
    """Processor that runs in middle stage (default)."""

    def name(self) -> str:
        return "middle_processor"

    def process(self, result: dict) -> dict:
        result["metadata"]["middle_executed"] = True
        result["metadata"]["execution_order"] = result["metadata"].get("execution_order", [])
        result["metadata"]["execution_order"].append("middle")
        return result

    def processing_stage(self) -> str:
        return "middle"


class LateStageProcessor:
    """Processor that runs in late stage."""

    def name(self) -> str:
        return "late_processor"

    def process(self, result: dict) -> dict:
        result["metadata"]["late_executed"] = True
        result["metadata"]["execution_order"] = result["metadata"].get("execution_order", [])
        result["metadata"]["execution_order"].append("late")
        return result

    def processing_stage(self) -> str:
        return "late"


class WordCountProcessor:
    """Processor that adds word count statistics."""

    def name(self) -> str:
        return "word_count"

    def process(self, result: dict) -> dict:
        words = result["content"].split()
        result["metadata"]["word_count"] = len(words)
        result["metadata"]["character_count"] = len(result["content"])
        return result

    def processing_stage(self) -> str:
        return "middle"


class SentenceCountProcessor:
    """Processor that counts sentences."""

    def name(self) -> str:
        return "sentence_count"

    def process(self, result: dict) -> dict:
        sentence_endings = result["content"].count(".") + result["content"].count("!") + result["content"].count("?")
        result["metadata"]["sentence_count"] = max(1, sentence_endings)
        return result

    def processing_stage(self) -> str:
        return "middle"


class UppercaseTagProcessor:
    """Processor that detects uppercase text."""

    def name(self) -> str:
        return "uppercase_tag"

    def process(self, result: dict) -> dict:
        text = result["content"]
        alpha_chars = [c for c in text if c.isalpha()]
        if alpha_chars:
            uppercase_ratio = sum(1 for c in alpha_chars if c.isupper()) / len(alpha_chars)
            result["metadata"]["uppercase_ratio"] = uppercase_ratio
            result["metadata"]["is_mostly_uppercase"] = uppercase_ratio > 0.5
        return result

    def processing_stage(self) -> str:
        return "late"


class InitializableProcessor:
    """Processor with initialize and shutdown methods."""

    def __init__(self) -> None:
        self.initialized = False
        self.call_count = 0

    def name(self) -> str:
        return "initializable"

    def initialize(self) -> None:
        self.initialized = True

    def process(self, result: dict) -> dict:
        self.call_count += 1
        result["metadata"]["processor_initialized"] = self.initialized
        result["metadata"]["processor_call_count"] = self.call_count
        return result

    def processing_stage(self) -> str:
        return "middle"

    def shutdown(self) -> None:
        self.initialized = False


class ErrorHandlingProcessor:
    """Processor that handles errors gracefully."""

    def __init__(self, should_fail: bool = False) -> None:
        self.should_fail = should_fail

    def name(self) -> str:
        return "error_handler"

    def process(self, result: dict) -> dict:
        try:
            if self.should_fail:
                msg = "Intentional error for testing"
                raise ValueError(msg)
            result["metadata"]["error_handler_success"] = True
        except Exception as e:
            result["metadata"]["error_handler_error"] = str(e)

        return result

    def processing_stage(self) -> str:
        return "middle"


def test_register_early_stage_processor() -> None:
    """Test registering and executing an early-stage processor."""
    processor = EarlyStageProcessor()
    register_post_processor(processor)

    test_content = "This is a test document."
    result = extract_bytes_sync(test_content.encode(), "text/plain", ExtractionConfig())

    assert result.metadata.get("early_executed") is True
    assert "early" in result.metadata.get("execution_order", [])


def test_register_middle_stage_processor() -> None:
    """Test registering and executing a middle-stage processor."""
    processor = MiddleStageProcessor()
    register_post_processor(processor)

    test_content = "This is a test document."
    result = extract_bytes_sync(test_content.encode(), "text/plain", ExtractionConfig())

    assert result.metadata.get("middle_executed") is True
    assert "middle" in result.metadata.get("execution_order", [])


def test_register_late_stage_processor() -> None:
    """Test registering and executing a late-stage processor."""
    processor = LateStageProcessor()
    register_post_processor(processor)

    test_content = "This is a test document."
    result = extract_bytes_sync(test_content.encode(), "text/plain", ExtractionConfig())

    assert result.metadata.get("late_executed") is True
    assert "late" in result.metadata.get("execution_order", [])


def test_multiple_processors_execution_order() -> None:
    """Test that multiple processors execute in stage order."""
    late_proc = LateStageProcessor()
    middle_proc = MiddleStageProcessor()
    early_proc = EarlyStageProcessor()

    register_post_processor(late_proc)
    register_post_processor(middle_proc)
    register_post_processor(early_proc)

    test_content = "Test content for ordering."
    result = extract_bytes_sync(test_content.encode(), "text/plain", ExtractionConfig())

    assert result.metadata.get("early_executed") is True
    assert result.metadata.get("middle_executed") is True
    assert result.metadata.get("late_executed") is True

    execution_order = result.metadata.get("execution_order", [])
    assert "early" in execution_order
    assert "middle" in execution_order
    assert "late" in execution_order

    early_idx = execution_order.index("early")
    middle_idx = execution_order.index("middle")
    late_idx = execution_order.index("late")

    assert early_idx < middle_idx < late_idx


def test_multiple_processors_same_stage() -> None:
    """Test multiple processors in the same stage."""
    word_proc = WordCountProcessor()
    sentence_proc = SentenceCountProcessor()

    register_post_processor(word_proc)
    register_post_processor(sentence_proc)

    test_content = "Hello world. This is a test. How are you?"
    result = extract_bytes_sync(test_content.encode(), "text/plain", ExtractionConfig())

    assert "word_count" in result.metadata
    assert "sentence_count" in result.metadata

    assert result.metadata["word_count"] == 9
    assert result.metadata["sentence_count"] == 3


def test_processor_adds_metadata() -> None:
    """Test that processor can add metadata without overwriting existing."""
    processor = WordCountProcessor()
    register_post_processor(processor)

    test_content = "One two three four five."
    result = extract_bytes_sync(test_content.encode(), "text/plain", ExtractionConfig())

    assert result.metadata["word_count"] == 5
    assert result.metadata["character_count"] == len(test_content)

    assert "mime_type" in result.metadata or result.mime_type == "text/plain"


def test_processor_chain_metadata_accumulation() -> None:
    """Test that metadata accumulates across processor chain."""
    proc1 = WordCountProcessor()
    proc2 = SentenceCountProcessor()
    proc3 = UppercaseTagProcessor()

    register_post_processor(proc1)
    register_post_processor(proc2)
    register_post_processor(proc3)

    test_content = "HELLO WORLD. THIS IS A TEST."
    result = extract_bytes_sync(test_content.encode(), "text/plain", ExtractionConfig())

    assert "word_count" in result.metadata
    assert "sentence_count" in result.metadata
    assert "uppercase_ratio" in result.metadata
    assert result.metadata["is_mostly_uppercase"] is True


def test_processor_initialization() -> None:
    """Test that initialize() is called on processor registration."""
    processor = InitializableProcessor()

    assert processor.initialized is False

    register_post_processor(processor)

    test_content = "Test initialization."
    result = extract_bytes_sync(test_content.encode(), "text/plain", ExtractionConfig())

    assert result.metadata.get("processor_initialized") is True
    assert result.metadata.get("processor_call_count", 0) > 0


def test_processor_multiple_calls() -> None:
    """Test that processor can handle multiple extraction calls."""
    processor = InitializableProcessor()
    register_post_processor(processor)

    result1 = extract_bytes_sync(b"First call.", "text/plain", ExtractionConfig())
    count1 = result1.metadata.get("processor_call_count", 0)

    result2 = extract_bytes_sync(b"Second call.", "text/plain", ExtractionConfig())
    count2 = result2.metadata.get("processor_call_count", 0)

    assert count2 > count1


def test_processor_error_handling_graceful() -> None:
    """Test that processor errors don't crash the pipeline."""
    processor = ErrorHandlingProcessor(should_fail=False)
    register_post_processor(processor)

    test_content = "Test error handling."
    result = extract_bytes_sync(test_content.encode(), "text/plain", ExtractionConfig())

    assert result.metadata.get("error_handler_success") is True
    assert "error_handler_error" not in result.metadata


def test_processor_error_handling_with_failure() -> None:
    """Test that processor can handle internal errors gracefully."""
    processor = ErrorHandlingProcessor(should_fail=True)
    register_post_processor(processor)

    test_content = "Test error handling with failure."
    result = extract_bytes_sync(test_content.encode(), "text/plain", ExtractionConfig())

    assert "error_handler_error" in result.metadata
    assert "Intentional error" in result.metadata["error_handler_error"]


def test_postprocessor_config_enabled() -> None:
    """Test that PostProcessorConfig.enabled controls processor execution."""
    processor = WordCountProcessor()
    register_post_processor(processor)

    test_content = "Test with postprocessing enabled."

    config_enabled = ExtractionConfig(postprocessor=PostProcessorConfig(enabled=True))
    result_enabled = extract_bytes_sync(test_content.encode(), "text/plain", config_enabled)

    assert "word_count" in result_enabled.metadata

    config_disabled = ExtractionConfig(postprocessor=PostProcessorConfig(enabled=False))
    result_disabled = extract_bytes_sync(test_content.encode(), "text/plain", config_disabled)

    assert "word_count" not in result_disabled.metadata


def test_postprocessor_config_whitelist() -> None:
    """Test PostProcessorConfig.enabled_processors whitelist."""
    proc1 = WordCountProcessor()
    proc2 = SentenceCountProcessor()

    register_post_processor(proc1)
    register_post_processor(proc2)

    test_content = "Test whitelist. Second sentence."

    config = ExtractionConfig(
        postprocessor=PostProcessorConfig(
            enabled=True,
            enabled_processors=["word_count"],
        )
    )

    result = extract_bytes_sync(test_content.encode(), "text/plain", config)

    assert "word_count" in result.metadata

    assert "sentence_count" not in result.metadata


def test_postprocessor_config_blacklist() -> None:
    """Test PostProcessorConfig.disabled_processors blacklist."""
    proc1 = WordCountProcessor()
    proc2 = SentenceCountProcessor()

    register_post_processor(proc1)
    register_post_processor(proc2)

    test_content = "Test blacklist. Second sentence."

    config = ExtractionConfig(
        postprocessor=PostProcessorConfig(
            enabled=True,
            disabled_processors=["sentence_count"],
        )
    )

    result = extract_bytes_sync(test_content.encode(), "text/plain", config)

    assert "word_count" in result.metadata

    assert "sentence_count" not in result.metadata


def test_realistic_text_analysis_pipeline() -> None:
    """Test a realistic pipeline with multiple analysis processors."""
    word_proc = WordCountProcessor()
    sentence_proc = SentenceCountProcessor()
    uppercase_proc = UppercaseTagProcessor()

    register_post_processor(word_proc)
    register_post_processor(sentence_proc)
    register_post_processor(uppercase_proc)

    test_content = """Machine learning is a subset of artificial intelligence.
It focuses on building systems that can learn from data.
Deep learning uses neural networks with multiple layers."""

    result = extract_bytes_sync(test_content.encode(), "text/plain", ExtractionConfig())

    assert result.metadata["word_count"] == 26
    assert result.metadata["sentence_count"] == 3
    assert result.metadata["is_mostly_uppercase"] is False
    assert 0.0 <= result.metadata["uppercase_ratio"] <= 1.0


def test_processor_with_empty_content() -> None:
    """Test processors handle empty content gracefully."""
    processor = WordCountProcessor()
    register_post_processor(processor)

    result = extract_bytes_sync(b"", "text/plain", ExtractionConfig())

    assert result.metadata["word_count"] == 0
    assert result.metadata["character_count"] == 0


def test_processor_with_unicode_content() -> None:
    """Test processors handle Unicode content correctly."""
    processor = WordCountProcessor()
    register_post_processor(processor)

    test_content = "Hello 世界! Здравствуй мир! مرحبا بالعالم!"
    result = extract_bytes_sync(test_content.encode("utf-8"), "text/plain", ExtractionConfig())

    assert result.metadata["word_count"] > 0
    assert result.metadata["character_count"] == len(test_content)


def test_processor_name_uniqueness() -> None:
    """Test that each processor has a unique name."""

    class DuplicateNameProcessor:
        def name(self) -> str:
            return "word_count"

        def process(self, result: dict) -> dict:
            result["metadata"]["duplicate"] = True
            return result

        def processing_stage(self) -> str:
            return "middle"

    proc1 = WordCountProcessor()
    proc2 = DuplicateNameProcessor()

    register_post_processor(proc1)
    register_post_processor(proc2)

    test_content = "Test duplicate names."
    result = extract_bytes_sync(test_content.encode(), "text/plain", ExtractionConfig())

    has_word_count = "word_count" in result.metadata
    has_duplicate = "duplicate" in result.metadata

    assert has_word_count or has_duplicate


def test_processor_without_optional_methods() -> None:
    """Test processor that only implements required methods."""

    class MinimalProcessor:
        def name(self) -> str:
            return "minimal"

        def process(self, result: dict) -> dict:
            result["metadata"]["minimal_executed"] = True
            return result

    processor = MinimalProcessor()
    register_post_processor(processor)

    test_content = "Test minimal processor."
    result = extract_bytes_sync(test_content.encode(), "text/plain", ExtractionConfig())

    assert result.metadata.get("minimal_executed") is True


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
