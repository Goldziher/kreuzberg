from __future__ import annotations

from unittest.mock import Mock, patch

import pytest

from kreuzberg import ExtractionResult, extract_file
from kreuzberg._types import ExtractionConfig
from kreuzberg.extraction import extract_file_sync


def sync_validation_hook(result: ExtractionResult) -> None:
    if not result.content:
        raise ValueError("Content cannot be empty")


async def async_validation_hook(result: ExtractionResult) -> None:
    if not result.content:
        raise ValueError("Content cannot be empty")


def sync_post_processing_hook(result: ExtractionResult) -> ExtractionResult:
    return ExtractionResult(
        content=result.content.upper(), mime_type=result.mime_type, metadata=result.metadata, chunks=[]
    )


async def async_post_processing_hook(result: ExtractionResult) -> ExtractionResult:
    return ExtractionResult(
        content=result.content.upper(), mime_type=result.mime_type, metadata=result.metadata, chunks=[]
    )


@pytest.mark.anyio
async def test_async_validation_hook() -> None:
    mock_validation = Mock(side_effect=async_validation_hook)

    config = ExtractionConfig(validators=[mock_validation])

    with (
        patch("kreuzberg.extraction.validate_mime_type", return_value="text/plain"),
        patch("kreuzberg.extraction.ExtractorRegistry.get_extractor", return_value=None),
        patch("anyio.Path.read_bytes", return_value=b"test content"),
    ):
        result = await extract_file("test.txt", config=config)

        mock_validation.assert_called_once()
        assert result.content == "test content"


@pytest.mark.anyio
async def test_async_validation_hook_error() -> None:
    async def failing_validation_hook(_: ExtractionResult) -> None:
        raise ValueError("Validation failed")

    config = ExtractionConfig(validators=[failing_validation_hook])

    with (
        patch("kreuzberg.extraction.validate_mime_type", return_value="text/plain"),
        patch("kreuzberg.extraction.ExtractorRegistry.get_extractor", return_value=None),
        patch("anyio.Path.read_bytes", return_value=b"test content"),
        pytest.raises(ValueError, match="Validation failed"),
    ):
        await extract_file("test.txt", config=config)


@pytest.mark.anyio
async def test_async_post_processing_hook() -> None:
    mock_post_processor = Mock(side_effect=async_post_processing_hook)

    config = ExtractionConfig(post_processing_hooks=[mock_post_processor])

    with (
        patch("kreuzberg.extraction.validate_mime_type", return_value="text/plain"),
        patch("kreuzberg.extraction.ExtractorRegistry.get_extractor", return_value=None),
        patch("anyio.Path.read_bytes", return_value=b"test content"),
    ):
        result = await extract_file("test.txt", config=config)

        mock_post_processor.assert_called_once()
        assert result.content == "TEST CONTENT"


@pytest.mark.anyio
async def test_multiple_async_post_processing_hooks() -> None:
    async def second_post_processor(result: ExtractionResult) -> ExtractionResult:
        return ExtractionResult(
            content=f"Processed: {result.content}", mime_type=result.mime_type, metadata=result.metadata, chunks=[]
        )

    config = ExtractionConfig(post_processing_hooks=[async_post_processing_hook, second_post_processor])

    with (
        patch("kreuzberg.extraction.validate_mime_type", return_value="text/plain"),
        patch("kreuzberg.extraction.ExtractorRegistry.get_extractor", return_value=None),
        patch("anyio.Path.read_bytes", return_value=b"test content"),
    ):
        result = await extract_file("test.txt", config=config)

        assert result.content == "Processed: TEST CONTENT"


def test_sync_validation_hook() -> None:
    mock_validation = Mock(side_effect=sync_validation_hook)

    config = ExtractionConfig(validators=[mock_validation])

    with (
        patch("kreuzberg.extraction.validate_mime_type", return_value="text/plain"),
        patch("kreuzberg.extraction.ExtractorRegistry.get_extractor", return_value=None),
        patch("pathlib.Path.read_text", return_value="test content"),
    ):
        result = extract_file_sync("test.txt", config=config)

        mock_validation.assert_called_once()
        assert result.content == "test content"


def test_sync_validation_hook_error() -> None:
    def failing_validation_hook(_: ExtractionResult) -> None:
        raise ValueError("Validation failed")

    config = ExtractionConfig(validators=[failing_validation_hook])

    with (
        patch("kreuzberg.extraction.validate_mime_type", return_value="text/plain"),
        patch("kreuzberg.extraction.ExtractorRegistry.get_extractor", return_value=None),
        patch("pathlib.Path.read_text", return_value="test content"),
        pytest.raises(ValueError, match="Validation failed"),
    ):
        extract_file_sync("test.txt", config=config)


def test_sync_post_processing_hook() -> None:
    mock_post_processor = Mock(side_effect=sync_post_processing_hook)

    config = ExtractionConfig(post_processing_hooks=[mock_post_processor])

    with (
        patch("kreuzberg.extraction.validate_mime_type", return_value="text/plain"),
        patch("kreuzberg.extraction.ExtractorRegistry.get_extractor", return_value=None),
        patch("pathlib.Path.read_text", return_value="test content"),
    ):
        result = extract_file_sync("test.txt", config=config)

        mock_post_processor.assert_called_once()
        assert result.content == "TEST CONTENT"


def test_multiple_sync_post_processing_hooks() -> None:
    def second_post_processor(result: ExtractionResult) -> ExtractionResult:
        return ExtractionResult(
            content=f"Processed: {result.content}", mime_type=result.mime_type, metadata=result.metadata, chunks=[]
        )

    config = ExtractionConfig(post_processing_hooks=[sync_post_processing_hook, second_post_processor])

    with (
        patch("kreuzberg.extraction.validate_mime_type", return_value="text/plain"),
        patch("kreuzberg.extraction.ExtractorRegistry.get_extractor", return_value=None),
        patch("pathlib.Path.read_text", return_value="test content"),
    ):
        result = extract_file_sync("test.txt", config=config)

        assert result.content == "Processed: TEST CONTENT"


def test_mixing_sync_and_async_hooks_in_sync_context() -> None:
    config = ExtractionConfig(validators=[sync_validation_hook], post_processing_hooks=[sync_post_processing_hook])

    with (
        patch("kreuzberg.extraction.validate_mime_type", return_value="text/plain"),
        patch("kreuzberg.extraction.ExtractorRegistry.get_extractor", return_value=None),
        patch("pathlib.Path.read_text", return_value="test content"),
    ):
        result = extract_file_sync("test.txt", config=config)

        assert result.content == "TEST CONTENT"


@pytest.mark.anyio
async def test_mixing_sync_and_async_hooks_in_async_context() -> None:
    config = ExtractionConfig(
        validators=[sync_validation_hook, async_validation_hook],
        post_processing_hooks=[sync_post_processing_hook, async_post_processing_hook],
    )

    with (
        patch("kreuzberg.extraction.validate_mime_type", return_value="text/plain"),
        patch("kreuzberg.extraction.ExtractorRegistry.get_extractor", return_value=None),
        patch("anyio.Path.read_bytes", return_value=b"test content"),
    ):
        result = await extract_file("test.txt", config=config)

        assert result.content == "TEST CONTENT"
