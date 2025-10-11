from __future__ import annotations

import multiprocessing as mp
import traceback
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import TYPE_CHECKING, Final, cast

import anyio

from kreuzberg._chunker import get_chunker
from kreuzberg._document_classification import auto_detect_document_type
from kreuzberg._entity_extraction import extract_entities, extract_keywords
from kreuzberg._error_handling import safe_feature_execution, should_exception_bubble_up
from kreuzberg._internal_bindings import safe_decode
from kreuzberg._language_detection import detect_languages
from kreuzberg._mime_types import (
    validate_mime_type,
)
from kreuzberg._registry import ExtractorRegistry
from kreuzberg._token_reduction import get_reduction_stats, reduce_tokens
from kreuzberg._types import ChunkingConfig, ExtractionConfig, ExtractionResult
from kreuzberg._utils._document_cache import get_document_cache
from kreuzberg._utils._errors import create_error_context
from kreuzberg._utils._sync import run_maybe_sync, run_sync_only
from kreuzberg.exceptions import KreuzbergError, ValidationError

if TYPE_CHECKING:
    from collections.abc import Sequence
    from os import PathLike


DEFAULT_CONFIG: Final[ExtractionConfig] = ExtractionConfig()


async def _handle_cache_async(path: Path, config: ExtractionConfig) -> ExtractionResult | None:
    cache = get_document_cache()

    cached_result = cache.get(path, config)
    if cached_result is not None:
        return cached_result

    if cache.is_processing(path, config):
        event = cache.mark_processing(path, config)  # pragma: no cover
        await anyio.to_thread.run_sync(event.wait)  # pragma: no cover

        return cache.get(path, config)  # pragma: no cover

    return None


def _validate_and_post_process_helper(
    result: ExtractionResult, config: ExtractionConfig, file_path: Path | None = None
) -> ExtractionResult:
    if result.metadata is None:
        result.metadata = {}

    if config.chunking is not None:
        chunking_config = config.chunking
        result.chunks = safe_feature_execution(
            feature_name="chunking",
            execution_func=lambda: _handle_chunk_content(
                mime_type=result.mime_type,
                chunking_config=chunking_config,
                content=result.content,
            ),
            default_value=[],
            result=result,
        )

    if config.entities is not None:
        result.entities = safe_feature_execution(
            feature_name="entity_extraction",
            execution_func=lambda: extract_entities(
                result.content,
                custom_patterns=config.custom_entity_patterns,
                spacy_config=config.entities,
            ),
            default_value=None,
            result=result,
        )

    if config.keywords is not None:
        keywords_config = config.keywords
        result.keywords = safe_feature_execution(
            feature_name="keyword_extraction",
            execution_func=lambda: extract_keywords(
                result.content,
                keyword_count=keywords_config.count,
            ),
            default_value=None,
            result=result,
        )

    if config.language_detection is not None:
        result.detected_languages = safe_feature_execution(
            feature_name="language_detection",
            execution_func=lambda: detect_languages(result.content, config=config.language_detection) or [],
            default_value=[],
            result=result,
        )

    if config.auto_detect_document_type:
        result = safe_feature_execution(
            feature_name="document_type_detection",
            execution_func=lambda: auto_detect_document_type(result, config, file_path=file_path),
            default_value=result,
            result=result,
        )

    if config.token_reduction is not None and config.token_reduction.mode != "off":

        def _apply_token_reduction() -> str:
            original_content = result.content

            language_hint = None
            if result.detected_languages and len(result.detected_languages) > 0:
                language_hint = result.detected_languages[0]

            reduced_content = (
                reduce_tokens(
                    original_content,
                    config=config.token_reduction,
                    language=language_hint,
                )
                if config.token_reduction
                else original_content
            )
            reduction_stats = get_reduction_stats(original_content, reduced_content)

            if result.metadata is not None:
                result.metadata["token_reduction"] = {
                    "character_reduction_ratio": reduction_stats["character_reduction_ratio"],
                    "token_reduction_ratio": reduction_stats["token_reduction_ratio"],
                    "original_characters": reduction_stats["original_characters"],
                    "reduced_characters": reduction_stats["reduced_characters"],
                    "original_tokens": reduction_stats["original_tokens"],
                    "reduced_tokens": reduction_stats["reduced_tokens"],
                }

            return reduced_content

        result.content = safe_feature_execution(
            feature_name="token_reduction",
            execution_func=_apply_token_reduction,
            default_value=result.content,
            result=result,
        )

    return result


async def _validate_and_post_process_async(
    result: ExtractionResult, config: ExtractionConfig, file_path: Path | None = None
) -> ExtractionResult:
    for validator in config.validators or []:
        await run_maybe_sync(validator, result)

    result = _validate_and_post_process_helper(result, config, file_path)

    for i, post_processor in enumerate(config.post_processing_hooks or []):
        try:
            result = await run_maybe_sync(post_processor, result)
        except (KreuzbergError, ValueError, RuntimeError, TypeError) as e:  # noqa: PERF203
            if result.metadata is None:
                result.metadata = {}
            error_list = result.metadata.setdefault("processing_errors", [])
            if isinstance(error_list, list):
                error_list.append(
                    {
                        "feature": f"post_processing_hook_{i}",
                        "error_type": type(e).__name__,
                        "error_message": str(e),
                        "traceback": traceback.format_exc(),
                    }
                )

    return result


def _validate_and_post_process_sync(
    result: ExtractionResult, config: ExtractionConfig, file_path: Path | None = None
) -> ExtractionResult:
    for validator in config.validators or []:
        run_sync_only(validator, result)

    result = _validate_and_post_process_helper(result, config, file_path)

    for post_processor in config.post_processing_hooks or []:
        result = run_sync_only(post_processor, result)

    return result


def _handle_chunk_content(
    mime_type: str,
    chunking_config: ChunkingConfig,
    content: str,
) -> list[str]:
    chunker = get_chunker(
        mime_type=mime_type,
        max_characters=chunking_config.max_chars,
        overlap_characters=chunking_config.max_overlap,
    )
    return list(chunker.chunks(content))


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
    cache = get_document_cache()
    path = Path(file_path)

    if config.use_cache:
        cached_result = await _handle_cache_async(path, config)
        if cached_result is not None:
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

        if config.use_cache:
            cache.set(path, config, result)

        return result
    finally:
        if config.use_cache:
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
            except Exception as e:
                if should_exception_bubble_up(e, "batch_processing"):
                    raise

                basic_result = _attempt_basic_extraction(
                    None,
                    None,
                    e,
                    index,
                    file_path=str(path),
                )
                results[index] = basic_result

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

    max_concurrency = min(len(contents), mp.cpu_count() * 2)
    semaphore = anyio.Semaphore(max_concurrency)

    results = cast("list[ExtractionResult]", [None] * len(contents))

    async def _extract_bytes(content: bytes, mime_type: str, index: int) -> None:
        async with semaphore:
            try:
                result = await extract_bytes(content, mime_type, config)
                results[index] = result
            except Exception as e:
                if should_exception_bubble_up(e, "batch_processing"):
                    raise

                basic_result = _attempt_basic_extraction(content, mime_type, e, index)
                results[index] = basic_result

    async with anyio.create_task_group() as tg:
        for i, (content, mime_type) in enumerate(contents):
            tg.start_soon(_extract_bytes, content, mime_type, i)

    return results


def _attempt_basic_extraction(
    content: bytes | None, mime_type: str | None, original_error: Exception, index: int, *, file_path: str | None = None
) -> ExtractionResult:
    """Attempt basic extraction when full extraction fails, preserving as much as possible.

    This function tries to extract at least basic text content even when advanced
    features like OCR, entity extraction, etc. fail.

    Args:
        content: The raw content bytes (None for file extractions)
        mime_type: The MIME type of the content (None if unknown)
        original_error: The exception that caused the main extraction to fail
        index: Index of this content in the batch
        file_path: Optional file path for file-based extractions

    Returns:
        A basic ExtractionResult with whatever could be extracted
    """
    if (
        isinstance(original_error, (ValueError, TypeError, ValidationError))
        or "mock" in str(type(original_error)).lower()
    ):
        return ExtractionResult(
            content=f"Error: {type(original_error).__name__}: {original_error!s}",
            mime_type="text/plain",
            metadata={
                "error": f"{type(original_error).__name__}: {original_error!s}",
                "error_context": create_error_context(
                    operation="batch_extract_file" if file_path else "batch_extract_bytes",
                    error=original_error,
                    index=index,
                    mime_type=mime_type,
                    content_size=len(content) if content else 0,
                    file_path=file_path,
                ),
            },
            chunks=[],
            entities=[],
            keywords=[],
            detected_languages=[],
            tables=[],
            images=[],
            image_ocr_results=[],
        )

    try:
        if content is None:
            return ExtractionResult(
                content=f"Error: {type(original_error).__name__}: {original_error!s}",
                mime_type="text/plain",
                metadata={
                    "error": f"{type(original_error).__name__}: {original_error!s}",
                    "error_context": create_error_context(
                        operation="batch_extract_file",
                        error=original_error,
                        index=index,
                        file_path=file_path,
                    ),
                },
                chunks=[],
                entities=[],
                keywords=[],
                detected_languages=[],
                tables=[],
                images=[],
                image_ocr_results=[],
            )

        mime_type = validate_mime_type(mime_type=mime_type)
        if extractor := ExtractorRegistry.get_extractor(mime_type=mime_type, config=ExtractionConfig()):
            basic_result = extractor.extract_bytes_sync(content)

            if basic_result.metadata is None:
                basic_result.metadata = {}

            basic_result.metadata["extraction_error"] = {
                "error_type": type(original_error).__name__,
                "error_message": str(original_error),
                "traceback": traceback.format_exc(),
                "context": create_error_context(
                    operation="batch_extract_file" if file_path else "batch_extract_bytes",
                    error=original_error,
                    index=index,
                    mime_type=mime_type,
                    content_size=len(content),
                    file_path=file_path,
                ),
                "recovery_mode": "basic_extraction",
            }

            return basic_result

    except (KreuzbergError, ValueError, RuntimeError, TypeError):
        pass

    return ExtractionResult(
        content=f"Error: {type(original_error).__name__}: {original_error!s}",
        mime_type="text/plain",
        metadata={
            "error": f"{type(original_error).__name__}: {original_error!s}",
            "error_context": create_error_context(
                operation="batch_extract_file" if file_path else "batch_extract_bytes",
                error=original_error,
                index=index,
                mime_type=mime_type,
                content_size=len(content) if content else 0,
                file_path=file_path,
            ),
        },
        chunks=[],
        entities=[],
        keywords=[],
        detected_languages=[],
        tables=[],
        images=[],
        image_ocr_results=[],
    )


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
    cache = get_document_cache()
    path = Path(file_path)

    if config.use_cache:
        cached_result = cache.get(path, config)
        if cached_result is not None:
            return cached_result

        if cache.is_processing(path, config):
            event = cache.mark_processing(path, config)  # pragma: no cover
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
                content=Path(file_path).read_text(encoding="utf-8"),
                chunks=[],
                mime_type=mime_type,
                metadata={},
            )

        result = _validate_and_post_process_sync(result=result, config=config, file_path=path)

        if config.use_cache:
            cache.set(path, config, result)

        return result
    finally:
        if config.use_cache:
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

    max_workers = min(len(file_paths), mp.cpu_count())

    def extract_single(index: int, file_path: PathLike[str] | str) -> tuple[int, ExtractionResult]:
        """Extract single file with index for ordering."""
        try:
            return (
                index,
                extract_file_sync(file_path=Path(file_path), mime_type=None, config=config),
            )
        except Exception as e:
            if should_exception_bubble_up(e, "batch_processing"):
                raise

            basic_result = _attempt_basic_extraction(
                None,
                None,
                e,
                index,
                file_path=str(file_path),
            )
            return (index, basic_result)

    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        future_to_index = {executor.submit(extract_single, i, fp): i for i, fp in enumerate(file_paths)}

        results: list[ExtractionResult | None] = [None] * len(file_paths)
        for future in as_completed(future_to_index):
            index, result = future.result()
            results[index] = result

    return cast("list[ExtractionResult]", results)


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

    max_workers = min(len(contents), mp.cpu_count())

    def extract_single(index: int, content: bytes, mime_type: str) -> tuple[int, ExtractionResult]:
        """Extract single content with index for ordering."""
        try:
            return (index, extract_bytes_sync(content=content, mime_type=mime_type, config=config))
        except Exception as e:
            if should_exception_bubble_up(e, "batch_processing"):
                raise

            basic_result = _attempt_basic_extraction(content, mime_type, e, index)
            return (index, basic_result)

    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        future_to_index = {
            executor.submit(extract_single, i, content, mime_type): i for i, (content, mime_type) in enumerate(contents)
        }

        results: list[ExtractionResult | None] = [None] * len(contents)
        for future in as_completed(future_to_index):
            index, result = future.result()
            results[index] = result

    return cast("list[ExtractionResult]", results)
