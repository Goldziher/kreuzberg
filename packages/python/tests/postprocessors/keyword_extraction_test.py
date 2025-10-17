"""Tests for KeywordExtractionProcessor.

Tests keyword extraction using KeyBERT with sentence-transformers, including
model configuration, scoring, and error handling.
"""

from __future__ import annotations

import pytest

from kreuzberg import ExtractionResult
from kreuzberg.postprocessors.keyword_extraction import KeywordExtractionProcessor


def test_processor_initialization_with_custom_config() -> None:
    """Test processor can be initialized with custom configuration."""
    processor = KeywordExtractionProcessor(
        model="paraphrase-MiniLM-L6-v2",
        model_cache_dir="/custom/cache",
        top_n=5,
        ngram_range=(1, 3),
        min_score=0.3,
    )

    assert processor.model_name == "paraphrase-MiniLM-L6-v2"
    assert processor.model_cache_dir == "/custom/cache"
    assert processor.top_n == 5
    assert processor.ngram_range == (1, 3)
    assert processor.min_score == 0.3


def test_processor_with_model_kwargs() -> None:
    """Test processor accepts and stores model kwargs."""
    processor = KeywordExtractionProcessor(device="cpu", batch_size=32)

    assert processor.model_kwargs["device"] == "cpu"
    assert processor.model_kwargs["batch_size"] == 32


def test_process_empty_content() -> None:
    """Test processor handles empty content gracefully."""
    processor = KeywordExtractionProcessor()
    result = ExtractionResult(content="", mime_type="text/plain", metadata={}, tables=[])

    processed = processor.process(result)

    # Should return result unchanged
    assert processed.content == ""


def test_process_non_string_content() -> None:
    """Test processor handles non-string content gracefully by converting to string."""
    processor = KeywordExtractionProcessor()
    # Content should be string, but test robustness
    result = ExtractionResult(content="12345", mime_type="text/plain", metadata={}, tables=[])

    processed = processor.process(result)

    # Should return result unchanged
    assert processed.content == "12345"


@pytest.mark.skipif(
    pytest.importorskip("keybert", reason="KeyBERT not installed") is None,
    reason="KeyBERT not installed",
)
def test_process_with_keybert() -> None:
    """Test processor extracts keywords using KeyBERT (if available)."""
    processor = KeywordExtractionProcessor(top_n=3)
    result = ExtractionResult(
        content="Machine learning and artificial intelligence are transforming document processing and text extraction.",
        mime_type="text/plain",
        metadata={},
        tables=[],
    )

    processed = processor.process(result)

    # Should have keywords in metadata
    assert "keywords" in processed.metadata
    assert isinstance(processed.metadata["keywords"], list)

    keywords = processed.metadata["keywords"]

    # Each keyword should have keyword and score
    for kw in keywords:
        assert "keyword" in kw
        assert "score" in kw
        assert isinstance(kw["keyword"], str)
        assert isinstance(kw["score"], float)
        assert 0.0 <= kw["score"] <= 1.0

    # Should respect top_n limit
    assert len(keywords) <= 3


@pytest.mark.skipif(
    pytest.importorskip("keybert", reason="KeyBERT not installed") is None,
    reason="KeyBERT not installed",
)
def test_top_n_parameter() -> None:
    """Test that top_n parameter limits number of keywords."""
    processor = KeywordExtractionProcessor(top_n=2)
    result = ExtractionResult(
        content="Machine learning, artificial intelligence, deep learning, neural networks, and natural language processing are all related fields.",
        mime_type="text/plain",
        metadata={},
        tables=[],
    )

    processed = processor.process(result)

    keywords = processed.metadata["keywords"]
    assert len(keywords) <= 2


@pytest.mark.skipif(
    pytest.importorskip("keybert", reason="KeyBERT not installed") is None,
    reason="KeyBERT not installed",
)
def test_min_score_filtering() -> None:
    """Test that min_score parameter filters low-scoring keywords."""
    processor = KeywordExtractionProcessor(top_n=10, min_score=0.5)
    result = ExtractionResult(
        content="Machine learning and artificial intelligence are transforming document processing.",
        mime_type="text/plain",
        metadata={},
        tables=[],
    )

    processed = processor.process(result)

    keywords = processed.metadata["keywords"]

    # All keywords should have score >= min_score
    for kw in keywords:
        assert kw["score"] >= 0.5


@pytest.mark.skipif(
    pytest.importorskip("keybert", reason="KeyBERT not installed") is None,
    reason="KeyBERT not installed",
)
def test_ngram_range_parameter() -> None:
    """Test that ngram_range parameter affects extracted keywords."""
    # Extract only single words
    processor_unigram = KeywordExtractionProcessor(ngram_range=(1, 1), top_n=5)
    result = ExtractionResult(
        content="Machine learning and artificial intelligence are transforming document processing.",
        mime_type="text/plain",
        metadata={},
        tables=[],
    )

    processed = processor_unigram.process(result)
    keywords_unigram = processed.metadata["keywords"]

    # All keywords should be single words
    for kw in keywords_unigram:
        assert " " not in kw["keyword"]
