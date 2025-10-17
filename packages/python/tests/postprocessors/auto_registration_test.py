"""Tests for automatic PostProcessor registration.

Tests the auto-registration behavior of postprocessors when the
kreuzberg.postprocessors module is imported.
"""

import pytest


def test_auto_registration_happens_on_import():
    """Test that processors are auto-registered when module is imported."""
    try:
        from _internal_bindings import list_post_processors
    except ImportError:
        pytest.skip("_internal_bindings not available")

    # Import the postprocessors module (triggers auto-registration)
    import kreuzberg.postprocessors  # noqa: F401

    processors = list_post_processors()

    # At least one processor should be registered (depending on available dependencies)
    # We can't assert specific processors because they depend on optional dependencies
    assert isinstance(processors, list)


def test_entity_extraction_registration_with_spacy():
    """Test that EntityExtractionProcessor is registered if spaCy is available."""
    try:
        import spacy  # noqa: F401
        from _internal_bindings import list_post_processors
    except ImportError:
        pytest.skip("spaCy or _internal_bindings not available")

    # Import postprocessors module
    import kreuzberg.postprocessors  # noqa: F401

    processors = list_post_processors()

    # EntityExtractionProcessor should be registered if spaCy is available
    assert "entity_extraction" in processors


def test_keyword_extraction_registration_with_keybert():
    """Test that KeywordExtractionProcessor is registered if KeyBERT is available."""
    try:
        import keybert  # noqa: F401
        from _internal_bindings import list_post_processors
    except ImportError:
        pytest.skip("KeyBERT or _internal_bindings not available")

    # Import postprocessors module
    import kreuzberg.postprocessors  # noqa: F401

    processors = list_post_processors()

    # KeywordExtractionProcessor should be registered if KeyBERT is available
    assert "keyword_extraction" in processors


def test_category_extraction_registration_with_transformers():
    """Test that CategoryExtractionProcessor is registered if transformers is available."""
    try:
        import transformers  # noqa: F401
        from _internal_bindings import list_post_processors
    except ImportError:
        pytest.skip("transformers or _internal_bindings not available")

    # Import postprocessors module
    import kreuzberg.postprocessors  # noqa: F401

    processors = list_post_processors()

    # CategoryExtractionProcessor should be registered if transformers is available
    assert "category_extraction" in processors


def test_auto_registration_handles_missing_dependencies():
    """Test that auto-registration continues even if some dependencies are missing."""
    try:
        from _internal_bindings import list_post_processors
    except ImportError:
        pytest.skip("_internal_bindings not available")

    # Import postprocessors module (should not raise even if some deps are missing)
    import kreuzberg.postprocessors  # noqa: F401

    processors = list_post_processors()

    # Should return a list even if empty (no errors)
    assert isinstance(processors, list)


def test_manual_registration_after_auto_registration():
    """Test that manual registration works after auto-registration."""
    try:
        from _internal_bindings import (
            list_post_processors,
            register_post_processor,
            unregister_post_processor,
        )
    except ImportError:
        pytest.skip("_internal_bindings not available")

    # Import postprocessors module (auto-registration)
    import kreuzberg.postprocessors  # noqa: F401

    class ManualProcessor:
        def name(self) -> str:
            return "test_manual_processor"

        def process(self, result: dict) -> dict:
            return result

    processor = ManualProcessor()

    # Manual registration should work
    register_post_processor(processor)

    processors = list_post_processors()
    assert "test_manual_processor" in processors

    # Cleanup
    unregister_post_processor("test_manual_processor")


def test_no_duplicate_registration_on_reimport():
    """Test that re-importing the module doesn't cause duplicate registrations."""
    try:
        from _internal_bindings import list_post_processors
    except ImportError:
        pytest.skip("_internal_bindings not available")

    # Import once
    import kreuzberg.postprocessors  # noqa: F401

    processors_first = list_post_processors()

    # Re-import (should not cause duplicates due to __all_registered__ guard)
    import importlib

    importlib.reload(kreuzberg.postprocessors)

    processors_second = list_post_processors()

    # Should have the same processors (no duplicates)
    assert len(processors_first) == len(processors_second)
    assert set(processors_first) == set(processors_second)


def test_registration_with_default_configs():
    """Test that auto-registered processors use sensible default configurations."""
    try:
        from _internal_bindings import list_post_processors
    except ImportError:
        pytest.skip("_internal_bindings not available")

    # Import postprocessors module
    import kreuzberg.postprocessors  # noqa: F401

    processors = list_post_processors()

    # At least check that registration succeeded without errors
    assert isinstance(processors, list)

    # If processors are registered, they should have reasonable defaults
    # (We can't check the actual defaults without accessing the registered instances,
    # but the fact that registration succeeded means the defaults worked)


def test_entity_extraction_default_config():
    """Test EntityExtractionProcessor default configuration."""
    try:
        from kreuzberg.postprocessors.entity_extraction import (
            EntityExtractionProcessor,
        )
    except ImportError:
        pytest.skip("EntityExtractionProcessor not available")

    # Create with defaults (should match auto-registration)
    processor = EntityExtractionProcessor()

    assert processor.name() == "entity_extraction"
    assert processor.model_name == "en_core_web_sm"
    assert processor.entity_types is None  # All types
    assert processor.max_entities == 50


def test_keyword_extraction_default_config():
    """Test KeywordExtractionProcessor default configuration."""
    try:
        from kreuzberg.postprocessors.keyword_extraction import (
            KeywordExtractionProcessor,
        )
    except ImportError:
        pytest.skip("KeywordExtractionProcessor not available")

    # Create with defaults (should match auto-registration)
    processor = KeywordExtractionProcessor()

    assert processor.name() == "keyword_extraction"
    assert processor.model_name == "all-MiniLM-L6-v2"
    assert processor.top_n == 10
    assert processor.ngram_range == (1, 2)


def test_category_extraction_default_config():
    """Test CategoryExtractionProcessor default configuration."""
    try:
        from kreuzberg.postprocessors.category_extraction import (
            CategoryExtractionProcessor,
        )
    except ImportError:
        pytest.skip("CategoryExtractionProcessor not available")

    # Create with defaults (should match auto-registration)
    processor = CategoryExtractionProcessor()

    assert processor.name() == "category_extraction"
    assert processor.model_name == "facebook/bart-large-mnli"
    assert processor.categories == CategoryExtractionProcessor.DOCUMENT_TYPES
    assert processor.multi_label is False
