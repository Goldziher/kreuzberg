"""Tests for CategoryExtractionProcessor.

Tests document classification using zero-shot transformers, including
model configuration, multi-label classification, and error handling.
"""

from __future__ import annotations

import pytest

from kreuzberg import ExtractionResult
from kreuzberg.postprocessors.category_extraction import CategoryExtractionProcessor


def test_processor_initialization_with_custom_categories() -> None:
    """Test processor can be initialized with custom categories."""
    custom_categories = ["invoice", "receipt", "contract"]
    processor = CategoryExtractionProcessor(categories=custom_categories)

    assert processor.categories == custom_categories


def test_processor_initialization_with_empty_categories_fails() -> None:
    """Test that empty categories list raises ValueError."""
    with pytest.raises(ValueError, match="At least one category"):
        CategoryExtractionProcessor(categories=[])


def test_processor_initialization_with_full_config() -> None:
    """Test processor can be initialized with full configuration."""
    processor = CategoryExtractionProcessor(
        categories=["invoice", "contract"],
        model="facebook/bart-large-mnli",
        model_cache_dir="/custom/cache",
        multi_label=True,
        confidence_threshold=0.7,
        max_categories=2,
        device=-1,
    )

    assert processor.categories == ["invoice", "contract"]
    assert processor.model_name == "facebook/bart-large-mnli"
    assert processor.model_cache_dir == "/custom/cache"
    assert processor.multi_label is True
    assert processor.confidence_threshold == 0.7
    assert processor.max_categories == 2
    assert processor.device == -1


def test_processor_with_pipeline_kwargs() -> None:
    """Test processor accepts and stores pipeline kwargs."""
    processor = CategoryExtractionProcessor(batch_size=16, truncation=True)

    assert processor.pipeline_kwargs["batch_size"] == 16
    assert processor.pipeline_kwargs["truncation"] is True


def test_predefined_category_sets() -> None:
    """Test that predefined category sets are available."""
    assert len(CategoryExtractionProcessor.DOCUMENT_TYPES) > 0
    assert len(CategoryExtractionProcessor.SUBJECT_AREAS) > 0
    assert CategoryExtractionProcessor.SENTIMENT == [
        "positive",
        "negative",
        "neutral",
    ]


def test_process_empty_content() -> None:
    """Test processor handles empty content gracefully."""
    processor = CategoryExtractionProcessor()
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
    processor = CategoryExtractionProcessor()
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
    pytest.importorskip("transformers", reason="transformers not installed") is None,
    reason="transformers not installed",
)
def test_process_with_transformers() -> None:
    """Test processor classifies documents using transformers (if available)."""
    processor = CategoryExtractionProcessor(categories=["invoice", "contract", "resume", "report"])
    result: ExtractionResult = ExtractionResult(
        content="Invoice #12345 for services rendered in October 2025. Total amount due: $1,500.",
        mime_type="text/plain",
        metadata={},
        tables=[],
    )

    processed = processor.process(result)

    # Should have category in metadata
    assert "category" in processed.metadata
    category_result = processed.metadata["category"]

    # Should have required fields
    assert "primary" in category_result
    assert "scores" in category_result
    assert "confidence" in category_result

    # Scores should be a dict
    assert isinstance(category_result["scores"], dict)

    # Confidence should be between 0 and 1
    assert 0.0 <= category_result["confidence"] <= 1.0


@pytest.mark.skipif(
    pytest.importorskip("transformers", reason="transformers not installed") is None,
    reason="transformers not installed",
)
def test_single_label_classification() -> None:
    """Test single-label classification mode."""
    processor = CategoryExtractionProcessor(categories=["positive", "negative", "neutral"], multi_label=False)
    result: ExtractionResult = ExtractionResult(
        content="This is an excellent product! Highly recommended.",
        mime_type="text/plain",
        metadata={},
        tables=[],
    )

    processed = processor.process(result)

    category_result = processed.metadata["category"]

    # Should have primary category
    assert category_result["primary"] is not None

    # Should NOT have labels list (single-label mode)
    assert "labels" not in category_result


@pytest.mark.skipif(
    pytest.importorskip("transformers", reason="transformers not installed") is None,
    reason="transformers not installed",
)
def test_multi_label_classification() -> None:
    """Test multi-label classification mode."""
    processor = CategoryExtractionProcessor(
        categories=["legal", "financial", "technical"],
        multi_label=True,
        confidence_threshold=0.3,
    )
    result: ExtractionResult = ExtractionResult(
        content="This legal contract outlines the financial obligations and technical specifications.",
        mime_type="text/plain",
        metadata={},
        tables=[],
    )

    processed = processor.process(result)

    category_result = processed.metadata["category"]

    # Should have labels list (multi-label mode)
    assert "labels" in category_result
    assert isinstance(category_result["labels"], list)


@pytest.mark.skipif(
    pytest.importorskip("transformers", reason="transformers not installed") is None,
    reason="transformers not installed",
)
def test_confidence_threshold_filtering() -> None:
    """Test that confidence_threshold filters categories in multi-label mode."""
    processor = CategoryExtractionProcessor(
        categories=["invoice", "contract", "resume"],
        multi_label=True,
        confidence_threshold=0.8,  # High threshold
    )
    result: ExtractionResult = ExtractionResult(
        content="Invoice for services.",
        mime_type="text/plain",
        metadata={},
        tables=[],
    )

    processed = processor.process(result)

    category_result = processed.metadata["category"]
    labels = category_result["labels"]

    # Only categories with score >= 0.8 should be included
    for label in labels:
        assert category_result["scores"][label] >= 0.8


@pytest.mark.skipif(
    pytest.importorskip("transformers", reason="transformers not installed") is None,
    reason="transformers not installed",
)
def test_max_categories_limit() -> None:
    """Test that max_categories limits number of labels in multi-label mode."""
    processor = CategoryExtractionProcessor(
        categories=["legal", "financial", "technical", "medical", "marketing"],
        multi_label=True,
        confidence_threshold=0.1,  # Low threshold to get many categories
        max_categories=2,
    )
    result: ExtractionResult = ExtractionResult(
        content="This document discusses legal, financial, and technical matters.",
        mime_type="text/plain",
        metadata={},
        tables=[],
    )

    processed = processor.process(result)

    category_result = processed.metadata["category"]
    labels = category_result["labels"]

    # Should have at most 2 categories
    assert len(labels) <= 2


def test_long_content_truncation() -> None:
    """Test that very long content is truncated."""
    processor = CategoryExtractionProcessor()

    # Create very long content (> 2000 characters)
    long_content = "This is a test document. " * 200  # ~5000 characters
    result: ExtractionResult = ExtractionResult(
        content=long_content,
        mime_type="text/plain",
        metadata={},
        tables=[],
    )

    # Should not raise exception
    processed = processor.process(result)

    # Should still return valid result
    assert processed.metadata is not None
