"""Tests for EntityExtractionProcessor.

Tests entity extraction using spaCy NER, including model configuration,
entity filtering, and error handling.
"""

import pytest


class TestEntityExtractionProcessor:
    """Test EntityExtractionProcessor functionality."""

    def test_processor_initialization(self):
        """Test processor can be initialized with default config."""
        try:
            from kreuzberg.postprocessors.entity_extraction import (
                EntityExtractionProcessor,
            )
        except ImportError:
            pytest.skip("EntityExtractionProcessor not available")

        processor = EntityExtractionProcessor()
        assert processor.name() == "entity_extraction"
        assert processor.processing_stage() == "early"
        assert processor.version() == "1.0.0"

    def test_processor_initialization_with_custom_config(self):
        """Test processor can be initialized with custom configuration."""
        try:
            from kreuzberg.postprocessors.entity_extraction import (
                EntityExtractionProcessor,
            )
        except ImportError:
            pytest.skip("EntityExtractionProcessor not available")

        processor = EntityExtractionProcessor(
            model="en_core_web_md",
            entity_types=["PERSON", "ORG"],
            max_entities=10,
            min_confidence=0.5,
        )

        assert processor.model_name == "en_core_web_md"
        assert processor.entity_types == ["PERSON", "ORG"]
        assert processor.max_entities == 10
        assert processor.min_confidence == 0.5

    def test_processor_with_model_path(self):
        """Test processor can be configured with a custom model path."""
        try:
            from kreuzberg.postprocessors.entity_extraction import (
                EntityExtractionProcessor,
            )
        except ImportError:
            pytest.skip("EntityExtractionProcessor not available")

        processor = EntityExtractionProcessor(model="en_core_web_sm", model_path="/custom/path/to/model")

        assert processor.model_path == "/custom/path/to/model"

    def test_processor_with_model_kwargs(self):
        """Test processor accepts and stores model kwargs."""
        try:
            from kreuzberg.postprocessors.entity_extraction import (
                EntityExtractionProcessor,
            )
        except ImportError:
            pytest.skip("EntityExtractionProcessor not available")

        processor = EntityExtractionProcessor(disable=["parser", "tagger"], exclude=["lemmatizer"])

        assert processor.model_kwargs["disable"] == ["parser", "tagger"]
        assert processor.model_kwargs["exclude"] == ["lemmatizer"]

    def test_process_empty_content(self):
        """Test processor handles empty content gracefully."""
        try:
            from kreuzberg.postprocessors.entity_extraction import (
                EntityExtractionProcessor,
            )
        except ImportError:
            pytest.skip("EntityExtractionProcessor not available")

        processor = EntityExtractionProcessor()
        result = {"content": "", "metadata": {}}

        processed = processor.process(result)

        # Should return result unchanged
        assert processed == result

    def test_process_missing_content(self):
        """Test processor handles missing content field gracefully."""
        try:
            from kreuzberg.postprocessors.entity_extraction import (
                EntityExtractionProcessor,
            )
        except ImportError:
            pytest.skip("EntityExtractionProcessor not available")

        processor = EntityExtractionProcessor()
        result = {"metadata": {}}

        processed = processor.process(result)

        # Should return result unchanged
        assert processed == result

    def test_process_non_string_content(self):
        """Test processor handles non-string content gracefully."""
        try:
            from kreuzberg.postprocessors.entity_extraction import (
                EntityExtractionProcessor,
            )
        except ImportError:
            pytest.skip("EntityExtractionProcessor not available")

        processor = EntityExtractionProcessor()
        result = {"content": 12345, "metadata": {}}

        processed = processor.process(result)

        # Should return result unchanged
        assert processed == result

    @pytest.mark.skipif(
        pytest.importorskip("spacy", reason="spaCy not installed") is None,
        reason="spaCy not installed",
    )
    def test_process_with_spacy(self):
        """Test processor extracts entities using spaCy (if available)."""
        try:
            import spacy

            from kreuzberg.postprocessors.entity_extraction import (
                EntityExtractionProcessor,
            )
        except ImportError:
            pytest.skip("spaCy or EntityExtractionProcessor not available")

        # Try to load spaCy model
        try:
            spacy.load("en_core_web_sm")
        except OSError:
            pytest.skip("spaCy model en_core_web_sm not installed")

        processor = EntityExtractionProcessor()
        result = {
            "content": "John Doe works at Microsoft in Seattle.",
            "metadata": {},
        }

        processed = processor.process(result)

        # Should have entities in metadata
        assert "entities" in processed["metadata"]
        assert isinstance(processed["metadata"]["entities"], dict)

        # Should have entity_count
        assert "entity_count" in processed["metadata"]
        assert isinstance(processed["metadata"]["entity_count"], int)

    @pytest.mark.skipif(
        pytest.importorskip("spacy", reason="spaCy not installed") is None,
        reason="spaCy not installed",
    )
    def test_entity_type_filtering(self):
        """Test that entity_types parameter filters entities correctly."""
        try:
            import spacy

            from kreuzberg.postprocessors.entity_extraction import (
                EntityExtractionProcessor,
            )
        except ImportError:
            pytest.skip("spaCy or EntityExtractionProcessor not available")

        try:
            spacy.load("en_core_web_sm")
        except OSError:
            pytest.skip("spaCy model en_core_web_sm not installed")

        # Only extract PERSON entities
        processor = EntityExtractionProcessor(entity_types=["PERSON"])
        result = {
            "content": "John Doe works at Microsoft in Seattle.",
            "metadata": {},
        }

        processed = processor.process(result)

        entities = processed["metadata"]["entities"]

        # Should only have PERSON entities
        assert "PERSON" in entities or len(entities) == 0
        assert "ORG" not in entities
        assert "GPE" not in entities

    @pytest.mark.skipif(
        pytest.importorskip("spacy", reason="spaCy not installed") is None,
        reason="spaCy not installed",
    )
    def test_max_entities_limit(self):
        """Test that max_entities parameter limits entities per type."""
        try:
            import spacy

            from kreuzberg.postprocessors.entity_extraction import (
                EntityExtractionProcessor,
            )
        except ImportError:
            pytest.skip("spaCy or EntityExtractionProcessor not available")

        try:
            spacy.load("en_core_web_sm")
        except OSError:
            pytest.skip("spaCy model en_core_web_sm not installed")

        processor = EntityExtractionProcessor(max_entities=2)
        result = {
            "content": "Alice works with Bob and Charlie at Microsoft, Google, and Apple in Seattle, Portland, and San Francisco.",
            "metadata": {},
        }

        processed = processor.process(result)

        entities = processed["metadata"]["entities"]

        # Each entity type should have at most 2 entities
        for _entity_type, entity_list in entities.items():
            assert len(entity_list) <= 2

    @pytest.mark.skip(reason="Module mocking too complex - tested via integration tests instead")
    def test_initialize_without_spacy(self):
        """Test that initialize() raises ImportError without spaCy.

        Note: This test is skipped because properly mocking module absence
        is complex and fragile. The behavior is tested in integration tests
        where spaCy is actually not installed.
        """
        pass

    def test_shutdown_releases_resources(self):
        """Test that shutdown() releases model resources."""
        try:
            from kreuzberg.postprocessors.entity_extraction import (
                EntityExtractionProcessor,
            )
        except ImportError:
            pytest.skip("EntityExtractionProcessor not available")

        processor = EntityExtractionProcessor()
        processor._nlp = "mock_model"  # Simulate loaded model

        processor.shutdown()

        assert processor._nlp is None

    def test_metadata_not_overwritten(self):
        """Test that processor doesn't overwrite existing metadata."""
        try:
            from kreuzberg.postprocessors.entity_extraction import (
                EntityExtractionProcessor,
            )
        except ImportError:
            pytest.skip("EntityExtractionProcessor not available")

        processor = EntityExtractionProcessor()
        result = {
            "content": "Some text",
            "metadata": {"entities": {"EXISTING": ["data"]}, "entity_count": 999},
        }

        processed = processor.process(result)

        # Existing metadata should be preserved
        assert processed["metadata"]["entities"] == {"EXISTING": ["data"]}
        assert processed["metadata"]["entity_count"] == 999
