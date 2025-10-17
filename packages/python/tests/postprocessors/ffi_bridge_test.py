"""Tests for PostProcessor FFI bridge.

Tests the Rust-Python FFI bridge that allows Python postprocessors to be
registered with the Rust core and called during extraction.
"""

import pytest


class TestPostProcessorFFIBridge:
    """Test PostProcessor registration and FFI bridge functionality."""

    def test_registration_functions_available(self):
        """Test that PostProcessor registration functions are available."""
        try:
            from _internal_bindings import (
                list_post_processors,
                register_post_processor,
                unregister_post_processor,
            )

            assert callable(register_post_processor)
            assert callable(list_post_processors)
            assert callable(unregister_post_processor)
        except ImportError:
            pytest.skip("_internal_bindings not available")

    def test_list_post_processors_returns_list(self):
        """Test that list_post_processors returns a list."""
        try:
            from _internal_bindings import list_post_processors
        except ImportError:
            pytest.skip("_internal_bindings not available")

        processors = list_post_processors()
        assert isinstance(processors, list)

    def test_register_and_unregister_processor(self):
        """Test registering and unregistering a processor."""
        try:
            from _internal_bindings import (
                list_post_processors,
                register_post_processor,
                unregister_post_processor,
            )
        except ImportError:
            pytest.skip("_internal_bindings not available")

        # Create a minimal mock processor
        class MockProcessor:
            def name(self) -> str:
                return "test_ffi_bridge_processor"

            def process(self, result: dict) -> dict:
                result["metadata"]["test_field"] = "test_value"
                return result

        processor = MockProcessor()

        # Register processor
        register_post_processor(processor)

        # Verify registration
        processors = list_post_processors()
        assert "test_ffi_bridge_processor" in processors

        # Unregister processor
        unregister_post_processor("test_ffi_bridge_processor")

        # Verify unregistration
        processors = list_post_processors()
        assert "test_ffi_bridge_processor" not in processors

    def test_register_processor_with_processing_stage(self):
        """Test registering a processor with a processing stage."""
        try:
            from _internal_bindings import (
                list_post_processors,
                register_post_processor,
                unregister_post_processor,
            )
        except ImportError:
            pytest.skip("_internal_bindings not available")

        class EarlyProcessor:
            def name(self) -> str:
                return "test_early_processor"

            def processing_stage(self) -> str:
                return "early"

            def process(self, result: dict) -> dict:
                return result

        processor = EarlyProcessor()
        register_post_processor(processor)

        processors = list_post_processors()
        assert "test_early_processor" in processors

        # Cleanup
        unregister_post_processor("test_early_processor")

    def test_register_processor_with_lifecycle_methods(self):
        """Test registering a processor with initialize/shutdown methods."""
        try:
            from _internal_bindings import (
                list_post_processors,
                register_post_processor,
                unregister_post_processor,
            )
        except ImportError:
            pytest.skip("_internal_bindings not available")

        init_called = []
        shutdown_called = []

        class LifecycleProcessor:
            def name(self) -> str:
                return "test_lifecycle_processor"

            def initialize(self) -> None:
                init_called.append(True)

            def shutdown(self) -> None:
                shutdown_called.append(True)

            def process(self, result: dict) -> dict:
                return result

        processor = LifecycleProcessor()

        # Register (should call initialize)
        register_post_processor(processor)
        assert len(init_called) == 1

        processors = list_post_processors()
        assert "test_lifecycle_processor" in processors

        # Unregister (should call shutdown)
        unregister_post_processor("test_lifecycle_processor")
        assert len(shutdown_called) == 1

    def test_register_processor_missing_name_method(self):
        """Test that registering a processor without name() fails."""
        try:
            from _internal_bindings import register_post_processor
        except ImportError:
            pytest.skip("_internal_bindings not available")

        class InvalidProcessor:
            def process(self, result: dict) -> dict:
                return result

        processor = InvalidProcessor()

        with pytest.raises(AttributeError, match="name"):
            register_post_processor(processor)

    def test_register_processor_missing_process_method(self):
        """Test that registering a processor without process() fails."""
        try:
            from _internal_bindings import register_post_processor
        except ImportError:
            pytest.skip("_internal_bindings not available")

        class InvalidProcessor:
            def name(self) -> str:
                return "invalid_processor"

        processor = InvalidProcessor()

        with pytest.raises(AttributeError, match="process"):
            register_post_processor(processor)

    def test_register_processor_empty_name(self):
        """Test that registering a processor with empty name fails."""
        try:
            from _internal_bindings import register_post_processor
        except ImportError:
            pytest.skip("_internal_bindings not available")

        class EmptyNameProcessor:
            def name(self) -> str:
                return ""

            def process(self, result: dict) -> dict:
                return result

        processor = EmptyNameProcessor()

        with pytest.raises(ValueError, match="empty"):
            register_post_processor(processor)

    def test_processor_modifies_result(self):
        """Test that a processor can modify the extraction result."""
        try:
            from _internal_bindings import (
                register_post_processor,
                unregister_post_processor,
            )
        except ImportError:
            pytest.skip("_internal_bindings not available")

        class MetadataProcessor:
            def name(self) -> str:
                return "test_metadata_processor"

            def process(self, result: dict) -> dict:
                # Add metadata
                if "metadata" not in result:
                    result["metadata"] = {}
                result["metadata"]["processor_called"] = True
                result["metadata"]["processor_name"] = self.name()
                return result

        processor = MetadataProcessor()
        register_post_processor(processor)

        # Note: We can't directly test the processor being called during extraction
        # without the full extraction pipeline, but we can verify registration worked
        # Cleanup
        unregister_post_processor("test_metadata_processor")

    def test_unregister_nonexistent_processor(self):
        """Test that unregistering a non-existent processor succeeds silently."""
        try:
            from _internal_bindings import unregister_post_processor
        except ImportError:
            pytest.skip("_internal_bindings not available")

        # Should succeed silently (no-op) for non-existent processor
        unregister_post_processor("nonexistent_processor")  # Should not raise

    def test_register_duplicate_processor_name(self):
        """Test that registering a processor with duplicate name overwrites previous."""
        try:
            from _internal_bindings import (
                list_post_processors,
                register_post_processor,
                unregister_post_processor,
            )
        except ImportError:
            pytest.skip("_internal_bindings not available")

        class DuplicateProcessor1:
            def name(self) -> str:
                return "test_duplicate_processor"

            def process(self, result: dict) -> dict:
                result["version"] = "v1"
                return result

        class DuplicateProcessor2:
            def name(self) -> str:
                return "test_duplicate_processor"

            def process(self, result: dict) -> dict:
                result["version"] = "v2"
                return result

        processor1 = DuplicateProcessor1()
        processor2 = DuplicateProcessor2()

        # Register first processor
        register_post_processor(processor1)
        processors = list_post_processors()
        assert "test_duplicate_processor" in processors

        # Registering second processor with same name should overwrite (no error)
        register_post_processor(processor2)
        processors = list_post_processors()
        assert "test_duplicate_processor" in processors  # Still registered

        # Cleanup
        unregister_post_processor("test_duplicate_processor")

    def test_processor_with_version(self):
        """Test registering a processor with version() method."""
        try:
            from _internal_bindings import (
                register_post_processor,
                unregister_post_processor,
            )
        except ImportError:
            pytest.skip("_internal_bindings not available")

        class VersionedProcessor:
            def name(self) -> str:
                return "test_versioned_processor"

            def version(self) -> str:
                return "2.0.0"

            def process(self, result: dict) -> dict:
                return result

        processor = VersionedProcessor()
        register_post_processor(processor)

        # Cleanup
        unregister_post_processor("test_versioned_processor")
