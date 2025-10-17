from __future__ import annotations

from typing import Literal

import pytest

from kreuzberg import (
    ChunkingConfig,
    EntityExtractionConfig,
    ExtractionConfig,
    LanguageDetectionConfig,
    TokenReductionConfig,
    extract_bytes_sync,
)


def test_token_reduction_integration_off_mode() -> None:
    content = b"The quick brown fox jumps over the lazy dog."
    config = ExtractionConfig(token_reduction=TokenReductionConfig(mode="off"))

    result = extract_bytes_sync(content, "text/plain", config)

    assert result.content == "The quick brown fox jumps over the lazy dog."
    assert "token_reduction" not in result.metadata


def test_token_reduction_integration_light_mode() -> None:
    content = b"The   quick   brown   fox   jumps   over   the   lazy   dog."
    config = ExtractionConfig(token_reduction=TokenReductionConfig(mode="light"))

    result = extract_bytes_sync(content, "text/plain", config)

    assert result.content == "The quick brown fox jumps over the lazy dog."
    assert "token_reduction" in result.metadata
    assert "character_reduction_ratio" in result.metadata["token_reduction"]
    assert "token_reduction_ratio" in result.metadata["token_reduction"]


def test_token_reduction_integration_moderate_mode() -> None:
    content = b"The quick brown fox jumps over the lazy dog."
    config = ExtractionConfig(token_reduction=TokenReductionConfig(mode="moderate"))

    result = extract_bytes_sync(content, "text/plain", config)

    assert len(result.content) < len(content)
    assert "quick" in result.content
    assert "brown" in result.content
    assert "fox" in result.content
    assert "token_reduction" in result.metadata


def test_token_reduction_integration_with_language_detection() -> None:
    content = b"""Le chat est sur le tapis. Le chat est sur le tapis. Le chat est sur le tapis.
    Le chien joue dans le jardin. Le chien joue dans le jardin. Le chien joue dans le jardin.
    Les oiseaux chantent dans les arbres. Les oiseaux chantent dans les arbres.
    Cette phrase est repetee plusieurs fois. Cette phrase est repetee plusieurs fois.
    Cette phrase est repetee plusieurs fois. Cette phrase est repetee plusieurs fois."""
    config = ExtractionConfig(
        language_detection=LanguageDetectionConfig(),
        token_reduction=TokenReductionConfig(mode="moderate"),
    )

    result = extract_bytes_sync(content, "text/plain", config)

    assert result.detected_languages is not None
    assert len(result.detected_languages) > 0
    assert "token_reduction" in result.metadata
    assert "token_reduction" in result.metadata


def test_token_reduction_integration_markdown_preservation() -> None:
    content = b"""# Main Header

The quick brown fox jumps over the lazy dog.

## Sub Header

More text with the and a and some stopwords.

```python
def function():
    return "code should be preserved"
```

- List item with the word the
- Another item with some stopwords
"""
    config = ExtractionConfig(token_reduction=TokenReductionConfig(mode="moderate", preserve_markdown=True))

    result = extract_bytes_sync(content, "text/markdown", config)

    assert "# Main Header" in result.content
    assert "## Sub Header" in result.content
    assert "```python" in result.content
    assert "def function():" in result.content
    assert "- List item" in result.content
    assert "token_reduction" in result.metadata


def test_token_reduction_integration_with_custom_stopwords() -> None:
    content = b"The custom word should be removed but other words remain."
    config = ExtractionConfig(
        token_reduction=TokenReductionConfig(mode="moderate", custom_stopwords={"en": ["custom", "should"]}),
    )

    result = extract_bytes_sync(content, "text/plain", config)

    assert "custom" not in result.content
    assert "should" not in result.content
    assert "word" in result.content
    assert "removed" in result.content
    assert "token_reduction" in result.metadata


def test_token_reduction_integration_preserves_entities() -> None:
    content = b"John Doe works at OpenAI in San Francisco."
    config = ExtractionConfig(
        entities=EntityExtractionConfig(),
        token_reduction=TokenReductionConfig(mode="moderate"),
    )

    try:
        result = extract_bytes_sync(content, "text/plain", config)

        assert "John" in result.content
        assert "Doe" in result.content
        assert "OpenAI" in result.content
        assert "Francisco" in result.content
        assert "token_reduction" in result.metadata
    except Exception as e:
        if "spacy" in str(e).lower() or "model" in str(e).lower():
            import pytest

            pytest.skip(f"Spacy model not available: {e}")
        raise


def test_token_reduction_integration_with_chunking() -> None:
    content = b"The quick brown fox jumps over the lazy dog and the cat and the bird. " * 10
    config = ExtractionConfig(
        chunking=ChunkingConfig(max_chars=200, max_overlap=50),
        token_reduction=TokenReductionConfig(mode="moderate"),
    )

    result = extract_bytes_sync(content, "text/plain", config)

    assert len(result.chunks) > 0
    assert "token_reduction" in result.metadata
    assert (
        result.metadata["token_reduction"]["original_characters"]
        > result.metadata["token_reduction"]["reduced_characters"]
    )


@pytest.mark.parametrize("mode", ["light", "moderate"])
def test_token_reduction_integration_all_modes_work_with_pipeline(mode: Literal["light", "moderate"]) -> None:
    content = b"The quick brown fox jumps over the lazy dog with many words."
    config = ExtractionConfig(token_reduction=TokenReductionConfig(mode=mode))

    result = extract_bytes_sync(content, "text/plain", config)

    assert len(result.content) > 0
    assert "token_reduction" in result.metadata
    assert isinstance(result.metadata["token_reduction"], dict)
    assert "character_reduction_ratio" in result.metadata["token_reduction"]


def test_token_reduction_integration_stats_accuracy() -> None:
    content = b"The quick brown fox jumps over the lazy dog."
    config = ExtractionConfig(token_reduction=TokenReductionConfig(mode="light"))

    result = extract_bytes_sync(content, "text/plain", config)

    stats = result.metadata["token_reduction"]
    assert stats["original_characters"] == len(content.decode())
    assert stats["reduced_characters"] == len(result.content)
    assert stats["original_tokens"] == len(content.decode().split())
    assert stats["reduced_tokens"] == len(result.content.split())


def test_token_reduction_integration_no_config_provided() -> None:
    content = b"The quick brown fox jumps over the lazy dog."
    config = ExtractionConfig(token_reduction=None)

    result = extract_bytes_sync(content, "text/plain", config)

    assert result.content == content.decode()
    assert "token_reduction" not in result.metadata
