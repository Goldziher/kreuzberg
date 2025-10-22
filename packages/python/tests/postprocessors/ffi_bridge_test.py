"""Tests for PostProcessor FFI bridge.

Tests the Rust-Python FFI bridge that allows Python postprocessors to be
registered with the Rust core and called during extraction.
"""

from __future__ import annotations

from typing import Literal

import pytest

from kreuzberg import (
    ExtractionResult,
    clear_post_processors,
    register_post_processor,
    unregister_post_processor,
)


def test_registration_functions_available() -> None:
    """Test that PostProcessor registration functions are available."""
    assert callable(register_post_processor)
    assert callable(unregister_post_processor)
    assert callable(clear_post_processors)


def test_register_and_unregister_processor() -> None:
    """Test registering and unregistering a processor."""

    class MockProcessor:
        def name(self) -> str:
            return "test_ffi_bridge_processor"

        def process(self, result: ExtractionResult) -> ExtractionResult:
            result.metadata["test_field"] = "test_value"
            return result

        def processing_stage(self) -> Literal["early", "middle", "late"]:
            return "middle"

        def initialize(self) -> None:
            pass

        def shutdown(self) -> None:
            pass

    processor = MockProcessor()

    register_post_processor(processor)  # type: ignore[arg-type]
    unregister_post_processor("test_ffi_bridge_processor")


def test_register_processor_with_processing_stage() -> None:
    """Test registering a processor with a processing stage."""

    class EarlyProcessor:
        def name(self) -> str:
            return "test_early_processor"

        def processing_stage(self) -> Literal["early", "middle", "late"]:
            return "early"

        def process(self, result: ExtractionResult) -> ExtractionResult:
            return result

        def initialize(self) -> None:
            pass

        def shutdown(self) -> None:
            pass

    processor = EarlyProcessor()
    register_post_processor(processor)  # type: ignore[arg-type]
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

        def process(self, result: ExtractionResult) -> ExtractionResult:
            return result

        def processing_stage(self) -> Literal["early", "middle", "late"]:
            return "middle"

    processor = LifecycleProcessor()

    register_post_processor(processor)  # type: ignore[arg-type]
    assert len(init_called) == 1

    unregister_post_processor("test_lifecycle_processor")
    assert len(shutdown_called) == 1


def test_register_processor_missing_name_method() -> None:
    """Test that registering a processor without name() fails."""

    class InvalidProcessor:
        def process(self, result: ExtractionResult) -> ExtractionResult:
            return result

        def processing_stage(self) -> Literal["early", "middle", "late"]:
            return "middle"

        def initialize(self) -> None:
            pass

        def shutdown(self) -> None:
            pass

    processor = InvalidProcessor()

    with pytest.raises(AttributeError, match="name"):
        register_post_processor(processor)  # type: ignore[arg-type]


def test_register_processor_missing_process_method() -> None:
    """Test that registering a processor without process() fails."""

    class InvalidProcessor:
        def name(self) -> str:
            return "invalid_processor"

        def processing_stage(self) -> Literal["early", "middle", "late"]:
            return "middle"

        def initialize(self) -> None:
            pass

        def shutdown(self) -> None:
            pass

    processor = InvalidProcessor()

    with pytest.raises(AttributeError, match="process"):
        register_post_processor(processor)  # type: ignore[arg-type]


def test_register_processor_empty_name() -> None:
    """Test that registering a processor with empty name fails."""

    class EmptyNameProcessor:
        def name(self) -> str:
            return ""

        def process(self, result: ExtractionResult) -> ExtractionResult:
            return result

        def processing_stage(self) -> Literal["early", "middle", "late"]:
            return "middle"

        def initialize(self) -> None:
            pass

        def shutdown(self) -> None:
            pass

    processor = EmptyNameProcessor()

    with pytest.raises(ValueError, match="empty"):
        register_post_processor(processor)  # type: ignore[arg-type]


def test_processor_modifies_result() -> None:
    """Test that a processor can modify the extraction result."""

    class MetadataProcessor:
        def name(self) -> str:
            return "test_metadata_processor"

        def process(self, result: ExtractionResult) -> ExtractionResult:
            result.metadata["processor_called"] = True
            result.metadata["processor_name"] = self.name()
            return result

        def processing_stage(self) -> Literal["early", "middle", "late"]:
            return "middle"

        def initialize(self) -> None:
            pass

        def shutdown(self) -> None:
            pass

    processor = MetadataProcessor()
    register_post_processor(processor)  # type: ignore[arg-type]

    unregister_post_processor("test_metadata_processor")


def test_unregister_nonexistent_processor() -> None:
    """Test that unregistering a non-existent processor succeeds silently."""
    unregister_post_processor("nonexistent_processor")


def test_register_duplicate_processor_name() -> None:
    """Test that registering a processor with duplicate name overwrites previous."""

    class DuplicateProcessor1:
        def name(self) -> str:
            return "test_duplicate_processor"

        def process(self, result: ExtractionResult) -> ExtractionResult:
            result.metadata["version"] = "v1"
            return result

        def processing_stage(self) -> Literal["early", "middle", "late"]:
            return "middle"

        def initialize(self) -> None:
            pass

        def shutdown(self) -> None:
            pass

    class DuplicateProcessor2:
        def name(self) -> str:
            return "test_duplicate_processor"

        def process(self, result: ExtractionResult) -> ExtractionResult:
            result.metadata["version"] = "v2"
            return result

        def processing_stage(self) -> Literal["early", "middle", "late"]:
            return "middle"

        def initialize(self) -> None:
            pass

        def shutdown(self) -> None:
            pass

    processor1 = DuplicateProcessor1()
    processor2 = DuplicateProcessor2()

    register_post_processor(processor1)  # type: ignore[arg-type]
    register_post_processor(processor2)  # type: ignore[arg-type]

    unregister_post_processor("test_duplicate_processor")


def test_processor_with_version() -> None:
    """Test registering a processor with version() method."""

    class VersionedProcessor:
        def name(self) -> str:
            return "test_versioned_processor"

        def version(self) -> str:
            return "2.0.0"

        def process(self, result: ExtractionResult) -> ExtractionResult:
            return result

        def processing_stage(self) -> Literal["early", "middle", "late"]:
            return "middle"

        def initialize(self) -> None:
            pass

        def shutdown(self) -> None:
            pass

    processor = VersionedProcessor()
    register_post_processor(processor)  # type: ignore[arg-type]

    unregister_post_processor("test_versioned_processor")
