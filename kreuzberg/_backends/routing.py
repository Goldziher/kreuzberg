"""Simple routing logic for hybrid backend."""

from __future__ import annotations

import mimetypes
from enum import Enum
from pathlib import Path


class ExtractionStrategy(Enum):
    """Extraction strategy for backend selection."""

    BALANCED = "balanced"
    SPEED = "speed"
    RICH_METADATA = "rich_metadata"


def detect_file_type(file_path: str | Path) -> str:
    """Detect file type from path."""
    if isinstance(file_path, str):
        file_path = Path(file_path)

    suffix = file_path.suffix.lower()
    if suffix:
        return suffix[1:]  # Remove the dot

    # Try to guess from MIME type
    mime_type, _ = mimetypes.guess_type(str(file_path))
    if mime_type:
        return mime_type

    return "unknown"


def select_optimal_backend(file_type: str, strategy: ExtractionStrategy, _file_path: str | Path | None = None) -> str:
    """Select optimal backend based on file type and strategy."""
    # Based on benchmark results:
    # - Kreuzberg is generally faster for most formats
    # - Extractous may provide better metadata for some formats but is slower
    # - Prefer speed unless quality is specifically requested

    if strategy == ExtractionStrategy.SPEED:
        # Always use kreuzberg for speed - it's consistently faster
        return "kreuzberg"

    if strategy == ExtractionStrategy.RICH_METADATA:
        # Use extractous only for formats where it provides significantly better metadata
        # XLSX: Always use kreuzberg (extractous has parsing issues)
        # PDF: Use extractous for richer metadata
        # Everything else: Use kreuzberg for reliability and speed
        metadata_preferred = {
            "pdf",
            "application/pdf",
        }
        if any(ext in file_type.lower() for ext in metadata_preferred):
            return "extractous"
        return "kreuzberg"

    # BALANCED strategy
    # Use extractous only for PDFs where metadata quality matters
    # Always use kreuzberg for XLSX and other Office formats due to:
    # - Better reliability (no parsing errors)
    # - Superior performance
    # - Equivalent or better extraction quality
    pdf_only_extractous = {
        "pdf",
        "application/pdf",
    }

    if any(ext in file_type.lower() for ext in pdf_only_extractous):
        return "extractous"
    return "kreuzberg"


def get_fallback_backend(primary_backend: str, _file_type: str) -> str | None:
    """Get fallback backend if primary fails."""
    if primary_backend == "kreuzberg":
        return "extractous"
    if primary_backend == "extractous":
        return "kreuzberg"
    return None
