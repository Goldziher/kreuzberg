def test_check_ocr_backend_unknown() -> None:
    from kreuzberg._api.main import _check_ocr_backend_available

    result = _check_ocr_backend_available("unknown_backend")  # type: ignore[arg-type]
    assert result is False


def test_check_feature_backend_unknown() -> None:
    from kreuzberg._api.main import _check_feature_backend_available

    result = _check_feature_backend_available("unknown_feature")  # type: ignore[arg-type]
    assert result is False


def test_bytes_encoder() -> None:
    from kreuzberg._api.main import _bytes_encoder

    test_bytes = b"hello world"
    result = _bytes_encoder(test_bytes)

    assert isinstance(result, str)
    assert result == "aGVsbG8gd29ybGQ="
