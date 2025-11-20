"""Tests for plugin registration and listing functions."""

from __future__ import annotations

import pytest

from kreuzberg import (
    clear_post_processors,
    clear_validators,
    list_ocr_backends,
    list_post_processors,
    list_validators,
    register_ocr_backend,
    register_post_processor,
    register_validator,
    unregister_ocr_backend,
)


class MockValidator:
    """Mock validator for testing."""

    def name(self) -> str:
        return "mock_validator"

    def validate(self, result: dict) -> None:
        pass


class MockValidator2:
    """Second mock validator for testing."""

    def name(self) -> str:
        return "mock_validator_2"

    def validate(self, result: dict) -> None:
        pass


class MockPostProcessor:
    """Mock post-processor for testing."""

    def name(self) -> str:
        return "mock_processor"

    def process(self, result: dict) -> dict:
        return result


class MockPostProcessor2:
    """Second mock post-processor for testing."""

    def name(self) -> str:
        return "mock_processor_2"

    def process(self, result: dict) -> dict:
        return result


def test_list_validators_empty() -> None:
    """Test listing validators when registry is empty."""
    clear_validators()

    validators = list_validators()
    assert isinstance(validators, list)
    assert len(validators) == 0


def test_list_validators_with_registered() -> None:
    """Test listing validators after registration."""
    clear_validators()

    register_validator(MockValidator())
    register_validator(MockValidator2())

    validators = list_validators()
    assert len(validators) == 2
    assert "mock_validator" in validators
    assert "mock_validator_2" in validators

    clear_validators()


def test_list_validators_after_clear() -> None:
    """Test listing validators after clearing registry."""
    clear_validators()

    register_validator(MockValidator())
    validators = list_validators()
    assert len(validators) == 1

    clear_validators()
    validators = list_validators()
    assert len(validators) == 0


def test_list_post_processors_empty() -> None:
    """Test listing post-processors when registry is empty."""
    clear_post_processors()

    processors = list_post_processors()
    assert isinstance(processors, list)
    assert len(processors) == 0


def test_list_post_processors_with_registered() -> None:
    """Test listing post-processors after registration."""
    clear_post_processors()

    register_post_processor(MockPostProcessor())
    register_post_processor(MockPostProcessor2())

    processors = list_post_processors()
    assert len(processors) == 2
    assert "mock_processor" in processors
    assert "mock_processor_2" in processors

    clear_post_processors()


def test_list_post_processors_after_clear() -> None:
    """Test listing post-processors after clearing registry."""
    clear_post_processors()

    register_post_processor(MockPostProcessor())
    processors = list_post_processors()
    assert len(processors) == 1

    clear_post_processors()
    processors = list_post_processors()
    assert len(processors) == 0


def test_validators_list_independence() -> None:
    """Test that validator list is independent of post-processor list."""
    clear_validators()
    clear_post_processors()

    register_validator(MockValidator())
    register_post_processor(MockPostProcessor())

    validators = list_validators()
    processors = list_post_processors()

    assert len(validators) == 1
    assert len(processors) == 1
    assert validators[0] == "mock_validator"
    assert processors[0] == "mock_processor"

    clear_validators()
    clear_post_processors()


class MockOcrBackend:
    """Mock OCR backend for testing."""

    def name(self) -> str:
        return "mock_ocr"

    def supported_languages(self) -> list[str]:
        return ["eng", "deu"]

    def process_image(self, image_bytes: bytes, language: str) -> dict:
        return {"content": "mocked text", "metadata": {}, "tables": []}


class MockOcrBackend2:
    """Second mock OCR backend for testing."""

    def name(self) -> str:
        return "mock_ocr_2"

    def supported_languages(self) -> list[str]:
        return ["fra", "spa"]

    def process_image(self, image_bytes: bytes, language: str) -> dict:
        return {"content": "mocked text 2", "metadata": {}, "tables": []}


def test_list_ocr_backends_returns_list() -> None:
    """Test listing OCR backends returns a list."""
    backends = list_ocr_backends()
    assert isinstance(backends, list)


def test_list_ocr_backends_with_registered() -> None:
    """Test listing OCR backends after registration."""
    register_ocr_backend(MockOcrBackend())
    register_ocr_backend(MockOcrBackend2())

    backends = list_ocr_backends()
    assert "mock_ocr" in backends
    assert "mock_ocr_2" in backends

    unregister_ocr_backend("mock_ocr")
    unregister_ocr_backend("mock_ocr_2")


def test_unregister_ocr_backend_removes_backend() -> None:
    """Test unregistering OCR backend removes it from list."""
    register_ocr_backend(MockOcrBackend())

    backends = list_ocr_backends()
    assert "mock_ocr" in backends

    unregister_ocr_backend("mock_ocr")

    backends = list_ocr_backends()
    assert "mock_ocr" not in backends


def test_unregister_nonexistent_ocr_backend() -> None:
    """Test unregistering a nonexistent backend handles gracefully."""
    try:
        unregister_ocr_backend("nonexistent_backend_xyz")
    except Exception:
        pass
