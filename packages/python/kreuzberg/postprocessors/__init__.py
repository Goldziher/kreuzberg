"""Python postprocessors for Kreuzberg.

This module provides ML-based postprocessing capabilities that enrich extraction
results with additional metadata like entities, keywords, and document categories.

Postprocessors are automatically registered with the Rust core when this module
is imported, making them available to the extraction pipeline.

Available processors (if dependencies are installed):
- EntityExtractionProcessor: Extract named entities using spaCy NER
- KeywordExtractionProcessor: Extract keywords using KeyBERT (sentence-transformers)
- CategoryExtractionProcessor: Classify documents using zero-shot transformers

Example:
    >>> from kreuzberg import extract_file
    >>> # Postprocessors are automatically registered
    >>> result = extract_file("document.pdf")
    >>> # Result now includes entities, keywords, and category in metadata
    >>> print(result.metadata.get("entities"))
    >>> print(result.metadata.get("keywords"))
    >>> print(result.metadata.get("category"))

"""

from __future__ import annotations

import logging

# Make processors available for import
__all__ = [
    "CategoryExtractionProcessor",
    "EntityExtractionProcessor",
    "KeywordExtractionProcessor",
    "PostProcessorProtocol",
]

from .protocol import PostProcessorProtocol

logger = logging.getLogger(__name__)


def _register_processors() -> None:
    """Auto-register all available postprocessors.

    This function tries to import and register each processor. If the required
    dependencies are not installed, it silently skips that processor.

    Processors are registered with the Rust core PostProcessor registry via FFI bridge.
    """
    # Import the registration function from our bindings
    # This should always succeed - if it doesn't, that's a critical error
    from kreuzberg._internal_bindings import register_post_processor

    # Try to register Entity Extraction (requires spaCy)
    try:
        from .entity_extraction import EntityExtractionProcessor

        entity_processor = EntityExtractionProcessor()
        register_post_processor(entity_processor)
    except ImportError:
        # spaCy not installed - skip entity extraction
        pass
    except Exception:
        # Other error during registration
        logger.warning("Failed to register EntityExtractionProcessor", exc_info=True)

    # Try to register Keyword Extraction with KeyBERT (requires keybert + sentence-transformers)
    try:
        from .keyword_extraction import KeywordExtractionProcessor

        keyword_processor = KeywordExtractionProcessor(top_n=10)
        register_post_processor(keyword_processor)
    except ImportError:
        # KeyBERT not installed - skip
        pass
    except Exception:
        # Other error during registration
        logger.warning("Failed to register KeywordExtractionProcessor", exc_info=True)

    # Try to register Category Extraction (requires transformers)
    try:
        from .category_extraction import CategoryExtractionProcessor

        # Use default document type categories
        category_processor = CategoryExtractionProcessor(categories=CategoryExtractionProcessor.DOCUMENT_TYPES)
        register_post_processor(category_processor)
    except ImportError:
        # transformers not installed - skip
        pass
    except Exception:
        # Other error during registration
        logger.warning("Failed to register CategoryExtractionProcessor", exc_info=True)


# Auto-register processors when module is imported
# This makes them immediately available to the Rust extraction pipeline
_register_processors()

# Make processors available for direct import
try:
    from .entity_extraction import EntityExtractionProcessor
except ImportError:
    EntityExtractionProcessor = None  # type: ignore[assignment,misc]

try:
    from .keyword_extraction import KeywordExtractionProcessor
except ImportError:
    KeywordExtractionProcessor = None  # type: ignore[assignment,misc]

try:
    from .category_extraction import CategoryExtractionProcessor
except ImportError:
    CategoryExtractionProcessor = None  # type: ignore[assignment,misc]
