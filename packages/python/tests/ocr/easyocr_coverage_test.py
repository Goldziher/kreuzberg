"""Tests to improve coverage for kreuzberg/ocr/easyocr.py."""

from __future__ import annotations

from unittest.mock import Mock, patch

import pytest

from kreuzberg.exceptions import OCRError, ValidationError


def test_easyocr_import_error() -> None:
    """Test EasyOCRBackend raises ImportError when easyocr not installed."""
    from kreuzberg.ocr.easyocr import EasyOCRBackend

    with patch.dict("sys.modules", {"easyocr": None}):
        with pytest.raises(ImportError) as exc_info:
            EasyOCRBackend()

        assert "kreuzberg[easyocr]" in str(exc_info.value)


def test_easyocr_unsupported_language() -> None:
    """Test EasyOCRBackend raises ValidationError for unsupported language."""
    pytest.importorskip("easyocr", reason="EasyOCR not installed")

    from kreuzberg.ocr.easyocr import EasyOCRBackend

    with pytest.raises(ValidationError) as exc_info:
        EasyOCRBackend(languages=["invalid_lang"])

    assert "Unsupported EasyOCR language codes" in str(exc_info.value)
    assert exc_info.value.context is not None
    assert "unsupported_languages" in exc_info.value.context


def test_easyocr_initialize_idempotent() -> None:
    """Test EasyOCRBackend.initialize is idempotent."""
    easyocr = pytest.importorskip("easyocr", reason="EasyOCR not installed")

    from kreuzberg.ocr.easyocr import EasyOCRBackend

    mock_reader = Mock()

    with patch.object(easyocr, "Reader", return_value=mock_reader):
        backend = EasyOCRBackend(languages=["en"], use_gpu=False)

        backend.initialize()
        first_reader = backend._reader

        # Second call should return early
        backend.initialize()
        second_reader = backend._reader

        assert first_reader is second_reader
        # Reader should only be constructed once
        easyocr.Reader.assert_called_once()


def test_easyocr_initialize_failure() -> None:
    """Test EasyOCRBackend.initialize raises OCRError on failure."""
    easyocr = pytest.importorskip("easyocr", reason="EasyOCR not installed")

    from kreuzberg.ocr.easyocr import EasyOCRBackend

    with patch.object(easyocr, "Reader", side_effect=RuntimeError("Model load failed")):
        backend = EasyOCRBackend(languages=["en"], use_gpu=False)

        with pytest.raises(OCRError) as exc_info:
            backend.initialize()

        assert "Failed to initialize EasyOCR" in str(exc_info.value)


def test_easyocr_process_image_reader_none_after_init() -> None:
    """Test process_image raises RuntimeError when reader fails to initialize."""
    pytest.importorskip("easyocr", reason="EasyOCR not installed")

    from kreuzberg.ocr.easyocr import EasyOCRBackend

    backend = EasyOCRBackend(languages=["en"], use_gpu=False)

    # Mock initialize to succeed but leave _reader as None
    with patch.object(backend, "initialize"):
        backend._reader = None

        with pytest.raises(RuntimeError) as exc_info:
            backend.process_image(b"fake image data", "en")

        assert "EasyOCR reader failed to initialize" in str(exc_info.value)


def test_easyocr_process_image_unsupported_language() -> None:
    """Test process_image raises ValidationError for unsupported language."""
    pytest.importorskip("easyocr", reason="EasyOCR not installed")

    from kreuzberg.ocr.easyocr import EasyOCRBackend

    backend = EasyOCRBackend(languages=["en"], use_gpu=False)

    # Set reader to bypass initialization
    backend._reader = Mock()

    with pytest.raises(ValidationError) as exc_info:
        backend.process_image(b"fake image data", "invalid_lang")

    assert "Language 'invalid_lang' not supported" in str(exc_info.value)


def test_easyocr_process_easyocr_result_empty() -> None:
    """Test _process_easyocr_result with empty result."""
    from kreuzberg.ocr.easyocr import EasyOCRBackend

    content, confidence, text_regions = EasyOCRBackend._process_easyocr_result([])

    assert content == ""
    assert confidence == 0.0
    assert text_regions == 0


def test_easyocr_process_easyocr_result_two_item_format() -> None:
    """Test _process_easyocr_result with 2-item format (text, confidence)."""
    from kreuzberg.ocr.easyocr import EasyOCRBackend

    # 2-item format: [(text, confidence), ...]
    result = [
        ("Hello", 0.95),
        ("World", 0.90),
        ("", 0.85),  # Empty text should be skipped from text_parts but confidence still added
    ]

    content, confidence, text_regions = EasyOCRBackend._process_easyocr_result(result)

    assert content == "Hello\nWorld"
    # Confidence only adds for non-empty text but divides by total count
    assert confidence == pytest.approx((0.95 + 0.90) / 3)
    assert text_regions == 3


def test_easyocr_process_easyocr_result_line_grouping() -> None:
    """Test _process_easyocr_result line grouping logic."""
    from kreuzberg.ocr.easyocr import EasyOCRBackend

    # 3-item format: [(box, text, confidence), ...]
    # Create items with different y-coordinates to test line grouping
    result = [
        # Line 1 (y ~ 10)
        ([[0, 10], [50, 10], [50, 30], [0, 30]], "Hello", 0.95),
        ([[60, 10], [110, 10], [110, 30], [60, 30]], "World", 0.90),
        # Line 2 (y ~ 50) - more than 20 pixels away
        ([[0, 50], [50, 50], [50, 70], [0, 70]], "Second", 0.88),
        ([[60, 50], [110, 50], [110, 70], [60, 70]], "Line", 0.87),
    ]

    content, confidence, text_regions = EasyOCRBackend._process_easyocr_result(result)

    assert "Hello World" in content
    assert "Second Line" in content
    assert content.count("\n") >= 1  # At least one line break
    assert confidence > 0
    assert text_regions == 4


def test_easyocr_process_easyocr_result_single_line_group() -> None:
    """Test _process_easyocr_result with single line group."""
    from kreuzberg.ocr.easyocr import EasyOCRBackend

    # All items on same line (y ~ 10)
    result = [
        ([[0, 10], [50, 10], [50, 30], [0, 30]], "One", 0.95),
        ([[60, 12], [110, 12], [110, 32], [60, 32]], "Line", 0.90),
    ]

    content, confidence, text_regions = EasyOCRBackend._process_easyocr_result(result)

    assert content == "One Line"
    assert confidence == pytest.approx((0.95 + 0.90) / 2)
    assert text_regions == 2


def test_easyocr_process_easyocr_result_empty_text_in_line() -> None:
    """Test _process_easyocr_result with empty text items in line grouping."""
    from kreuzberg.ocr.easyocr import EasyOCRBackend

    result = [
        ([[0, 10], [50, 10], [50, 30], [0, 30]], "Hello", 0.95),
        ([[60, 10], [110, 10], [110, 30], [60, 30]], "", 0.90),  # Empty text
        ([[120, 10], [170, 10], [170, 30], [120, 30]], "World", 0.88),
    ]

    content, confidence, text_regions = EasyOCRBackend._process_easyocr_result(result)

    assert content == "Hello World"
    # Confidence includes empty text item
    assert confidence == pytest.approx((0.95 + 0.88) / 2)
    assert text_regions == 2


def test_easyocr_is_cuda_available_no_torch() -> None:
    """Test _is_cuda_available returns False when torch not installed."""
    from kreuzberg.ocr.easyocr import EasyOCRBackend

    with patch.dict("sys.modules", {"torch": None}):
        with patch("builtins.__import__", side_effect=ImportError("no torch")):
            available = EasyOCRBackend._is_cuda_available()

            assert available is False


def test_easyocr_use_gpu_none_auto_detects() -> None:
    """Test use_gpu=None auto-detects CUDA availability."""
    pytest.importorskip("easyocr", reason="EasyOCR not installed")

    from kreuzberg.ocr.easyocr import EasyOCRBackend

    with patch.object(EasyOCRBackend, "_is_cuda_available", return_value=True):
        backend = EasyOCRBackend(languages=["en"], use_gpu=None)

        assert backend.use_gpu is True


def test_easyocr_use_gpu_explicit() -> None:
    """Test use_gpu can be explicitly set."""
    pytest.importorskip("easyocr", reason="EasyOCR not installed")

    from kreuzberg.ocr.easyocr import EasyOCRBackend

    backend = EasyOCRBackend(languages=["en"], use_gpu=True)

    assert backend.use_gpu is True


def test_easyocr_shutdown() -> None:
    """Test shutdown clears reader."""
    pytest.importorskip("easyocr", reason="EasyOCR not installed")

    from kreuzberg.ocr.easyocr import EasyOCRBackend

    backend = EasyOCRBackend(languages=["en"], use_gpu=False)
    backend._reader = Mock()

    backend.shutdown()

    assert backend._reader is None


def test_easyocr_name() -> None:
    """Test name returns 'easyocr'."""
    pytest.importorskip("easyocr", reason="EasyOCR not installed")

    from kreuzberg.ocr.easyocr import EasyOCRBackend

    backend = EasyOCRBackend(languages=["en"], use_gpu=False)

    assert backend.name() == "easyocr"


def test_easyocr_supported_languages() -> None:
    """Test supported_languages returns sorted list."""
    pytest.importorskip("easyocr", reason="EasyOCR not installed")

    from kreuzberg.ocr.easyocr import EasyOCRBackend

    backend = EasyOCRBackend(languages=["en"], use_gpu=False)
    supported = backend.supported_languages()

    assert isinstance(supported, list)
    assert len(supported) > 0
    assert supported == sorted(supported)
    assert "en" in supported
