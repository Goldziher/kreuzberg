from __future__ import annotations

import json

from kreuzberg.exceptions import (
    KreuzbergError,
    MissingDependencyError,
    OCRError,
    ParsingError,
    ValidationError,
)


def test_kreuzberg_error_basic() -> None:
    error = KreuzbergError("Test error message")
    assert str(error) == "KreuzbergError: Test error message"
    assert error.context is None


def test_kreuzberg_error_with_context() -> None:
    context = {"file": "test.pdf", "page": 1}
    error = KreuzbergError("Failed to parse", context=context)

    error_str = str(error)
    assert "KreuzbergError: Failed to parse" in error_str
    assert "Context:" in error_str
    assert '"file": "test.pdf"' in error_str
    assert '"page": 1' in error_str


def test_kreuzberg_error_serialize_bytes_context() -> None:
    context = {"data": b"binary data", "other": "text"}
    error = KreuzbergError("Binary error", context=context)

    error_str = str(error)
    assert "binary data" in error_str
    assert "b'" not in error_str


def test_kreuzberg_error_serialize_list_context() -> None:
    context = {"items": [1, "two", b"three", {"nested": "dict"}], "tuple_items": (1, 2, 3)}
    error = KreuzbergError("List error", context=context)

    error_str = str(error)
    assert '"items": [1, "two", "three"' in error_str
    assert '"tuple_items": [1, 2, 3]' in error_str


def test_kreuzberg_error_serialize_exception_context() -> None:
    inner_error = ValueError("Inner error message")
    context = {"original_error": inner_error, "file": "test.txt"}
    error = KreuzbergError("Wrapper error", context=context)

    error_str = str(error)
    assert '"type": "ValueError"' in error_str
    assert '"message": "Inner error message"' in error_str


def test_kreuzberg_error_complex_nested_context() -> None:
    context = {
        "level1": {
            "level2": {
                "bytes_data": b"nested bytes",
                "list_data": [b"item1", "item2"],
                "error": RuntimeError("nested error"),
            },
        },
    }
    error = KreuzbergError("Complex error", context=context)

    error_str = str(error)
    assert "nested bytes" in error_str
    assert "item1" in error_str
    assert '"type": "RuntimeError"' in error_str


def test_kreuzberg_error_no_context() -> None:
    error = KreuzbergError("Simple error")
    error_str = str(error)

    assert error_str == "KreuzbergError: Simple error"
    assert "Context:" not in error_str


def test_parsing_error() -> None:
    error = ParsingError("Failed to parse document", context={"format": "pdf"})
    assert isinstance(error, KreuzbergError)
    assert "ParsingError" in str(error)
    assert "Failed to parse document" in str(error)


def test_validation_error() -> None:
    error = ValidationError("Invalid input", context={"field": "email"})
    assert isinstance(error, KreuzbergError)
    assert "ValidationError" in str(error)
    assert "Invalid input" in str(error)


def test_ocr_error() -> None:
    error = OCRError("OCR processing failed", context={"engine": "tesseract"})
    assert isinstance(error, KreuzbergError)
    assert "OCRError" in str(error)
    assert "OCR processing failed" in str(error)


def test_missing_dependency_error_create_for_package() -> None:
    error = MissingDependencyError.create_for_package(
        dependency_group="ocr",
        functionality="OCR processing",
        package_name="pytesseract",
    )

    assert isinstance(error, MissingDependencyError)
    assert isinstance(error, KreuzbergError)

    error_str = str(error)
    assert "MissingDependencyError" in error_str
    assert "pytesseract" in error_str
    assert "OCR processing" in error_str
    assert "kreuzberg['ocr']" in error_str


def test_missing_dependency_error_direct() -> None:
    error = MissingDependencyError("Package not found", context={"package": "numpy"})
    assert isinstance(error, KreuzbergError)
    assert "MissingDependencyError" in str(error)
    assert "Package not found" in str(error)


def test_error_context_json_serializable() -> None:
    context = {
        "string": "value",
        "number": 42,
        "float": 3.14,
        "bool": True,
        "none": None,
        "list": [1, 2, 3],
        "dict": {"nested": "value"},
        "bytes": b"binary",
        "exception": ValueError("test"),
    }

    error = KreuzbergError("Test", context=context)
    error_str = str(error)

    context_start = error_str.index("Context: ") + len("Context: ")
    context_json = error_str[context_start:]

    parsed = json.loads(context_json)
    assert parsed["string"] == "value"
    assert parsed["number"] == 42
    assert parsed["bytes"] == "binary"
    assert parsed["exception"]["type"] == "ValueError"


def test_error_inheritance() -> None:
    errors = [ParsingError("test"), ValidationError("test"), MissingDependencyError("test"), OCRError("test")]

    for error in errors:
        assert isinstance(error, KreuzbergError)
        assert isinstance(error, Exception)
