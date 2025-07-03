from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING, Final, cast

import anyio

from kreuzberg import ExtractionResult
from kreuzberg._chunker import get_chunker
from kreuzberg._mime_types import (
    validate_mime_type,
)
from kreuzberg._registry import ExtractorRegistry
from kreuzberg._types import ExtractionConfig
from kreuzberg._utils._string import safe_decode
from kreuzberg._utils._sync import run_maybe_async, run_maybe_sync

if TYPE_CHECKING:
    from collections.abc import Sequence
    from os import PathLike


DEFAULT_CONFIG: Final[ExtractionConfig] = ExtractionConfig()


async def _validate_and_post_process_async(result: ExtractionResult, config: ExtractionConfig) -> ExtractionResult:
    for validator in config.validators or []:
        await run_maybe_sync(validator, result)

    if config.chunk_content:
        result.chunks = _handle_chunk_content(
            mime_type=result.mime_type,
            config=config,
            content=result.content,
        )

    for post_processor in config.post_processing_hooks or []:
        result = await run_maybe_sync(post_processor, result)

    return result


def _validate_and_post_process_sync(result: ExtractionResult, config: ExtractionConfig) -> ExtractionResult:
    for validator in config.validators or []:
        run_maybe_async(validator, result)

    if config.chunk_content:
        result.chunks = _handle_chunk_content(
            mime_type=result.mime_type,
            config=config,
            content=result.content,
        )

    for post_processor in config.post_processing_hooks or []:
        result = run_maybe_async(post_processor, result)

    return result


def _handle_chunk_content(
    mime_type: str,
    config: ExtractionConfig,
    content: str,
) -> list[str]:
    chunker = get_chunker(mime_type=mime_type, max_characters=config.max_chars, overlap_characters=config.max_overlap)
    return chunker.chunks(content)


async def extract_bytes(content: bytes, mime_type: str, config: ExtractionConfig = DEFAULT_CONFIG) -> ExtractionResult:
    """Extract the textual content from a given byte string representing a file's contents.

    Args:
        content: The content to extract.
        mime_type: The mime type of the content.
        config: Extraction options object, defaults to the default object.


    Returns:
        The extracted content and the mime type of the content.
    """
    mime_type = validate_mime_type(mime_type=mime_type)
    if extractor := ExtractorRegistry.get_extractor(mime_type=mime_type, config=config):
        result = await extractor.extract_bytes_async(content)
    else:
        result = ExtractionResult(
            content=safe_decode(content),
            chunks=[],
            mime_type=mime_type,
            metadata={},
        )

    return await _validate_and_post_process_async(result=result, config=config)


async def extract_file(
    file_path: PathLike[str] | str, mime_type: str | None = None, config: ExtractionConfig = DEFAULT_CONFIG
) -> ExtractionResult:
    """Extract the textual content from a given file.

    Args:
        file_path: The path to the file.
        mime_type: The mime type of the content.
        config: Extraction options object, defaults to the default object.

    Returns:
        The extracted content and the mime type of the content.
    """
    mime_type = validate_mime_type(file_path=file_path, mime_type=mime_type)
    if extractor := ExtractorRegistry.get_extractor(mime_type=mime_type, config=config):
        result = await extractor.extract_path_async(Path(file_path))
    else:
        result = ExtractionResult(
            content=safe_decode(await anyio.Path(file_path).read_bytes()), chunks=[], mime_type=mime_type, metadata={}
        )

    return await _validate_and_post_process_async(result=result, config=config)


async def batch_extract_file(
    file_paths: Sequence[PathLike[str] | str], config: ExtractionConfig = DEFAULT_CONFIG
) -> list[ExtractionResult]:
    """Extract text from multiple files concurrently.

    Args:
        file_paths: A sequence of paths to files to extract text from.
        config: Extraction options object, defaults to the default object.

    Returns:
        A list of extraction results in the same order as the input paths.
    """
    results = cast("list[ExtractionResult]", ([None] * len(file_paths)))

    async def _extract_file(path: PathLike[str] | str, index: int) -> None:
        result = await extract_file(
            path,
            None,
            config,
        )
        results[index] = result

    async with anyio.create_task_group() as tg:
        for i, path in enumerate(file_paths):
            tg.start_soon(_extract_file, path, i)

    return results


async def batch_extract_bytes(
    contents: Sequence[tuple[bytes, str]], config: ExtractionConfig = DEFAULT_CONFIG
) -> list[ExtractionResult]:
    """Extract text from multiple byte contents concurrently.

    Args:
        contents: A sequence of tuples containing (content, mime_type) pairs.
        config: Extraction options object, defaults to the default object.

    Returns:
        A list of extraction results in the same order as the input contents.
    """
    results = cast("list[ExtractionResult]", [None] * len(contents))

    async def _extract_bytes(content: bytes, mime_type: str, index: int) -> None:
        result = await extract_bytes(content, mime_type, config)
        results[index] = result

    async with anyio.create_task_group() as tg:
        for i, (content, mime_type) in enumerate(contents):
            tg.start_soon(_extract_bytes, content, mime_type, i)

    return results


def extract_bytes_sync(content: bytes, mime_type: str, config: ExtractionConfig = DEFAULT_CONFIG) -> ExtractionResult:
    """Synchronous version of extract_bytes.

    Args:
        content: The content to extract.
        mime_type: The mime type of the content.
        config: Extraction options object, defaults to the default object.

    Returns:
        The extracted content and the mime type of the content.
    """
    mime_type = validate_mime_type(mime_type=mime_type)
    if extractor := ExtractorRegistry.get_extractor(mime_type=mime_type, config=config):
        result = extractor.extract_bytes_sync(content)
    else:
        result = ExtractionResult(
            content=safe_decode(content),
            chunks=[],
            mime_type=mime_type,
            metadata={},
        )

    return _validate_and_post_process_sync(result=result, config=config)


def extract_file_sync(
    file_path: Path | str, mime_type: str | None = None, config: ExtractionConfig = DEFAULT_CONFIG
) -> ExtractionResult:
    """Synchronous version of extract_file.

    Args:
        file_path: The path to the file.
        mime_type: The mime type of the content.
        config: Extraction options object, defaults to the default object.

    Returns:
        The extracted content and the mime type of the content.
    """
    mime_type = validate_mime_type(file_path=file_path, mime_type=mime_type)
    if extractor := ExtractorRegistry.get_extractor(mime_type=mime_type, config=config):
        result = extractor.extract_path_sync(Path(file_path))
    else:
        result = ExtractionResult(
            content=Path(file_path).read_text(),
            chunks=[],
            mime_type=mime_type,
            metadata={},
        )
    return _validate_and_post_process_sync(result=result, config=config)


def batch_extract_file_sync(
    file_paths: Sequence[PathLike[str] | str], config: ExtractionConfig = DEFAULT_CONFIG
) -> list[ExtractionResult]:
    """Synchronous version of batch_extract_file.

    Args:
        file_paths: A sequence of paths to files to extract text from.
        config: Extraction options object, defaults to the default object.

    Returns:
        A list of extraction results in the same order as the input paths.
    """
    return [extract_file_sync(file_path=Path(file_path), mime_type=None, config=config) for file_path in file_paths]


def batch_extract_bytes_sync(
    contents: Sequence[tuple[bytes, str]], config: ExtractionConfig = DEFAULT_CONFIG
) -> list[ExtractionResult]:
    """Synchronous version of batch_extract_bytes.

    Args:
        contents: A sequence of tuples containing (content, mime_type) pairs.
        config: Extraction options object, defaults to the default object.

    Returns:
        A list of extraction results in the same order as the input contents.
    """
    return [extract_bytes_sync(content=content, mime_type=mime_type, config=config) for content, mime_type in contents]
