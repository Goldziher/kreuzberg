"""Tests to improve coverage for kreuzberg/ocr/paddleocr.py."""

from __future__ import annotations

from unittest.mock import Mock, patch

import pytest

from kreuzberg.exceptions import OCRError, ValidationError


def test_paddleocr_import_error() -> None:
    """Test PaddleOCRBackend raises ImportError when paddleocr not installed."""
    from kreuzberg.ocr.paddleocr import PaddleOCRBackend

    with patch.dict("sys.modules", {"paddleocr": None}):
        with pytest.raises(ImportError) as exc_info:
            PaddleOCRBackend()

        assert "kreuzberg[paddleocr]" in str(exc_info.value)


def test_paddleocr_unsupported_language() -> None:
    """Test PaddleOCRBackend raises ValidationError for unsupported language."""
    pytest.importorskip("paddleocr", reason="PaddleOCR not installed")

    from kreuzberg.ocr.paddleocr import PaddleOCRBackend

    with pytest.raises(ValidationError) as exc_info:
        PaddleOCRBackend(lang="invalid_lang")

    assert "Unsupported PaddleOCR language code" in str(exc_info.value)
    assert exc_info.value.context is not None
    assert "language" in exc_info.value.context


def test_paddleocr_initialize_idempotent() -> None:
    """Test PaddleOCRBackend.initialize is idempotent."""
    paddleocr = pytest.importorskip("paddleocr", reason="PaddleOCR not installed")

    from kreuzberg.ocr.paddleocr import PaddleOCRBackend

    mock_ocr = Mock()

    with patch.object(paddleocr, "PaddleOCR", return_value=mock_ocr):
        backend = PaddleOCRBackend(lang="en", use_gpu=False)

        backend.initialize()
        first_ocr = backend._ocr

        # Second call should return early
        backend.initialize()
        second_ocr = backend._ocr

        assert first_ocr is second_ocr
        # PaddleOCR should only be constructed once
        paddleocr.PaddleOCR.assert_called_once()


def test_paddleocr_initialize_failure() -> None:
    """Test PaddleOCRBackend.initialize raises OCRError on failure."""
    paddleocr = pytest.importorskip("paddleocr", reason="PaddleOCR not installed")

    from kreuzberg.ocr.paddleocr import PaddleOCRBackend

    with patch.object(paddleocr, "PaddleOCR", side_effect=RuntimeError("Model load failed")):
        backend = PaddleOCRBackend(lang="en", use_gpu=False)

        with pytest.raises(OCRError) as exc_info:
            backend.initialize()

        assert "Failed to initialize PaddleOCR" in str(exc_info.value)


def test_paddleocr_process_image_reader_none_after_init() -> None:
    """Test process_image raises RuntimeError when _ocr fails to initialize."""
    pytest.importorskip("paddleocr", reason="PaddleOCR not installed")

    from kreuzberg.ocr.paddleocr import PaddleOCRBackend

    backend = PaddleOCRBackend(lang="en", use_gpu=False)

    # Mock initialize to succeed but leave _ocr as None
    with patch.object(backend, "initialize"):
        backend._ocr = None

        with pytest.raises(RuntimeError) as exc_info:
            backend.process_image(b"fake image data", "en")

        assert "PaddleOCR failed to initialize" in str(exc_info.value)


def test_paddleocr_process_file_reader_none_after_init() -> None:
    """Test process_file raises RuntimeError when _ocr fails to initialize."""
    pytest.importorskip("paddleocr", reason="PaddleOCR not installed")

    from kreuzberg.ocr.paddleocr import PaddleOCRBackend

    backend = PaddleOCRBackend(lang="en", use_gpu=False)

    # Mock initialize to succeed but leave _ocr as None
    with patch.object(backend, "initialize"):
        backend._ocr = None

        with pytest.raises(RuntimeError) as exc_info:
            backend.process_file("/fake/path.png", "en")

        assert "PaddleOCR failed to initialize" in str(exc_info.value)


def test_paddleocr_process_image_unsupported_language() -> None:
    """Test process_image raises ValidationError for unsupported language."""
    pytest.importorskip("paddleocr", reason="PaddleOCR not installed")

    from kreuzberg.ocr.paddleocr import PaddleOCRBackend

    backend = PaddleOCRBackend(lang="en", use_gpu=False)

    # Set _ocr to bypass initialization
    backend._ocr = Mock()

    with pytest.raises(ValidationError) as exc_info:
        backend.process_image(b"fake image data", "invalid_lang")

    assert "Language 'invalid_lang' not supported" in str(exc_info.value)


def test_paddleocr_process_paddleocr_result_empty() -> None:
    """Test _process_paddleocr_result with empty result."""
    from kreuzberg.ocr.paddleocr import PaddleOCRBackend

    content, confidence, text_regions = PaddleOCRBackend._process_paddleocr_result([])

    assert content == ""
    assert confidence == 0.0
    assert text_regions == 0


def test_paddleocr_process_paddleocr_result_none_page() -> None:
    """Test _process_paddleocr_result when result[0] is None."""
    from kreuzberg.ocr.paddleocr import PaddleOCRBackend

    content, confidence, text_regions = PaddleOCRBackend._process_paddleocr_result([None])

    assert content == ""
    assert confidence == 0.0
    assert text_regions == 0


def test_paddleocr_process_paddleocr_result_normal() -> None:
    """Test _process_paddleocr_result with normal PaddleOCR output."""
    from kreuzberg.ocr.paddleocr import PaddleOCRBackend

    # PaddleOCR format: [[[box_coords], (text, confidence)], ...]
    result = [
        [
            [[[0, 0], [100, 0], [100, 30], [0, 30]], ("Hello", 0.95)],
            [[[0, 40], [100, 40], [100, 70], [0, 70]], ("World", 0.90)],
        ]
    ]

    content, confidence, text_regions = PaddleOCRBackend._process_paddleocr_result(result)

    assert content == "Hello\nWorld"
    assert confidence == pytest.approx((0.95 + 0.90) / 2)
    assert text_regions == 2


def test_paddleocr_process_paddleocr_result_empty_text() -> None:
    """Test _process_paddleocr_result with empty text entries."""
    from kreuzberg.ocr.paddleocr import PaddleOCRBackend

    result = [
        [
            [[[0, 0], [100, 0], [100, 30], [0, 30]], ("Hello", 0.95)],
            [[[0, 40], [100, 40], [100, 70], [0, 70]], ("", 0.90)],  # Empty text
            [[[0, 80], [100, 80], [100, 110], [0, 110]], ("World", 0.88)],
        ]
    ]

    content, confidence, text_regions = PaddleOCRBackend._process_paddleocr_result(result)

    assert content == "Hello\nWorld"
    # Only non-empty text contributes to confidence
    assert confidence == pytest.approx((0.95 + 0.88) / 2)
    assert text_regions == 2


def test_paddleocr_process_paddleocr_result_invalid_structure() -> None:
    """Test _process_paddleocr_result with invalid line structure (not list/tuple)."""
    from kreuzberg.ocr.paddleocr import PaddleOCRBackend

    # Invalid structure: line is not a list/tuple
    result = [["invalid_structure"]]

    content, confidence, text_regions = PaddleOCRBackend._process_paddleocr_result(result)

    assert content == ""
    assert confidence == 0.0
    assert text_regions == 0


def test_paddleocr_process_paddleocr_result_invalid_text_info() -> None:
    """Test _process_paddleocr_result with invalid text_info structure."""
    from kreuzberg.ocr.paddleocr import PaddleOCRBackend

    # text_info is not a list/tuple with 2 elements
    result = [[["box"], "invalid_text_info"]]

    content, confidence, text_regions = PaddleOCRBackend._process_paddleocr_result(result)

    assert content == ""
    assert confidence == 0.0
    assert text_regions == 0


def test_paddleocr_is_cuda_available_no_paddle() -> None:
    """Test _is_cuda_available returns False when paddle not installed."""
    from kreuzberg.ocr.paddleocr import PaddleOCRBackend

    with patch.dict("sys.modules", {"paddle": None}):
        with patch("builtins.__import__", side_effect=ImportError("no paddle")):
            available = PaddleOCRBackend._is_cuda_available()

            assert available is False


def test_paddleocr_is_cuda_available_attribute_error() -> None:
    """Test _is_cuda_available handles AttributeError gracefully."""
    from kreuzberg.ocr.paddleocr import PaddleOCRBackend

    # Mock paddle module without is_compiled_with_cuda
    mock_paddle = Mock(spec=[])  # Empty spec, no attributes
    del mock_paddle.device  # Ensure device doesn't exist

    with patch.dict("sys.modules", {"paddle": mock_paddle}):
        available = PaddleOCRBackend._is_cuda_available()

        assert available is False


def test_paddleocr_use_gpu_none_auto_detects() -> None:
    """Test use_gpu=None auto-detects CUDA availability."""
    pytest.importorskip("paddleocr", reason="PaddleOCR not installed")

    from kreuzberg.ocr.paddleocr import PaddleOCRBackend

    with patch.object(PaddleOCRBackend, "_is_cuda_available", return_value=True):
        backend = PaddleOCRBackend(lang="en", use_gpu=None)

        assert backend.device == "gpu"


def test_paddleocr_use_gpu_explicit_true() -> None:
    """Test use_gpu=True explicitly sets device to gpu."""
    pytest.importorskip("paddleocr", reason="PaddleOCR not installed")

    from kreuzberg.ocr.paddleocr import PaddleOCRBackend

    backend = PaddleOCRBackend(lang="en", use_gpu=True)

    assert backend.device == "gpu"


def test_paddleocr_use_gpu_explicit_false() -> None:
    """Test use_gpu=False explicitly sets device to cpu."""
    pytest.importorskip("paddleocr", reason="PaddleOCR not installed")

    from kreuzberg.ocr.paddleocr import PaddleOCRBackend

    backend = PaddleOCRBackend(lang="en", use_gpu=False)

    assert backend.device == "cpu"


def test_paddleocr_shutdown() -> None:
    """Test shutdown clears _ocr."""
    pytest.importorskip("paddleocr", reason="PaddleOCR not installed")

    from kreuzberg.ocr.paddleocr import PaddleOCRBackend

    backend = PaddleOCRBackend(lang="en", use_gpu=False)
    backend._ocr = Mock()

    backend.shutdown()

    assert backend._ocr is None


def test_paddleocr_name() -> None:
    """Test name returns 'paddleocr'."""
    pytest.importorskip("paddleocr", reason="PaddleOCR not installed")

    from kreuzberg.ocr.paddleocr import PaddleOCRBackend

    backend = PaddleOCRBackend(lang="en", use_gpu=False)

    assert backend.name() == "paddleocr"


def test_paddleocr_supported_languages() -> None:
    """Test supported_languages returns sorted list."""
    pytest.importorskip("paddleocr", reason="PaddleOCR not installed")

    from kreuzberg.ocr.paddleocr import PaddleOCRBackend

    backend = PaddleOCRBackend(lang="en", use_gpu=False)
    supported = backend.supported_languages()

    assert isinstance(supported, list)
    assert len(supported) > 0
    assert supported == sorted(supported)
    assert "en" in supported
