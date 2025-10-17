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
    "PostProcessorProtocol",
    "EntityExtractionProcessor",
    "KeywordExtractionProcessor",
    "CategoryExtractionProcessor",
]

from .protocol import PostProcessorProtocol

logger = logging.getLogger(__name__)


def _register_processors() -> None:
    """Auto-register all available postprocessors.

    This function tries to import and register each processor. If the required
    dependencies are not installed, it silently skips that processor.

    Processors are registered both with:
    1. The Rust core PostProcessor registry (via FFI bridge)
    2. The Python-side registry (for use in Python extraction wrapper)
    """
    # Import the registration functions
    try:
        from _internal_bindings import register_post_processor as register_rust

        from kreuzberg.extraction import register_python_postprocessor
    except ImportError:
        # Bindings not available (shouldn't happen in normal usage)
        return

    # Try to register Entity Extraction (requires spaCy)
    try:
        from .entity_extraction import EntityExtractionProcessor

        processor = EntityExtractionProcessor()
        register_rust(processor)  # Register with Rust
        register_python_postprocessor(processor)  # Register with Python
    except ImportError:
        # spaCy not installed - skip entity extraction
        pass
    except Exception:
        # Other error during registration
        logger.warning("Failed to register EntityExtractionProcessor", exc_info=True)

    # Try to register Keyword Extraction with KeyBERT (requires keybert + sentence-transformers)
    try:
        from .keyword_extraction import KeywordExtractionProcessor

        processor = KeywordExtractionProcessor(top_n=10)
        register_rust(processor)  # Register with Rust
        register_python_postprocessor(processor)  # Register with Python
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
        processor = CategoryExtractionProcessor(categories=CategoryExtractionProcessor.DOCUMENT_TYPES)
        register_rust(processor)  # Register with Rust
        register_python_postprocessor(processor)  # Register with Python
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
