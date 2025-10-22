"""PostProcessor protocol interface.

This module defines the protocol that all Python postprocessors must implement
to be registered with the Rust core via the FFI bridge.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Literal, Protocol

if TYPE_CHECKING:
    from kreuzberg._internal_bindings import ExtractionResult


class PostProcessorProtocol(Protocol):
    """Protocol for Python postprocessors.

    All postprocessors must implement these methods to be compatible
    with the Rust PostProcessor FFI bridge.
    """

    def name(self) -> str:
        """Return the unique name of this postprocessor.

        Returns:
            str: Processor name (e.g., "entity_extraction", "keyword_extraction")

        """
        ...

    def process(self, result: ExtractionResult) -> ExtractionResult:
        """Process and enrich an extraction result.

        Args:
            result: ExtractionResult with extracted content, metadata, and tables

        Returns:
            ExtractionResult: Modified result with enriched metadata.
                  New metadata keys are added, existing keys are preserved.

        Note:
            The processor should add its results to result.metadata and
            return the modified ExtractionResult. Existing metadata keys will not be
            overwritten by the FFI bridge.

        Example:
            >>> def process(self, result: ExtractionResult) -> ExtractionResult:
            ...     text = result.content
            ...     entities = extract_entities(text)
            ...     result.metadata["entities"] = entities
            ...     return result

        """
        ...

    def processing_stage(self) -> Literal["early", "middle", "late"]:
        """Return the processing stage for this processor.

        Returns:
            str: One of "early", "middle", or "late" (default: "middle")

        Note:
            Processing stages control the order in which processors are called:
            - Early: Runs first (e.g., language detection)
            - Middle: Runs in the middle (default, e.g., entity extraction)
            - Late: Runs last (e.g., final formatting)

        """
        ...

    def initialize(self) -> None:
        """Initialize the processor (e.g., load ML models).

        Called once when the processor is registered.
        """
        ...

    def shutdown(self) -> None:
        """Shutdown the processor and release resources.

        Called when the processor is unregistered.
        """
        ...
