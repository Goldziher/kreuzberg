"""Entity extraction postprocessor using spaCy NER.

This module provides Named Entity Recognition (NER) using spaCy models.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from pathlib import Path

    from kreuzberg._internal_bindings import ExtractionResult


class EntityExtractionProcessor:
    """Extract named entities from text using spaCy.

    This processor uses spaCy's NER models to identify and extract entities like
    person names, organizations, locations, dates, etc.

    Args:
        model: spaCy model to use (default: "en_core_web_sm" for English)
        model_path: Path to load model from (optional, overrides model name)
        entity_types: List of entity types to extract. If None, extracts all types.
                     Common types: PERSON, ORG, GPE, LOC, DATE, MONEY, PERCENT, PRODUCT
        max_entities: Maximum number of entities per type (default: 50)
        min_confidence: Minimum confidence score for entity (default: 0.0, no filtering)
        **model_kwargs: Additional kwargs passed to spacy.load()

    Example:
        >>> processor = EntityExtractionProcessor(
        ...     model="en_core_web_sm",
        ...     model_path="/path/to/custom/model"  # Optional
        ... )
        >>> result = {"content": "John Doe works at Microsoft in Seattle.", "metadata": {}}
        >>> processed = processor.process(result)
        >>> print(processed["metadata"]["entities"])
        {"PERSON": ["John Doe"], "ORG": ["Microsoft"], "GPE": ["Seattle"]}
    """

    def __init__(
        self,
        model: str = "en_core_web_sm",
        model_path: str | Path | None = None,
        entity_types: list[str] | None = None,
        max_entities: int = 50,
        min_confidence: float = 0.0,
        **model_kwargs: Any,
    ):
        self.model_name = model
        self.model_path = str(model_path) if model_path else None
        self.entity_types = entity_types
        self.max_entities = max_entities
        self.min_confidence = min_confidence
        self.model_kwargs = model_kwargs
        self._nlp = None  # Lazy loaded

    def name(self) -> str:
        """Return processor name."""
        return "entity_extraction"

    def processing_stage(self) -> str:
        """Run in early stage (before other processors)."""
        return "early"

    def version(self) -> str:
        """Return processor version."""
        return "1.0.0"

    def initialize(self) -> None:
        """Load spaCy model."""
        try:
            import spacy
        except ImportError as e:
            msg = (
                "spaCy is required for entity extraction. "
                "Install with: pip install spacy && python -m spacy download en_core_web_sm"
            )
            raise ImportError(msg) from e

        try:
            # Load from custom path if provided, otherwise use model name
            model_to_load = self.model_path if self.model_path else self.model_name

            # Pass through any additional kwargs to spacy.load()
            self._nlp = spacy.load(model_to_load, **self.model_kwargs)  # type: ignore[assignment]
        except OSError as e:
            if self.model_path:
                msg = f"Failed to load spaCy model from path '{self.model_path}': {e}"
            else:
                msg = (
                    f"Failed to load spaCy model '{self.model_name}'. "
                    f"Download with: python -m spacy download {self.model_name}"
                )
            raise OSError(msg) from e

    def shutdown(self) -> None:
        """Release resources."""
        if self._nlp is not None:
            # spaCy models don't require explicit cleanup
            self._nlp = None

    def process(self, result: ExtractionResult) -> ExtractionResult:
        """Extract entities from the content.

        Args:
            result: ExtractionResult with content and metadata

        Returns:
            ExtractionResult: Result with entities added to metadata["entities"]

        Example result.metadata["entities"]:
            {
                "PERSON": ["John Doe", "Jane Smith"],
                "ORG": ["Microsoft", "Google"],
                "GPE": ["Seattle", "San Francisco"],
                "DATE": ["2025-10-16", "January 2025"],
                ...
            }

        Raises:
            RuntimeError: If the spaCy model fails to initialize.
        """
        # Lazy load model if not yet initialized
        if self._nlp is None:
            self.initialize()

        if self._nlp is None:
            raise RuntimeError("spaCy model failed to initialize")

        content = result.content
        if not content or not isinstance(content, str):
            return result

        # Process text with spaCy
        doc = self._nlp(content)

        # Extract entities and group by type
        entities_by_type: dict[str, list[str]] = {}

        for ent in doc.ents:
            # Filter by entity type if specified
            if self.entity_types and ent.label_ not in self.entity_types:
                continue

            # Filter by confidence if available (spaCy 3.x+)
            if hasattr(ent, "_.score") and ent._.score < self.min_confidence:
                continue

            # Add entity to the appropriate type
            entity_type = ent.label_
            entity_text = ent.text

            if entity_type not in entities_by_type:
                entities_by_type[entity_type] = []

            # Avoid duplicates and respect max_entities limit
            if (
                entity_text not in entities_by_type[entity_type]
                and len(entities_by_type[entity_type]) < self.max_entities
            ):
                entities_by_type[entity_type].append(entity_text)

        # Add entities to metadata (don't overwrite if key exists)
        if "entities" not in result.metadata:
            result.metadata["entities"] = entities_by_type

        # Add entity count for convenience
        total_entities = sum(len(ents) for ents in entities_by_type.values())
        if "entity_count" not in result.metadata:
            result.metadata["entity_count"] = total_entities

        return result
