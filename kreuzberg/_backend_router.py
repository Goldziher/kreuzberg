"""Backend routing logic that respects ExtractionConfig.extraction_backend."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from pathlib import Path

    from kreuzberg._types import ExtractionConfig, ExtractionResult


def extract_file_with_backend(
    file_path: str | Path,
    *,
    config: ExtractionConfig | None = None,
    **kwargs: Any,
) -> ExtractionResult:
    """Extract file using the backend specified in config.

    Args:
        file_path: Path to the file to extract from
        config: Extraction configuration with backend selection
        **kwargs: Additional arguments

    Returns:
        ExtractionResult from the selected backend
    """
    if config is None:
        from kreuzberg._types import ExtractionConfig

        config = ExtractionConfig()

    backend = config.extraction_backend

    if backend == "kreuzberg":
        # Force vanilla Kreuzberg
        from kreuzberg.extraction import extract_file_sync

        return extract_file_sync(file_path, config=config, **kwargs)

    if backend == "extractous":
        # Force Extractous only
        try:
            import extractous

            extractor = extractous.Extractor()
            extractor.set_extract_string_max_length(10_000_000)

            result = extractor.extract_file_to_string(str(file_path))
            expected_tuple_length = 2
            if isinstance(result, tuple) and len(result) >= expected_tuple_length:
                text, metadata = result
            else:
                text = str(result)
                metadata = {}

            # Create Kreuzberg-compatible result
            from kreuzberg._types import ExtractionResult, normalize_metadata

            return ExtractionResult(
                content=text,
                metadata=normalize_metadata(metadata),
                mime_type="text/plain",  # Extractous doesn't provide MIME type
                chunks=[],
                tables=[],
            )
        except ImportError as e:
            raise ImportError(
                "Extractous backend requested but not installed. Install with: pip install extractous"
            ) from e

    elif backend == "hybrid":
        # Use hybrid routing
        from kreuzberg._backends.hybrid_backend import HybridBackend

        hybrid = HybridBackend()
        return hybrid.extract(file_path, **kwargs)

    else:  # backend == "auto" or fallback
        # Auto selection - use hybrid if Extractous available, otherwise vanilla
        try:
            import extractous

            from kreuzberg._backends.hybrid_backend import HybridBackend

            hybrid = HybridBackend()
            return hybrid.extract(file_path, **kwargs)
        except ImportError:
            # Fallback to vanilla
            from kreuzberg.extraction import extract_file_sync

            return extract_file_sync(file_path, config=config, **kwargs)


def extract_bytes_with_backend(
    data: bytes,
    *,
    mime_type: str,
    config: ExtractionConfig | None = None,
    **kwargs: Any,
) -> ExtractionResult:
    """Extract bytes using the backend specified in config.

    Args:
        data: Raw bytes of the document
        mime_type: MIME type of the document
        config: Extraction configuration with backend selection
        **kwargs: Additional arguments

    Returns:
        ExtractionResult from the selected backend
    """
    if config is None:
        from kreuzberg._types import ExtractionConfig

        config = ExtractionConfig()

    backend = config.extraction_backend

    if backend == "kreuzberg":
        # Force vanilla Kreuzberg
        from kreuzberg.extraction import extract_bytes_sync

        return extract_bytes_sync(data, mime_type=mime_type, config=config, **kwargs)

    if backend == "extractous":
        # Force Extractous only
        try:
            import extractous

            extractor = extractous.Extractor()
            extractor.set_extract_string_max_length(10_000_000)

            result = extractor.extract_bytes_to_string(data)
            expected_tuple_length = 2
            if isinstance(result, tuple) and len(result) >= expected_tuple_length:
                text, metadata = result
            else:
                text = str(result)
                metadata = {}

            # Create Kreuzberg-compatible result
            from kreuzberg._types import ExtractionResult, normalize_metadata

            return ExtractionResult(
                content=text,
                metadata=normalize_metadata(metadata),
                mime_type=mime_type,
                chunks=[],
                tables=[],
            )
        except ImportError as e:
            raise ImportError(
                "Extractous backend requested but not installed. Install with: pip install extractous"
            ) from e

    elif backend == "hybrid":
        # Use hybrid routing
        from kreuzberg._backends.hybrid_backend import HybridBackend

        hybrid = HybridBackend()
        return hybrid.extract_bytes(data, mime_type=mime_type, **kwargs)

    else:  # backend == "auto" or fallback
        # Auto selection - use hybrid if Extractous available, otherwise vanilla
        try:
            import extractous

            from kreuzberg._backends.hybrid_backend import HybridBackend

            hybrid = HybridBackend()
            return hybrid.extract_bytes(data, mime_type=mime_type, **kwargs)
        except ImportError:
            # Fallback to vanilla
            from kreuzberg.extraction import extract_bytes_sync

            return extract_bytes_sync(data, mime_type=mime_type, config=config, **kwargs)
