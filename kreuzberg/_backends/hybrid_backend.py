"""Hybrid backend that intelligently routes between Kreuzberg and Extractous."""

from __future__ import annotations

import logging
from typing import TYPE_CHECKING, Any, cast

from kreuzberg._backends.base import ExtractionBackend
from kreuzberg._backends.kreuzberg_backend import KreuzbergBackend
from kreuzberg._backends.routing import (
    ExtractionStrategy,
    detect_file_type,
    get_fallback_backend,
    select_optimal_backend,
)
from kreuzberg._types import Metadata
from kreuzberg.exceptions import ParsingError

if TYPE_CHECKING:
    from pathlib import Path

    from kreuzberg._types import ExtractionResult

logger = logging.getLogger(__name__)


class HybridBackend(ExtractionBackend):
    """Hybrid backend that intelligently routes between Kreuzberg and Extractous."""

    def __init__(
        self,
        strategy: ExtractionStrategy = ExtractionStrategy.BALANCED,
        enable_fallback: bool = True,
        extractous_config: dict[str, Any] | None = None,
        kreuzberg_config: Any = None,
    ) -> None:
        """Initialize the hybrid backend.

        Args:
            strategy: Strategy for backend selection
            enable_fallback: Whether to enable fallback to alternative backend
            extractous_config: Configuration for Extractous backend
            kreuzberg_config: Configuration for Kreuzberg backend
        """
        self.strategy = strategy
        self.enable_fallback = enable_fallback

        # Initialize backends
        self.kreuzberg = KreuzbergBackend(config=kreuzberg_config)

        # Initialize Extractous backend if available
        self.extractous = None
        try:
            from kreuzberg._backends.extractous_backend import ExtractousBackend

            self.extractous = ExtractousBackend(config=extractous_config)
        except ImportError:
            logger.warning("Extractous backend not available. Install with: pip install kreuzberg[extractous]")

        self._stats = {
            "kreuzberg_calls": 0,
            "extractous_calls": 0,
            "fallback_successes": 0,
            "total_extractions": 0,
        }

    def extract(self, file_path: str | Path, **kwargs: Any) -> ExtractionResult:
        """Extract text using optimal backend selection.

        Args:
            file_path: Path to the file to extract from
            **kwargs: Additional configuration options

        Returns:
            ExtractionResult object containing extracted text and metadata
        """
        self._stats["total_extractions"] += 1

        file_type = detect_file_type(file_path)
        optimal_backend_name = select_optimal_backend(file_type, self.strategy, file_path)

        # Get the optimal backend
        backend = self._get_backend(optimal_backend_name)
        if backend is None:
            # Fall back to available backend
            backend = self.kreuzberg if self.extractous is None else self.extractous
            optimal_backend_name = backend.name

        # Track usage
        if optimal_backend_name == "kreuzberg":
            self._stats["kreuzberg_calls"] += 1
        elif optimal_backend_name == "extractous":
            self._stats["extractous_calls"] += 1

        # Try primary backend
        try:
            result = backend.extract(file_path, **kwargs)

            # Enhance metadata if using RICH_METADATA strategy and different backend available
            if (
                self.strategy == ExtractionStrategy.RICH_METADATA
                and optimal_backend_name == "kreuzberg"
                and self.extractous is not None
            ):
                result = self._enhance_metadata_if_beneficial(result, file_path, file_type)

            # Add hybrid backend metadata
            if result.metadata is None:
                result.metadata = {}
            result.metadata.update(
                {
                    "hybrid_backend": True,
                    "primary_backend": optimal_backend_name,
                    "extraction_strategy": self.strategy.value,
                    "file_type": file_type,
                }
            )

            logger.debug("Successfully extracted %s using %s backend", file_path, optimal_backend_name)

            return result

        except Exception as primary_error:
            if not self.enable_fallback:
                raise

            # Try fallback backend
            fallback_backend_name = get_fallback_backend(optimal_backend_name, file_type)
            if fallback_backend_name is None:
                logger.warning("No fallback available for %s after %s failed", file_type, optimal_backend_name)
                raise

            fallback_backend = self._get_backend(fallback_backend_name)
            if fallback_backend is None:
                logger.warning("Fallback backend %s not available", fallback_backend_name)
                raise

            try:
                logger.info(
                    "Primary backend %s failed, trying fallback %s", optimal_backend_name, fallback_backend_name
                )

                result = fallback_backend.extract(file_path, **kwargs)
                self._stats["fallback_successes"] += 1

                # Add hybrid backend metadata
                if result.metadata is None:
                    result.metadata = {}
                result.metadata.update(
                    {
                        "hybrid_backend": True,
                        "primary_backend": optimal_backend_name,
                        "fallback_backend": fallback_backend_name,
                        "extraction_strategy": self.strategy.value,
                        "file_type": file_type,
                        "primary_error": str(primary_error),
                    }
                )

                logger.info("Successfully extracted %s using fallback %s backend", file_path, fallback_backend_name)

                return result

            except Exception as fallback_error:
                # Both backends failed
                logger.error(
                    "Both %s and %s backends failed for %s", optimal_backend_name, fallback_backend_name, file_path
                )

                raise ParsingError(
                    "Hybrid extraction failed with both backends",
                    context={
                        "file_path": str(file_path),
                        "file_type": file_type,
                        "primary_backend": optimal_backend_name,
                        "primary_error": str(primary_error),
                        "fallback_backend": fallback_backend_name,
                        "fallback_error": str(fallback_error),
                        "extraction_strategy": self.strategy.value,
                    },
                ) from fallback_error

    def extract_bytes(self, data: bytes, mime_type: str, **kwargs: Any) -> ExtractionResult:
        """Extract text from bytes using optimal backend selection.

        Args:
            data: Raw bytes of the document
            mime_type: MIME type of the document
            **kwargs: Additional configuration options

        Returns:
            ExtractionResult object containing extracted text and metadata
        """
        self._stats["total_extractions"] += 1

        optimal_backend_name = select_optimal_backend(mime_type, self.strategy)

        # Get the optimal backend
        backend = self._get_backend(optimal_backend_name)
        if backend is None:
            # Fall back to available backend
            backend = self.kreuzberg if self.extractous is None else self.extractous
            optimal_backend_name = backend.name

        # Track usage
        if optimal_backend_name == "kreuzberg":
            self._stats["kreuzberg_calls"] += 1
        elif optimal_backend_name == "extractous":
            self._stats["extractous_calls"] += 1

        # Try primary backend
        try:
            result = backend.extract_bytes(data, mime_type, **kwargs)

            # Add hybrid backend metadata
            if result.metadata is None:
                result.metadata = {}
            result.metadata.update(
                {
                    "hybrid_backend": True,
                    "primary_backend": optimal_backend_name,
                    "extraction_strategy": self.strategy.value,
                    "mime_type": mime_type,
                }
            )

            return result

        except Exception as primary_error:
            if not self.enable_fallback:
                raise

            # Try fallback backend
            fallback_backend_name = get_fallback_backend(optimal_backend_name, mime_type)
            if fallback_backend_name is None:
                raise

            fallback_backend = self._get_backend(fallback_backend_name)
            if fallback_backend is None:
                raise

            try:
                result = fallback_backend.extract_bytes(data, mime_type, **kwargs)
                self._stats["fallback_successes"] += 1

                # Add hybrid backend metadata
                if result.metadata is None:
                    result.metadata = {}
                result.metadata.update(
                    {
                        "hybrid_backend": True,
                        "primary_backend": optimal_backend_name,
                        "fallback_backend": fallback_backend_name,
                        "extraction_strategy": self.strategy.value,
                        "mime_type": mime_type,
                        "primary_error": str(primary_error),
                    }
                )

                return result

            except Exception as fallback_error:
                # Both backends failed
                raise ParsingError(
                    "Hybrid extraction failed with both backends",
                    context={
                        "mime_type": mime_type,
                        "primary_backend": optimal_backend_name,
                        "primary_error": str(primary_error),
                        "fallback_backend": fallback_backend_name,
                        "fallback_error": str(fallback_error),
                        "extraction_strategy": self.strategy.value,
                    },
                ) from fallback_error

    def supports_format(self, file_type: str) -> bool:
        """Check if any backend supports the given format.

        Args:
            file_type: File extension or MIME type to check

        Returns:
            True if any backend supports the format
        """
        # Check if either backend supports the format
        if self.kreuzberg.supports_format(file_type):
            return True

        return bool(self.extractous and self.extractous.supports_format(file_type))

    def get_capabilities(self) -> dict[str, Any]:
        """Get combined capabilities of both backends.

        Returns:
            Dictionary containing capability information
        """
        available_backends = ["kreuzberg"]
        if self.extractous:
            available_backends.append("extractous")

        capabilities = {
            "name": "hybrid",
            "extraction_strategy": self.strategy.value,
            "enable_fallback": self.enable_fallback,
            "available_backends": available_backends,
            "stats": self._stats.copy(),
        }

        # Combine supported formats
        supported_formats = set()
        supported_formats.update(self.kreuzberg.get_capabilities()["supported_formats"])

        if self.extractous:
            supported_formats.update(self.extractous.get_capabilities()["supported_formats"])

        capabilities["supported_formats"] = list(supported_formats)
        capabilities["supports_ocr"] = True
        capabilities["supports_async"] = True
        capabilities["supports_batch"] = True

        return capabilities

    def get_stats(self) -> dict[str, Any]:
        """Get usage statistics for the hybrid backend.

        Returns:
            Dictionary containing usage statistics
        """
        total = self._stats["total_extractions"]
        if total == 0:
            return self._stats.copy()

        stats = cast("dict[str, int | float]", self._stats.copy())
        stats.update(
            {
                "kreuzberg_percentage": (self._stats["kreuzberg_calls"] / total) * 100,
                "extractous_percentage": (self._stats["extractous_calls"] / total) * 100,
                "fallback_rate": (self._stats["fallback_successes"] / total) * 100,
            }
        )

        return stats

    def _get_backend(self, backend_name: str) -> ExtractionBackend | None:
        """Get backend instance by name.

        Args:
            backend_name: Name of the backend

        Returns:
            Backend instance or None if not available
        """
        if backend_name == "kreuzberg":
            return self.kreuzberg
        if backend_name == "extractous":
            return self.extractous
        return None

    def _enhance_metadata_if_beneficial(
        self, result: ExtractionResult, file_path: str | Path, file_type: str
    ) -> ExtractionResult:
        """Enhance metadata using alternative backend if beneficial and safe.

        Args:
            result: Original extraction result
            file_path: Path to the file
            file_type: Detected file type

        Returns:
            Enhanced extraction result with additional metadata
        """
        # Only enhance metadata for specific formats where extractous provides value
        # and is known to be stable (avoid XLSX due to parsing issues)
        metadata_enhancement_formats = {
            "xlsx",
            "xls",  # Office spreadsheets - but only if extractous is stable
            "pptx",
            "ppt",  # Presentations
            "docx",
            "doc",  # Documents
        }

        # Skip enhancement for formats with known issues
        skip_formats = {
            "xlsx",
            "xls",  # Skip XLSX due to extractous parsing errors
        }

        if not any(ext in file_type.lower() for ext in metadata_enhancement_formats):
            return result

        if any(ext in file_type.lower() for ext in skip_formats):
            logger.debug("Skipping metadata enhancement for %s due to known compatibility issues", file_type)
            return result

        try:
            # Try to get additional metadata from extractous
            logger.debug("Attempting metadata enhancement for %s using extractous", file_path)
            if self.extractous is None:  # Type guard
                return result
            extractous_result = self.extractous.extract(file_path)

            # Merge metadata (prefer original result's text, enhance with extractous metadata)
            if extractous_result.metadata and result.metadata:
                # Add extractous metadata with prefix to distinguish source
                for key, value in extractous_result.metadata.items():
                    if (
                        key not in result.metadata and key in Metadata.__annotations__
                    ):  # Don't overwrite existing metadata
                        # Only add if it's a valid Metadata field
                        result.metadata[key] = str(value) if not isinstance(value, str) else value  # type: ignore[literal-required]
                logger.debug("Successfully enhanced metadata for %s", file_path)

        except Exception as e:  # noqa: BLE001
            # Metadata enhancement failure should not break the main extraction
            logger.warning("Metadata enhancement failed for %s: %s", file_path, e)
            if result.metadata is None:
                result.metadata = {}
            # Store enhancement failure in hybrid_backend metadata

        return result

    @property
    def name(self) -> str:
        """Get the name of this backend."""
        return "hybrid"
