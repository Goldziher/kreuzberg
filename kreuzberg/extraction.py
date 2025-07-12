from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING, Any, Final, cast

import anyio

from kreuzberg import ExtractionResult
from kreuzberg._chunker import get_chunker
from kreuzberg._entity_extraction import extract_entities, extract_keywords
from kreuzberg._language_detection import detect_languages
from kreuzberg._mime_types import (
    validate_mime_type,
)
from kreuzberg._registry import ExtractorRegistry
from kreuzberg._types import ExtractionConfig
from kreuzberg._utils._string import safe_decode
from kreuzberg._utils._sync import run_maybe_sync, run_sync_only
from kreuzberg.exceptions import ValidationError

if TYPE_CHECKING:
    from collections.abc import Sequence
    from os import PathLike


DEFAULT_CONFIG: Final[ExtractionConfig] = ExtractionConfig()


def _validate_and_post_process_helper(
    result: ExtractionResult, config: ExtractionConfig, file_path: Path | None = None
) -> ExtractionResult:
    if config.chunk_content:
        result.chunks = _handle_chunk_content(
            mime_type=result.mime_type,
            config=config,
            content=result.content,
        )

    if config.extract_entities:
        try:
            result.entities = extract_entities(
                result.content,
                custom_patterns=config.custom_entity_patterns,
            )
        except RuntimeError:
            result.entities = None

    if config.extract_keywords:
        try:
            result.keywords = extract_keywords(
                result.content,
                keyword_count=config.keyword_count,
            )
        except RuntimeError:
            result.keywords = None

    if config.auto_detect_language:
        result.detected_languages = detect_languages(
            result.content,
            config=config.language_detection_config,
        )

    if config.auto_detect_document_type:
        if config.document_classification_mode == "vision" and file_path:
            import anyio

            from kreuzberg._document_classification import classify_document_from_layout
            from kreuzberg._ocr import get_ocr_backend

            async def _get_layout() -> ExtractionResult:
                return await get_ocr_backend("tesseract").process_file(file_path, **config.get_config_dict())

            layout_result = anyio.run(_get_layout)
            result.document_type, result.type_confidence = classify_document_from_layout(layout_result, config)
        else:
            from kreuzberg._document_classification import classify_document

            result.document_type, result.type_confidence = classify_document(result, config)

    return result


async def _validate_and_post_process_async(
    result: ExtractionResult, config: ExtractionConfig, file_path: Path | None = None
) -> ExtractionResult:
    for validator in config.validators or []:
        await run_maybe_sync(validator, result)

    result = _validate_and_post_process_helper(result, config, file_path)

    for post_processor in config.post_processing_hooks or []:
        result = await run_maybe_sync(post_processor, result)

    return result


def _validate_and_post_process_sync(
    result: ExtractionResult, config: ExtractionConfig, file_path: Path | None = None
) -> ExtractionResult:
    for validator in config.validators or []:
        run_sync_only(validator, result)

    result = _validate_and_post_process_helper(result, config, file_path)

    for post_processor in config.post_processing_hooks or []:
        result = run_sync_only(post_processor, result)

    if hasattr(result, "layout"):
        result.layout = None

    return result


def _handle_chunk_content(
    mime_type: str,
    config: ExtractionConfig,
    content: str,
) -> Any:
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

    Raises:
        ValidationError: If the file path or configuration is invalid.
    """
    from kreuzberg._utils._document_cache import get_document_cache

    cache = get_document_cache()
    path = Path(file_path)
    cached_result = cache.get(path, config)
    if cached_result is not None:
        return cached_result

    if cache.is_processing(path, config):
        event = cache.mark_processing(path, config)
        await anyio.to_thread.run_sync(event.wait)  # pragma: no cover

        # Try cache again after waiting for other process to complete  # ~keep
        cached_result = cache.get(path, config)  # pragma: no cover
        if cached_result is not None:  # pragma: no cover
            return cached_result

    cache.mark_processing(path, config)

    try:
        if not path.exists():
            raise ValidationError("The file does not exist", context={"file_path": str(path)})

        mime_type = validate_mime_type(file_path=file_path, mime_type=mime_type)
        if extractor := ExtractorRegistry.get_extractor(mime_type=mime_type, config=config):
            result = await extractor.extract_path_async(Path(file_path))
        else:
            result = ExtractionResult(
                content=safe_decode(await anyio.Path(file_path).read_bytes()),
                chunks=[],
                mime_type=mime_type,
                metadata={},
            )

        result = await _validate_and_post_process_async(result=result, config=config, file_path=path)

        cache.set(path, config, result)

        return result
    finally:
        cache.mark_complete(path, config)


async def batch_extract_file(
    file_paths: Sequence[PathLike[str] | str], config: ExtractionConfig = DEFAULT_CONFIG
) -> list[ExtractionResult]:
    """Extract text from multiple files concurrently with optimizations.

    Args:
        file_paths: A sequence of paths to files to extract text from.
        config: Extraction options object, defaults to the default object.

    Returns:
        A list of extraction results in the same order as the input paths.
    """
    if not file_paths:
        return []

    import multiprocessing as mp

    max_concurrency = min(len(file_paths), mp.cpu_count() * 2)
    semaphore = anyio.Semaphore(max_concurrency)

    results = cast("list[ExtractionResult]", ([None] * len(file_paths)))

    async def _extract_file(path: PathLike[str] | str, index: int) -> None:
        async with semaphore:
            try:
                result = await extract_file(
                    path,
                    None,
                    config,
                )
                results[index] = result
            except Exception as e:  # noqa: BLE001
                from kreuzberg._utils._errors import create_error_context

                error_result = ExtractionResult(
                    content=f"Error: {type(e).__name__}: {e!s}",
                    mime_type="text/plain",
                    metadata={  # type: ignore[typeddict-unknown-key]
                        "error": True,
                        "error_context": create_error_context(
                            operation="batch_extract_file",
                            file_path=path,
                            error=e,
                            index=index,
                        ),
                    },
                    chunks=[],
                )
                results[index] = error_result

    async with anyio.create_task_group() as tg:
        for i, path in enumerate(file_paths):
            tg.start_soon(_extract_file, path, i)

    return results


async def batch_extract_bytes(
    contents: Sequence[tuple[bytes, str]], config: ExtractionConfig = DEFAULT_CONFIG
) -> list[ExtractionResult]:
    """Extract text from multiple byte contents concurrently with optimizations.

    Args:
        contents: A sequence of tuples containing (content, mime_type) pairs.
        config: Extraction options object, defaults to the default object.

    Returns:
        A list of extraction results in the same order as the input contents.
    """
    if not contents:
        return []

    import multiprocessing as mp

    max_concurrency = min(len(contents), mp.cpu_count() * 2)
    semaphore = anyio.Semaphore(max_concurrency)

    results = cast("list[ExtractionResult]", [None] * len(contents))

    async def _extract_bytes(content: bytes, mime_type: str, index: int) -> None:
        async with semaphore:
            try:
                result = await extract_bytes(content, mime_type, config)
                results[index] = result
            except Exception as e:  # noqa: BLE001
                from kreuzberg._utils._errors import create_error_context

                error_result = ExtractionResult(
                    content=f"Error: {type(e).__name__}: {e!s}",
                    mime_type="text/plain",
                    metadata={  # type: ignore[typeddict-unknown-key]
                        "error": True,
                        "error_context": create_error_context(
                            operation="batch_extract_bytes",
                            error=e,
                            index=index,
                            mime_type=mime_type,
                            content_size=len(content),
                        ),
                    },
                    chunks=[],
                )
                results[index] = error_result

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

    Raises:
        ValidationError: If the file path or configuration is invalid.
    """
    from kreuzberg._utils._document_cache import get_document_cache

    cache = get_document_cache()
    path = Path(file_path)
    cached_result = cache.get(path, config)
    if cached_result is not None:
        return cached_result

    if cache.is_processing(path, config):
        event = cache.mark_processing(path, config)
        event.wait()  # pragma: no cover

        # Try cache again after waiting for other process to complete  # ~keep
        cached_result = cache.get(path, config)  # pragma: no cover
        if cached_result is not None:  # pragma: no cover
            return cached_result

    cache.mark_processing(path, config)

    try:
        if not path.exists():
            raise ValidationError("The file does not exist", context={"file_path": str(path)})

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

        result = _validate_and_post_process_sync(result=result, config=config, file_path=path)

        cache.set(path, config, result)

        return result
    finally:
        cache.mark_complete(path, config)


def batch_extract_file_sync(
    file_paths: Sequence[PathLike[str] | str], config: ExtractionConfig = DEFAULT_CONFIG
) -> list[ExtractionResult]:
    """Synchronous version of batch_extract_file with parallel processing.

    Args:
        file_paths: A sequence of paths to files to extract text from.
        config: Extraction options object, defaults to the default object.

    Returns:
        A list of extraction results in the same order as the input paths.
    """
    if len(file_paths) <= 1:
        return [extract_file_sync(file_path=Path(file_path), mime_type=None, config=config) for file_path in file_paths]

    import multiprocessing as mp
    from concurrent.futures import ThreadPoolExecutor, as_completed

    max_workers = min(len(file_paths), mp.cpu_count())

    def extract_single(file_path: PathLike[str] | str) -> tuple[int, ExtractionResult]:
        """Extract single file with index for ordering."""
        try:
            return (
                file_paths.index(file_path),
                extract_file_sync(file_path=Path(file_path), mime_type=None, config=config),
            )
        except Exception as e:  # noqa: BLE001
            from kreuzberg._utils._errors import create_error_context

            error_result = ExtractionResult(
                content=f"Error: {type(e).__name__}: {e!s}",
                mime_type="text/plain",
                metadata={  # type: ignore[typeddict-unknown-key]
                    "error": True,
                    "error_context": create_error_context(
                        operation="batch_extract_file_sync",
                        file_path=file_path,
                        error=e,
                    ),
                },
                chunks=[],
            )
            return (file_paths.index(file_path), error_result)

    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        future_to_index = {executor.submit(extract_single, fp): i for i, fp in enumerate(file_paths)}

        results: list[ExtractionResult] = [None] * len(file_paths)  # type: ignore[list-item]
        for future in as_completed(future_to_index):
            index, result = future.result()
            results[index] = result

    return results


def batch_extract_bytes_sync(
    contents: Sequence[tuple[bytes, str]], config: ExtractionConfig = DEFAULT_CONFIG
) -> list[ExtractionResult]:
    """Synchronous version of batch_extract_bytes with parallel processing.

    Args:
        contents: A sequence of tuples containing (content, mime_type) pairs.
        config: Extraction options object, defaults to the default object.

    Returns:
        A list of extraction results in the same order as the input contents.
    """
    if len(contents) <= 1:
        return [
            extract_bytes_sync(content=content, mime_type=mime_type, config=config) for content, mime_type in contents
        ]

    import multiprocessing as mp
    from concurrent.futures import ThreadPoolExecutor, as_completed

    max_workers = min(len(contents), mp.cpu_count())

    def extract_single(index_and_content: tuple[int, tuple[bytes, str]]) -> tuple[int, ExtractionResult]:
        """Extract single content with index for ordering."""
        index, (content, mime_type) = index_and_content
        try:
            return (index, extract_bytes_sync(content=content, mime_type=mime_type, config=config))
        except Exception as e:  # noqa: BLE001
            from kreuzberg._utils._errors import create_error_context

            error_result = ExtractionResult(
                content=f"Error: {type(e).__name__}: {e!s}",
                mime_type="text/plain",
                metadata={  # type: ignore[typeddict-unknown-key]
                    "error": True,
                    "error_context": create_error_context(
                        operation="batch_extract_bytes_sync",
                        error=e,
                        index=index,
                        mime_type=mime_type,
                        content_size=len(content),
                    ),
                },
                chunks=[],
            )
            return (index, error_result)

    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        indexed_contents = list(enumerate(contents))
        future_to_index = {executor.submit(extract_single, ic): i for i, ic in enumerate(indexed_contents)}

        results: list[ExtractionResult] = [None] * len(contents)  # type: ignore[list-item]
        for future in as_completed(future_to_index):
            index, result = future.result()
            results[index] = result

    return results
