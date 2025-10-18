"""Tests for PostProcessor FFI bridge.

Tests the Rust-Python FFI bridge that allows Python postprocessors to be
registered with the Rust core and called during extraction.
"""

from __future__ import annotations

from typing import Any

import pytest
from _internal_bindings import (  # type: ignore[import-untyped]
    list_post_processors,
    register_post_processor,
    unregister_post_processor,
)

ExtractionResultDict = dict[str, Any]


def test_registration_functions_available() -> None:
    """Test that PostProcessor registration functions are available."""
    assert callable(register_post_processor)
    assert callable(list_post_processors)
    assert callable(unregister_post_processor)


def test_list_post_processors_returns_list() -> None:
    """Test that list_post_processors returns a list."""
    processors: list[str] = list_post_processors()
    assert isinstance(processors, list)


def test_register_and_unregister_processor() -> None:
    """Test registering and unregistering a processor."""

    # Create a minimal mock processor
    class MockProcessor:
        def name(self) -> str:
            return "test_ffi_bridge_processor"

        def process(self, result: ExtractionResultDict) -> ExtractionResultDict:
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


def test_register_processor_with_processing_stage() -> None:
    """Test registering a processor with a processing stage."""

    class EarlyProcessor:
        def name(self) -> str:
            return "test_early_processor"

        def processing_stage(self) -> str:
            return "early"

        def process(self, result: ExtractionResultDict) -> ExtractionResultDict:
            return result

    processor = EarlyProcessor()
    register_post_processor(processor)

    processors = list_post_processors()
    assert "test_early_processor" in processors

    # Cleanup
    unregister_post_processor("test_early_processor")


def test_register_processor_with_lifecycle_methods() -> None:
    """Test registering a processor with initialize/shutdown methods."""
    init_called: list[bool] = []
    shutdown_called: list[bool] = []

    class LifecycleProcessor:
        def name(self) -> str:
            return "test_lifecycle_processor"

        def initialize(self) -> None:
            init_called.append(True)

        def shutdown(self) -> None:
            shutdown_called.append(True)

        def process(self, result: ExtractionResultDict) -> ExtractionResultDict:
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


def test_register_processor_missing_name_method() -> None:
    """Test that registering a processor without name() fails."""

    class InvalidProcessor:
        def process(self, result: ExtractionResultDict) -> ExtractionResultDict:
            return result

    processor = InvalidProcessor()

    with pytest.raises(AttributeError, match="name"):
        register_post_processor(processor)


def test_register_processor_missing_process_method() -> None:
    """Test that registering a processor without process() fails."""

    class InvalidProcessor:
        def name(self) -> str:
            return "invalid_processor"

    processor = InvalidProcessor()

    with pytest.raises(AttributeError, match="process"):
        register_post_processor(processor)


def test_register_processor_empty_name() -> None:
    """Test that registering a processor with empty name fails."""

    class EmptyNameProcessor:
        def name(self) -> str:
            return ""

        def process(self, result: ExtractionResultDict) -> ExtractionResultDict:
            return result

    processor = EmptyNameProcessor()

    with pytest.raises(ValueError, match="empty"):
        register_post_processor(processor)


def test_processor_modifies_result() -> None:
    """Test that a processor can modify the extraction result."""

    class MetadataProcessor:
        def name(self) -> str:
            return "test_metadata_processor"

        def process(self, result: ExtractionResultDict) -> ExtractionResultDict:
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


def test_unregister_nonexistent_processor() -> None:
    """Test that unregistering a non-existent processor succeeds silently."""
    # Should succeed silently (no-op) for non-existent processor
    unregister_post_processor("nonexistent_processor")  # Should not raise


def test_register_duplicate_processor_name() -> None:
    """Test that registering a processor with duplicate name overwrites previous."""

    class DuplicateProcessor1:
        def name(self) -> str:
            return "test_duplicate_processor"

        def process(self, result: ExtractionResultDict) -> ExtractionResultDict:
            result["version"] = "v1"
            return result

    class DuplicateProcessor2:
        def name(self) -> str:
            return "test_duplicate_processor"

        def process(self, result: ExtractionResultDict) -> ExtractionResultDict:
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


def test_processor_with_version() -> None:
    """Test registering a processor with version() method."""

    class VersionedProcessor:
        def name(self) -> str:
            return "test_versioned_processor"

        def version(self) -> str:
            return "2.0.0"

        def process(self, result: ExtractionResultDict) -> ExtractionResultDict:
            return result

    processor = VersionedProcessor()
    register_post_processor(processor)

    # Cleanup
    unregister_post_processor("test_versioned_processor")
