"""Category extraction postprocessor using zero-shot classification.

This module provides document classification using transformer-based zero-shot learning.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from pathlib import Path

    from kreuzberg._internal_bindings import ExtractionResult


class CategoryExtractionProcessor:
    """Classify documents into categories using zero-shot classification.

    This processor uses transformer models to classify documents into predefined
    categories without requiring training data. It's particularly useful for
    document type detection, subject area classification, and sentiment analysis.

    Args:
        categories: List of category labels to classify into
        model: Hugging Face model to use (default: "facebook/bart-large-mnli")
        model_cache_dir: Directory to cache downloaded models (optional)
        multi_label: If True, allow multiple categories per document (default: False)
        confidence_threshold: Minimum confidence to assign a category (default: 0.5)
        max_categories: Maximum number of categories to assign (multi_label only)
        device: Device to run model on (-1 for CPU, 0+ for GPU, default: -1)
        **pipeline_kwargs: Additional kwargs passed to transformers.pipeline()

    Raises:
        ValueError: If no categories are provided during initialization.

    Attributes:
        DOCUMENT_TYPES: Default category set used when no custom categories are supplied.
        SUBJECT_AREAS: Example category set for subject area classification.
        SENTIMENT: Example category set for sentiment analysis.

    Example:
        >>> categories = ["invoice", "contract", "resume", "report"]
        >>> processor = CategoryExtractionProcessor(
        ...     categories=categories,
        ...     model_cache_dir="/path/to/cache"
        ... )
        >>> result = {"content": "Invoice #12345 for services rendered...", "metadata": {}}
        >>> processed = processor.process(result)
        >>> print(processed["metadata"]["category"])
        {
            "primary": "invoice",
            "scores": {
                "invoice": 0.95,
                "contract": 0.12,
                "resume": 0.08,
                "report": 0.05
            },
            "confidence": 0.95
        }
    """

    # Predefined category sets for common use cases
    DOCUMENT_TYPES = [
        "invoice",
        "contract",
        "resume",
        "report",
        "email",
        "letter",
        "memo",
        "presentation",
        "spreadsheet",
        "form",
    ]

    SUBJECT_AREAS = [
        "legal",
        "financial",
        "technical",
        "medical",
        "marketing",
        "hr",
        "sales",
        "support",
        "research",
        "operations",
    ]

    SENTIMENT = ["positive", "negative", "neutral"]

    def __init__(
        self,
        categories: list[str] | None = None,
        model: str = "facebook/bart-large-mnli",
        model_cache_dir: str | Path | None = None,
        multi_label: bool = False,
        confidence_threshold: float = 0.5,
        max_categories: int = 3,
        device: int = -1,
        **pipeline_kwargs: Any,
    ):
        if categories is None:
            categories = self.DOCUMENT_TYPES  # Default to document types

        if not categories:
            msg = "At least one category must be provided"
            raise ValueError(msg)

        self.categories = categories
        self.model_name = model
        self.model_cache_dir = str(model_cache_dir) if model_cache_dir else None
        self.multi_label = multi_label
        self.confidence_threshold = confidence_threshold
        self.max_categories = max_categories
        self.device = device
        self.pipeline_kwargs = pipeline_kwargs
        self._classifier = None  # Lazy loaded

    def name(self) -> str:
        """Return processor name."""
        return "category_extraction"

    def processing_stage(self) -> str:
        """Run in middle stage (default)."""
        return "middle"

    def version(self) -> str:
        """Return processor version."""
        return "1.0.0"

    def initialize(self) -> None:
        """Load zero-shot classification model."""
        try:
            from transformers import pipeline
        except ImportError as e:
            msg = "Transformers is required for category extraction. Install with: pip install transformers torch"
            raise ImportError(msg) from e

        try:
            # Build pipeline kwargs
            init_kwargs: dict[str, Any] = {
                "model": self.model_name,
                "device": self.device,
            }

            # Add cache directory if specified
            # transformers uses TRANSFORMERS_CACHE environment variable
            # or model_kwargs={"cache_dir": ...}
            if self.model_cache_dir:
                import os

                os.environ["TRANSFORMERS_CACHE"] = self.model_cache_dir

            # Add any additional user-provided kwargs
            init_kwargs.update(self.pipeline_kwargs)

            # Initialize the pipeline
            self._classifier = pipeline(  # type: ignore[assignment]
                "zero-shot-classification",
                **init_kwargs,
            )
        except Exception as e:
            msg = f"Failed to load model '{self.model_name}': {e}"
            raise RuntimeError(msg) from e

    def shutdown(self) -> None:
        """Release resources."""
        self._classifier = None

    def process(self, result: ExtractionResult) -> ExtractionResult:
        """Classify the content into categories.

        Args:
            result: ExtractionResult with content and metadata

        Returns:
            ExtractionResult: Result with category added to metadata["category"]

        Example result.metadata["category"] (single-label):
            {
                "primary": "invoice",
                "scores": {
                    "invoice": 0.95,
                    "contract": 0.42,
                    "resume": 0.18,
                    "report": 0.12
                },
                "confidence": 0.95
            }

        Example result.metadata["category"] (multi-label):
            {
                "primary": "financial",
                "labels": ["financial", "legal"],
                "scores": {
                    "financial": 0.92,
                    "legal": 0.78,
                    "technical": 0.23
                },
                "confidence": 0.92
            }

        Raises:
            RuntimeError: If the zero-shot classifier fails to initialize.
        """
        # Lazy load classifier if not yet initialized
        if self._classifier is None:
            self.initialize()

        if self._classifier is None:
            raise RuntimeError("Classifier failed to initialize")

        content = result.content
        if not content or not isinstance(content, str):
            return result

        # Truncate very long documents to avoid memory issues
        # Most models have a 512 token limit, roughly 2000 characters
        max_length = 2000
        if len(content) > max_length:
            # Take first part and last part to preserve context
            half_length = max_length // 2
            content = content[:half_length] + "\n...\n" + content[-half_length:]

        # Perform zero-shot classification
        try:
            classification: dict[str, Any] = self._classifier(
                content,
                self.categories,
                multi_label=self.multi_label,
            )

            # Build category result
            category_result: dict[str, Any] = {
                "scores": {},
            }

            # Add scores for all categories
            for label, score in zip(classification["labels"], classification["scores"], strict=False):
                category_result["scores"][label] = float(score)

            # Determine primary category (highest score)
            primary_label: str = classification["labels"][0]
            primary_score: float = classification["scores"][0]
            category_result["primary"] = primary_label
            category_result["confidence"] = float(primary_score)

            # For multi-label, add all categories above threshold
            if self.multi_label:
                labels = [
                    label
                    for label, score in zip(classification["labels"], classification["scores"], strict=False)
                    if score >= self.confidence_threshold
                ]
                # Limit to max_categories
                category_result["labels"] = labels[: self.max_categories]

        except Exception:
            # If classification fails, return minimal result
            category_result = {
                "primary": None,
                "scores": {},
                "confidence": 0.0,
            }

        # Add category to metadata
        if "category" not in result.metadata:
            result.metadata["category"] = category_result

        return result
