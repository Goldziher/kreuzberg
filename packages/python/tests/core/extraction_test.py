from __future__ import annotations

from pathlib import Path
from unittest.mock import AsyncMock, Mock, patch

import pytest

from kreuzberg import ExtractionConfig, ExtractionResult, LanguageDetectionConfig
from kreuzberg import ChunkingConfig, EntityExtractionConfig, KeywordExtractionConfig
from kreuzberg.exceptions import ValidationError
from kreuzberg import (
    DEFAULT_CONFIG,
    _handle_chunk_content,
    _validate_and_post_process_async,
    _validate_and_post_process_helper,
    _validate_and_post_process_sync,
    extract_bytes,
    extract_bytes_sync,
    extract_file,
    extract_file_sync,
)


def test_default_config_is_extraction_config() -> None:
    assert isinstance(DEFAULT_CONFIG, ExtractionConfig)
    assert DEFAULT_CONFIG.use_cache is True
    assert DEFAULT_CONFIG.chunking is None


@pytest.mark.anyio
async def test_extract_bytes_with_valid_mime_type() -> None:
    content = b"Hello, World!"
    mime_type = "text/plain"

    result = await extract_bytes(content, mime_type)

    assert isinstance(result, ExtractionResult)
    assert result.content == "Hello, World!"
    assert result.mime_type == "text/plain"
    assert result.chunks == []
    assert "line_count" in result.metadata
    assert "word_count" in result.metadata
    assert "character_count" in result.metadata
    assert result.metadata["line_count"] == 1
    assert result.metadata["word_count"] == 2
    assert result.metadata["character_count"] == 13


@pytest.mark.anyio
async def test_extract_bytes_with_unknown_mime_type() -> None:
    content = b"Unknown content type"
    mime_type = "application/unknown"

    with patch("kreuzberg.extraction.validate_mime_type", return_value=mime_type):
        result = await extract_bytes(content, mime_type)

    assert result.content == "Unknown content type"
    assert result.mime_type == "application/unknown"


def test_extract_bytes_sync_with_valid_mime_type() -> None:
    content = b"Hello, World!"
    mime_type = "text/plain"

    result = extract_bytes_sync(content, mime_type)

    assert isinstance(result, ExtractionResult)
    assert result.content == "Hello, World!"
    assert result.mime_type == "text/plain"
    assert "line_count" in result.metadata
    assert "word_count" in result.metadata
    assert "character_count" in result.metadata
    assert result.metadata["line_count"] == 1
    assert result.metadata["word_count"] == 2
    assert result.metadata["character_count"] == 13


def test_extract_bytes_sync_with_unknown_mime_type() -> None:
    content = b"Unknown content type"
    mime_type = "application/unknown"

    with patch("kreuzberg.extraction.validate_mime_type", return_value=mime_type):
        result = extract_bytes_sync(content, mime_type)

    assert result.content == "Unknown content type"
    assert result.mime_type == "application/unknown"


@pytest.mark.anyio
async def test_extract_file_nonexistent_file() -> None:
    nonexistent_path = "/nonexistent/file.txt"

    with pytest.raises(ValidationError, match="The file does not exist"):
        await extract_file(nonexistent_path)


def test_extract_file_sync_nonexistent_file() -> None:
    nonexistent_path = "/nonexistent/file.txt"

    with pytest.raises(ValidationError, match="The file does not exist"):
        extract_file_sync(nonexistent_path)


@pytest.mark.anyio
async def test_extract_file_with_cache_disabled(tmp_path: Path) -> None:
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")

    config = ExtractionConfig(use_cache=False)
    result = await extract_file(test_file, config=config)

    assert result.content == "Test content"
    assert result.mime_type == "text/plain"


def test_extract_file_sync_with_cache_disabled(tmp_path: Path) -> None:
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")

    config = ExtractionConfig(use_cache=False)
    result = extract_file_sync(test_file, config=config)

    assert result.content == "Test content"
    assert result.mime_type == "text/plain"


@pytest.mark.anyio
async def test_extract_file_with_cache_hit(tmp_path: Path) -> None:
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content for cache")

    config = ExtractionConfig(use_cache=True)

    result1 = await extract_file(test_file, config=config)

    result2 = await extract_file(test_file, config=config)

    assert result1.content == result2.content
    assert result1.content == "Test content for cache"
    assert result1.mime_type == result2.mime_type == "text/plain"


def test_extract_file_sync_with_cache_hit(tmp_path: Path) -> None:
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content for cache sync")

    config = ExtractionConfig(use_cache=True)

    result1 = extract_file_sync(test_file, config=config)

    result2 = extract_file_sync(test_file, config=config)

    assert result1.content == result2.content
    assert result1.content == "Test content for cache sync"
    assert result1.mime_type == result2.mime_type == "text/plain"


@pytest.mark.anyio
async def test_extract_file_with_unknown_mime_type(tmp_path: Path) -> None:
    test_file = tmp_path / "test.unknown"
    test_file.write_text("Unknown file type content")

    with patch("kreuzberg.extraction.validate_mime_type", return_value="application/unknown"):
        result = await extract_file(test_file, mime_type="application/unknown")

    assert result.content == "Unknown file type content"
    assert result.mime_type == "application/unknown"


def test_extract_file_sync_with_unknown_mime_type(tmp_path: Path) -> None:
    test_file = tmp_path / "test.unknown"
    test_file.write_text("Unknown file type content")

    with patch("kreuzberg.extraction.validate_mime_type", return_value="application/unknown"):
        result = extract_file_sync(test_file, mime_type="application/unknown")

    assert result.content == "Unknown file type content"
    assert result.mime_type == "application/unknown"


def test_handle_chunk_content() -> None:
    content = "This is a long text that should be chunked into smaller pieces for processing."
    chunking_config = ChunkingConfig(max_chars=20, max_overlap=5)

    chunks = _handle_chunk_content(
        mime_type="text/plain",
        chunking_config=chunking_config,
        content=content,
    )

    assert chunks is not None
    assert len(chunks) > 1


@pytest.mark.anyio
async def test_extract_bytes_with_chunking() -> None:
    content = b"This is a long text that should be chunked into smaller pieces for processing."
    mime_type = "text/plain"
    config = ExtractionConfig(chunking=ChunkingConfig(max_chars=20, max_overlap=5))

    result = await extract_bytes(content, mime_type, config)

    assert result.chunks is not None
    assert len(result.chunks) > 1


def test_extract_bytes_sync_with_chunking() -> None:
    content = b"This is a long text that should be chunked into smaller pieces for processing."
    mime_type = "text/plain"
    config = ExtractionConfig(chunking=ChunkingConfig(max_chars=20, max_overlap=5))

    result = extract_bytes_sync(content, mime_type, config)

    assert result.chunks is not None
    assert len(result.chunks) > 1


@pytest.mark.anyio
async def test_extract_bytes_with_language_detection() -> None:
    content = b"This is some English text for language detection."
    mime_type = "text/plain"
    config = ExtractionConfig(language_detection=LanguageDetectionConfig())

    with patch("kreuzberg.extraction.detect_languages") as mock_detect:
        mock_detect.return_value = ["en"]
        result = await extract_bytes(content, mime_type, config)

    assert result.detected_languages == ["en"]
    mock_detect.assert_called_once()


def test_extract_bytes_sync_with_language_detection() -> None:
    content = b"This is some English text for language detection."
    mime_type = "text/plain"
    config = ExtractionConfig(language_detection=LanguageDetectionConfig())

    with patch("kreuzberg.extraction.detect_languages") as mock_detect:
        mock_detect.return_value = ["en"]
        result = extract_bytes_sync(content, mime_type, config)

    assert result.detected_languages == ["en"]
    mock_detect.assert_called_once()


@pytest.mark.anyio
async def test_extract_bytes_with_entity_extraction_success() -> None:
    content = b"John works at Apple Inc. in California."
    mime_type = "text/plain"
    config = ExtractionConfig(entities=EntityExtractionConfig())

    with patch("kreuzberg.extraction.extract_entities") as mock_extract:
        mock_entities = [{"text": "John", "label": "PERSON"}]
        mock_extract.return_value = mock_entities
        result = await extract_bytes(content, mime_type, config)

    assert result.entities == mock_entities  # type: ignore[comparison-overlap]
    mock_extract.assert_called_once()


@pytest.mark.anyio
async def test_extract_bytes_with_entity_extraction_runtime_error() -> None:
    content = b"Some text for entity extraction."
    mime_type = "text/plain"
    config = ExtractionConfig(entities=EntityExtractionConfig())

    with patch("kreuzberg.extraction.extract_entities") as mock_extract:
        mock_extract.side_effect = RuntimeError("Entity extraction failed")
        with pytest.raises(RuntimeError, match="Entity extraction failed"):
            await extract_bytes(content, mime_type, config)


def test_extract_bytes_sync_with_entity_extraction_runtime_error() -> None:
    content = b"Some text for entity extraction."
    mime_type = "text/plain"
    config = ExtractionConfig(entities=EntityExtractionConfig())

    with patch("kreuzberg.extraction.extract_entities") as mock_extract:
        mock_extract.side_effect = RuntimeError("Entity extraction failed")
        with pytest.raises(RuntimeError, match="Entity extraction failed"):
            extract_bytes_sync(content, mime_type, config)


@pytest.mark.anyio
async def test_extract_bytes_with_keyword_extraction_success() -> None:
    content = b"Machine learning and artificial intelligence are important technologies."
    mime_type = "text/plain"
    config = ExtractionConfig(keywords=KeywordExtractionConfig(count=5))

    with patch("kreuzberg.extraction.extract_keywords") as mock_extract:
        mock_keywords = ["machine", "learning", "artificial", "intelligence", "technologies"]
        mock_extract.return_value = mock_keywords
        result = await extract_bytes(content, mime_type, config)

    assert result.keywords == mock_keywords  # type: ignore[comparison-overlap]
    mock_extract.assert_called_once_with(result.content, keyword_count=5)


@pytest.mark.anyio
async def test_extract_bytes_with_keyword_extraction_runtime_error() -> None:
    content = b"Some text for keyword extraction."
    mime_type = "text/plain"
    config = ExtractionConfig(keywords=KeywordExtractionConfig())

    with patch("kreuzberg.extraction.extract_keywords") as mock_extract:
        mock_extract.side_effect = RuntimeError("Keyword extraction failed")
        with pytest.raises(RuntimeError, match="Keyword extraction failed"):
            await extract_bytes(content, mime_type, config)


def test_extract_bytes_sync_with_keyword_extraction_runtime_error() -> None:
    content = b"Some text for keyword extraction."
    mime_type = "text/plain"
    config = ExtractionConfig(keywords=KeywordExtractionConfig())

    with patch("kreuzberg.extraction.extract_keywords") as mock_extract:
        mock_extract.side_effect = RuntimeError("Keyword extraction failed")
        with pytest.raises(RuntimeError, match="Keyword extraction failed"):
            extract_bytes_sync(content, mime_type, config)


@pytest.mark.anyio
async def test_extract_bytes_with_auto_detect_document_type() -> None:
    content = b"This is a document with specific formatting."
    mime_type = "text/plain"
    config = ExtractionConfig(auto_detect_document_type=True)

    with patch("kreuzberg.extraction.auto_detect_document_type") as mock_detect:
        mock_result = ExtractionResult(
            content="This is a document with specific formatting.",
            mime_type="text/plain",
            metadata={"document_type": "article"},  # type: ignore[typeddict-unknown-key]
            chunks=[],
        )
        mock_detect.return_value = mock_result
        result = await extract_bytes(content, mime_type, config)

    assert result.metadata.get("document_type") == "article"
    mock_detect.assert_called_once()


@pytest.mark.anyio
async def test_validate_and_post_process_async_with_validators() -> None:
    result = ExtractionResult(content="Test content", mime_type="text/plain", metadata={}, chunks=[])

    async_validator = AsyncMock()
    sync_validator = Mock()

    config = ExtractionConfig(validators=(async_validator, sync_validator))

    processed_result = await _validate_and_post_process_async(result, config)

    async_validator.assert_called_once_with(result)
    sync_validator.assert_called_once_with(result)
    assert processed_result.content == "Test content"


@pytest.mark.anyio
async def test_validate_and_post_process_async_with_post_processors() -> None:
    result = ExtractionResult(content="Test content", mime_type="text/plain", metadata={}, chunks=[])

    modified_result = ExtractionResult(
        content="Modified content",
        mime_type="text/plain",
        metadata={"processed": True},  # type: ignore[typeddict-unknown-key]
        chunks=[],
    )

    async_processor = AsyncMock(return_value=modified_result)
    sync_processor = Mock(return_value=modified_result)

    config = ExtractionConfig(post_processing_hooks=(async_processor, sync_processor))

    with patch("kreuzberg.extraction._validate_and_post_process_helper", return_value=result):
        processed_result = await _validate_and_post_process_async(result, config)

    assert processed_result.content == "Modified content"
    assert processed_result.metadata.get("processed") is True


def test_validate_and_post_process_sync_with_validators() -> None:
    result = ExtractionResult(content="Test content", mime_type="text/plain", metadata={}, chunks=[])

    validator = Mock()
    config = ExtractionConfig(validators=(validator,))

    processed_result = _validate_and_post_process_sync(result, config)

    validator.assert_called_once_with(result)
    assert processed_result.content == "Test content"


def test_validate_and_post_process_sync_with_post_processors() -> None:
    result = ExtractionResult(content="Test content", mime_type="text/plain", metadata={}, chunks=[])

    modified_result = ExtractionResult(
        content="Modified content",
        mime_type="text/plain",
        metadata={"processed": True},  # type: ignore[typeddict-unknown-key]
        chunks=[],
    )

    processor = Mock(return_value=modified_result)
    config = ExtractionConfig(post_processing_hooks=(processor,))

    with patch("kreuzberg.extraction._validate_and_post_process_helper", return_value=result):
        processed_result = _validate_and_post_process_sync(result, config)

    assert processed_result.content == "Modified content"
    assert processed_result.metadata.get("processed") is True
    processor.assert_called_once_with(result)


def test_validate_and_post_process_helper_with_all_features() -> None:
    result = ExtractionResult(
        content="This is test content for processing.", mime_type="text/plain", metadata={}, chunks=[]
    )

    config = ExtractionConfig(
        chunking=ChunkingConfig(max_chars=10, max_overlap=2),
        entities=EntityExtractionConfig(),
        keywords=KeywordExtractionConfig(count=3),
        language_detection=LanguageDetectionConfig(),
        auto_detect_document_type=True,
    )

    with (
        patch("kreuzberg.extraction.get_chunker") as mock_chunker,
        patch("kreuzberg.extraction.extract_entities") as mock_entities,
        patch("kreuzberg.extraction.extract_keywords") as mock_keywords,
        patch("kreuzberg.extraction.detect_languages") as mock_languages,
        patch("kreuzberg.extraction.auto_detect_document_type") as mock_doc_type,
    ):
        mock_chunker_instance = Mock()
        mock_chunker_instance.chunks.return_value = ["chunk1", "chunk2"]
        mock_chunker.return_value = mock_chunker_instance

        mock_entities.return_value = [{"text": "test", "label": "MISC"}]
        mock_keywords.return_value = ["test", "content", "processing"]
        mock_languages.return_value = ["en"]
        mock_doc_type.return_value = result

        processed_result = _validate_and_post_process_helper(result, config, Path("/test/path.txt"))

    assert processed_result.chunks == ["chunk1", "chunk2"]
    assert processed_result.entities == [{"text": "test", "label": "MISC"}]  # type: ignore[comparison-overlap]
    assert processed_result.keywords == ["test", "content", "processing"]  # type: ignore[comparison-overlap]
    assert processed_result.detected_languages == ["en"]

    mock_chunker.assert_called_once_with(mime_type="text/plain", max_characters=10, overlap_characters=2)
    mock_entities.assert_called_once_with(result.content, custom_patterns=None, spacy_config=config.entities)
    mock_keywords.assert_called_once_with(result.content, keyword_count=3)
    mock_languages.assert_called_once()
    mock_doc_type.assert_called_once_with(result, config, file_path=Path("/test/path.txt"))


@pytest.mark.anyio
async def test_extract_bytes_with_html_extractor() -> None:
    content = b"<html><body><h1>Test HTML</h1></body></html>"
    mime_type = "text/html"

    result = await extract_bytes(content, mime_type)

    assert "Test HTML" in result.content
    assert result.mime_type == "text/markdown"


def test_extract_bytes_sync_with_html_extractor() -> None:
    content = b"<html><body><h1>Test HTML</h1></body></html>"
    mime_type = "text/html"

    result = extract_bytes_sync(content, mime_type)

    assert "Test HTML" in result.content
    assert result.mime_type == "text/markdown"


@pytest.mark.anyio
async def test_extract_file_with_html_extractor(tmp_path: Path) -> None:
    test_file = tmp_path / "test.html"
    test_file.write_text("<html><body><h1>Test HTML File</h1></body></html>")

    result = await extract_file(test_file, mime_type="text/html")

    assert "Test HTML File" in result.content
    assert result.mime_type == "text/markdown"


def test_extract_file_sync_with_html_extractor(tmp_path: Path) -> None:
    test_file = tmp_path / "test.html"
    test_file.write_text("<html><body><h1>Test HTML File</h1></body></html>")

    result = extract_file_sync(test_file, mime_type="text/html")

    assert "Test HTML File" in result.content
    assert result.mime_type == "text/markdown"
