from typing import Any

import pytest


def test_check_ocr_backend_unknown() -> None:
    """Test OCR backend check with unknown backend name."""
    from kreuzberg._api.main import _check_ocr_backend_available

    # Unknown backend should return False (hits line 182)
    result = _check_ocr_backend_available("unknown_backend")  # type: ignore[arg-type]
    assert result is False


def test_check_feature_backend_unknown() -> None:
    """Test feature backend check with unknown backend name."""
    from kreuzberg._api.main import _check_feature_backend_available

    # Unknown backend should return False (hits line 198)
    result = _check_feature_backend_available("unknown_feature")  # type: ignore[arg-type]
    assert result is False


def test_bytes_encoder() -> None:
    """Test base64 bytes encoder function."""
    from kreuzberg._api.main import _bytes_encoder

    test_bytes = b"hello world"
    result = _bytes_encoder(test_bytes)

    # Should be base64 encoded
    assert isinstance(result, str)
    assert result == "aGVsbG8gd29ybGQ="
