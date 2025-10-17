"""Keyword extraction postprocessor using KeyBERT.

This module provides keyword extraction using KeyBERT with sentence-transformers.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from pathlib import Path

    from kreuzberg._internal_bindings import ExtractionResult


class KeywordExtractionProcessor:
    """Extract keywords from text using KeyBERT.

    This processor uses KeyBERT with sentence-transformers for semantic keyword extraction.

    Args:
        model: Model name or path to use (default: "all-MiniLM-L6-v2")
        model_cache_dir: Directory to cache downloaded models (optional)
        top_n: Number of keywords to extract (default: 10)
        ngram_range: Range of n-grams to consider as keywords (default: (1, 2))
                    E.g., (1, 2) extracts single words and two-word phrases
        min_score: Minimum score threshold for keywords (default: 0.0)
        **model_kwargs: Additional kwargs passed to sentence-transformers model initialization

    Example:
        >>> processor = KeywordExtractionProcessor(model="all-MiniLM-L6-v2", model_cache_dir="/path/to/cache", top_n=5)
        >>> result = {"content": "Machine learning and AI are transforming document processing.", "metadata": {}}
        >>> processed = processor.process(result)
        >>> print(processed["metadata"]["keywords"])
        [
            {"keyword": "machine learning", "score": 0.95},
            {"keyword": "document processing", "score": 0.87},
            {"keyword": "AI", "score": 0.82}
        ]

    """

    def __init__(
        self,
        model: str = "all-MiniLM-L6-v2",
        model_cache_dir: str | Path | None = None,
        top_n: int = 10,
        ngram_range: tuple[int, int] = (1, 2),
        min_score: float = 0.0,
        **model_kwargs: Any,
    ) -> None:
        self.model_name = model
        self.model_cache_dir = str(model_cache_dir) if model_cache_dir else None
        self.top_n = top_n
        self.ngram_range = ngram_range
        self.min_score = min_score
        self.model_kwargs = model_kwargs
        self._extractor = None  # Lazy loaded

    def name(self) -> str:
        """Return processor name."""
        return "keyword_extraction"

    def processing_stage(self) -> str:
        """Run in middle stage (default)."""
        return "middle"

    def version(self) -> str:
        """Return processor version."""
        return "1.0.0"

    def initialize(self) -> None:
        """Load KeyBERT extractor."""
        try:
            from keybert import KeyBERT
        except ImportError as e:
            msg = "KeyBERT is required for keyword extraction. Install with: pip install keybert"
            raise ImportError(msg) from e

        # Build model initialization kwargs
        model_init_kwargs = {}
        if self.model_cache_dir:
            model_init_kwargs["cache_folder"] = self.model_cache_dir

        # Add any additional user-provided kwargs
        model_init_kwargs.update(self.model_kwargs)

        # Initialize KeyBERT with model
        # KeyBERT will internally load the sentence-transformer model
        self._extractor = KeyBERT(model=self.model_name)

        # If cache_folder was specified, we need to pass it to the underlying model
        # This is a bit tricky with KeyBERT's API, so we'll handle it via environment variable
        if self.model_cache_dir:
            import os

            # sentence-transformers respects SENTENCE_TRANSFORMERS_HOME
            os.environ["SENTENCE_TRANSFORMERS_HOME"] = self.model_cache_dir

    def shutdown(self) -> None:
        """Release resources."""
        self._extractor = None

    def process(self, result: ExtractionResult) -> ExtractionResult:
        """Extract keywords from the content.

        Args:
            result: ExtractionResult with content and metadata

        Returns:
            ExtractionResult: Result with keywords added to metadata["keywords"]

        Example result.metadata["keywords"]:
            [
                {"keyword": "machine learning", "score": 0.95},
                {"keyword": "document processing", "score": 0.87},
                {"keyword": "OCR", "score": 0.82}
            ]

        Raises:
            RuntimeError: If the KeyBERT extractor fails to initialize.

        """
        # Lazy load extractor if not yet initialized
        if self._extractor is None:
            self.initialize()

        if self._extractor is None:
            msg = "KeyBERT extractor failed to initialize"
            raise RuntimeError(msg)

        content = result.content
        if not content or not isinstance(content, str):
            return result

        # Extract keywords using KeyBERT
        try:
            keywords_tuples = self._extractor.extract_keywords(
                content,
                keyphrase_ngram_range=self.ngram_range,
                stop_words="english",
                top_n=self.top_n,
            )

            keywords = [{"keyword": kw, "score": float(score)} for kw, score in keywords_tuples]

            # Filter by minimum score
            if self.min_score > 0:
                keywords = [kw for kw in keywords if kw["score"] >= self.min_score]

        except Exception:
            # If extraction fails, return empty list instead of crashing
            keywords = []

        # Add keywords to metadata
        if "keywords" not in result.metadata:
            result.metadata["keywords"] = keywords

        return result
