"""Custom PostProcessor support for Kreuzberg.

This module provides the protocol interface for creating custom post-processors
that can be registered with the Rust extraction pipeline.

Example:
    >>> from kreuzberg import PostProcessorProtocol, register_post_processor, ExtractionResult
    >>>
    >>> class MyCustomProcessor:
    ...     '''Custom processor that adds custom metadata.'''
    ...
    ...     def name(self) -> str:
    ...         return "my_custom_processor"
    ...
    ...     def process(self, result: ExtractionResult) -> ExtractionResult:
    ...         # Add custom processing logic
    ...         word_count = len(result.content.split())
    ...         result.metadata["custom_word_count"] = word_count
    ...         result.metadata["custom_tag"] = "processed"
    ...         return result
    ...
    ...     def processing_stage(self) -> str:
    ...         return "middle"  # or "early" or "late"
    ...
    ...     def initialize(self) -> None:
    ...         # Optional: Initialize resources (e.g., load ML models)
    ...         pass
    ...
    ...     def shutdown(self) -> None:
    ...         # Optional: Release resources
    ...         pass
    >>>
    >>> # Register the processor
    >>> processor = MyCustomProcessor()
    >>> register_post_processor(processor)
    >>>
    >>> # Now it will be called automatically during extraction
    >>> from kreuzberg import extract_file_sync
    >>> result = extract_file_sync("document.pdf")
    >>> print(result.metadata.get("custom_word_count"))
    >>> print(result.metadata.get("custom_tag"))

Processing Stages:
    - **early**: Runs first in the pipeline (e.g., language detection)
    - **middle**: Runs in the middle (default, most processors)
    - **late**: Runs last (e.g., final formatting, summaries)

Thread Safety:
    Processors are called from the Rust core which may use threading.
    Ensure your processor is thread-safe or uses appropriate locking.

Performance:
    Keep processing fast - slow processors will impact extraction performance.
    Consider lazy initialization for expensive resources (ML models, etc.).
"""

from __future__ import annotations

from kreuzberg.postprocessors.protocol import PostProcessorProtocol

__all__ = ["PostProcessorProtocol"]
