"""Python extraction API - thin wrappers around Rust core.

All extraction logic is in the Rust core. This module provides pure re-exports
with Python-friendly signatures.
"""

from __future__ import annotations

from kreuzberg._internal_bindings import (
    PostProcessorConfig,
    batch_extract_bytes,
    batch_extract_bytes_sync,
    batch_extract_files,
    batch_extract_files_sync,
    extract_bytes,
    extract_bytes_sync,
    extract_file,
    extract_file_sync,
)

__all__ = [
    "PostProcessorConfig",
    "batch_extract_bytes",
    "batch_extract_bytes_sync",
    "batch_extract_files",
    "batch_extract_files_sync",
    "extract_bytes",
    "extract_bytes_sync",
    "extract_file",
    "extract_file_sync",
]
