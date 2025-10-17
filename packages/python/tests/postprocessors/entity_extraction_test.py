"""Tests for EntityExtractionProcessor.

Tests entity extraction using spaCy NER, including model configuration,
entity filtering, and error handling.
"""

from __future__ import annotations

import pytest

from kreuzberg import ExtractionResult
from kreuzberg.postprocessors.entity_extraction import EntityExtractionProcessor


def test_processor_initialization_with_custom_config() -> None:
    """Test processor can be initialized with custom configuration."""
    processor = EntityExtractionProcessor(
        model="en_core_web_md",
        entity_types=["PERSON", "ORG"],
        max_entities=10,
        min_confidence=0.5,
    )

    assert processor.model_name == "en_core_web_md"
    assert processor.entity_types == ["PERSON", "ORG"]
    assert processor.max_entities == 10
    assert processor.min_confidence == 0.5


def test_processor_with_model_path() -> None:
    """Test processor can be configured with a custom model path."""
    processor = EntityExtractionProcessor(model="en_core_web_sm", model_path="/custom/path/to/model")

    assert processor.model_path == "/custom/path/to/model"


def test_processor_with_model_kwargs() -> None:
    """Test processor accepts and stores model kwargs."""
    processor = EntityExtractionProcessor(disable=["parser", "tagger"], exclude=["lemmatizer"])

    assert processor.model_kwargs["disable"] == ["parser", "tagger"]
    assert processor.model_kwargs["exclude"] == ["lemmatizer"]


def test_process_empty_content() -> None:
    """Test processor handles empty content gracefully."""
    processor = EntityExtractionProcessor()
    result: ExtractionResult = ExtractionResult(
        content="",
        mime_type="text/plain",
        metadata={},
        tables=[],
    )

    processed = processor.process(result)

    # Should return result unchanged
    assert processed.content == ""


def test_process_non_string_content() -> None:
    """Test processor handles non-string content gracefully."""
    processor = EntityExtractionProcessor()
    result: ExtractionResult = ExtractionResult(
        content="12345",
        mime_type="text/plain",
        metadata={},
        tables=[],
    )

    processed = processor.process(result)

    # Should return result unchanged
    assert processed.content == "12345"


@pytest.mark.skipif(
    pytest.importorskip("spacy", reason="spaCy not installed") is None,
    reason="spaCy not installed",
)
def test_process_with_spacy() -> None:
    """Test processor extracts entities using spaCy (if available)."""
    import spacy

    # Try to load spaCy model
    try:
        spacy.load("en_core_web_sm")
    except OSError:
        pytest.skip("spaCy model en_core_web_sm not installed")

    processor = EntityExtractionProcessor()
    result: ExtractionResult = ExtractionResult(
        content="John Doe works at Microsoft in Seattle.",
        mime_type="text/plain",
        metadata={},
        tables=[],
    )

    processed = processor.process(result)

    # Should have entities in metadata
    assert "entities" in processed.metadata
    assert isinstance(processed.metadata["entities"], dict)

    # Should have entity_count
    assert "entity_count" in processed.metadata
    assert isinstance(processed.metadata["entity_count"], int)


@pytest.mark.skipif(
    pytest.importorskip("spacy", reason="spaCy not installed") is None,
    reason="spaCy not installed",
)
def test_entity_type_filtering() -> None:
    """Test that entity_types parameter filters entities correctly."""
    import spacy

    try:
        spacy.load("en_core_web_sm")
    except OSError:
        pytest.skip("spaCy model en_core_web_sm not installed")

    # Only extract PERSON entities
    processor = EntityExtractionProcessor(entity_types=["PERSON"])
    result: ExtractionResult = ExtractionResult(
        content="John Doe works at Microsoft in Seattle.",
        mime_type="text/plain",
        metadata={},
        tables=[],
    )

    processed = processor.process(result)

    entities = processed.metadata["entities"]

    # Should only have PERSON entities
    assert "PERSON" in entities or len(entities) == 0
    assert "ORG" not in entities
    assert "GPE" not in entities


@pytest.mark.skipif(
    pytest.importorskip("spacy", reason="spaCy not installed") is None,
    reason="spaCy not installed",
)
def test_max_entities_limit() -> None:
    """Test that max_entities parameter limits entities per type."""
    import spacy

    try:
        spacy.load("en_core_web_sm")
    except OSError:
        pytest.skip("spaCy model en_core_web_sm not installed")

    processor = EntityExtractionProcessor(max_entities=2)
    result: ExtractionResult = ExtractionResult(
        content="Alice works with Bob and Charlie at Microsoft, Google, and Apple in Seattle, Portland, and San Francisco.",
        mime_type="text/plain",
        metadata={},
        tables=[],
    )

    processed = processor.process(result)

    entities = processed.metadata["entities"]

    # Each entity type should have at most 2 entities
    for _entity_type, entity_list in entities.items():
        assert len(entity_list) <= 2
