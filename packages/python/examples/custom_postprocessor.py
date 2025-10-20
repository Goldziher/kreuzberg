# mypy: ignore-errors
"""Example: Creating and registering a custom PostProcessor.

This example shows how to create a custom post-processor that adds
custom metadata to extraction results.
"""

from typing import Literal

from kreuzberg import (
    ExtractionResult,
    extract_file_sync,
    register_post_processor,
)


class WordCountProcessor:
    """Custom processor that adds word and sentence counts to metadata."""

    def name(self) -> str:
        """Return the unique name of this processor."""
        return "word_count_processor"

    def process(self, result: ExtractionResult) -> ExtractionResult:
        """Add word count and sentence count to metadata.

        Args:
            result: ExtractionResult with extracted content

        Returns:
            ExtractionResult: Result with added metadata
        """
        content = result.content

        words = content.split()
        word_count = len(words)

        sentence_endings = content.count(".") + content.count("!") + content.count("?")

        result.metadata["word_count"] = word_count
        result.metadata["sentence_count"] = sentence_endings
        result.metadata["avg_word_length"] = sum(len(word) for word in words) / word_count if word_count > 0 else 0.0

        return result

    def processing_stage(self) -> Literal["early", "middle", "late"]:
        """Run in the middle stage."""
        return "middle"

    def initialize(self) -> None:
        """Optional: Initialize resources."""

    def shutdown(self) -> None:
        """Optional: Release resources."""


class UpperCaseProcessor:
    """Custom processor that converts content to uppercase (demo only)."""

    def name(self) -> str:
        """Return the unique name of this processor."""
        return "uppercase_processor"

    def process(self, result: ExtractionResult) -> ExtractionResult:
        """Convert content to uppercase.

        Note: This modifies the actual content - most processors
        should only add metadata, not modify content.
        """
        result.content = result.content.upper()
        result.metadata["uppercase_applied"] = True
        return result

    def processing_stage(self) -> Literal["early", "middle", "late"]:
        """Run in late stage (after other processors)."""
        return "late"


def main() -> None:
    """Demo custom postprocessors."""
    word_counter = WordCountProcessor()
    register_post_processor(word_counter)

    result = extract_file_sync("example.txt")

    if result.metadata.get("uppercase_applied"):
        pass


if __name__ == "__main__":
    main()
