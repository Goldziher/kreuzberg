"""Simple routing logic for hybrid backend."""

from __future__ import annotations

import mimetypes
from enum import Enum
from pathlib import Path


class ExtractionStrategy(Enum):
    """Extraction strategy for backend selection."""

    BALANCED = "balanced"
    SPEED = "speed"
    QUALITY = "quality"


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
    # Simple strategy: prefer kreuzberg for most document types
    # Use extractous for complex formats where it might excel

    if strategy == ExtractionStrategy.SPEED:
        return "extractous"
    if strategy == ExtractionStrategy.QUALITY:
        return "kreuzberg"
    # BALANCED
    # Use extractous for certain formats, kreuzberg for others
    extractous_preferred = {
        "pdf",
        "docx",
        "pptx",
        "xlsx",
        "application/pdf",
        "application/vnd.openxmlformats-officedocument",
    }

    if any(ext in file_type.lower() for ext in extractous_preferred):
        return "extractous"
    return "kreuzberg"


def get_fallback_backend(primary_backend: str, _file_type: str) -> str | None:
    """Get fallback backend if primary fails."""
    if primary_backend == "kreuzberg":
        return "extractous"
    if primary_backend == "extractous":
        return "kreuzberg"
    return None
