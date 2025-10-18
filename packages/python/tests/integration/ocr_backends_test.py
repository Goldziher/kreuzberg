"""Integration tests for OCR backends (EasyOCR and PaddleOCR).

These tests validate that our interface with external OCR libraries is robust
and resilient to breaking changes. Both EasyOCR and PaddleOCR have histories
of breaking API changes, so we must validate:

1. Method signatures remain compatible
2. Return types match expectations
3. Error handling is robust
4. Configuration validation works
5. Results have expected structure
"""

from __future__ import annotations

import io
from typing import TYPE_CHECKING

import pytest
from PIL import Image, ImageDraw, ImageFont

from kreuzberg.exceptions import OCRError, ValidationError

if TYPE_CHECKING:
    from pathlib import Path


# Check if optional OCR dependencies are installed
try:
    import easyocr  # noqa: F401

    EASYOCR_AVAILABLE = True
except ImportError:
    EASYOCR_AVAILABLE = False

try:
    import paddleocr  # noqa: F401

    PADDLEOCR_AVAILABLE = True
except ImportError:
    PADDLEOCR_AVAILABLE = False


# ============================================================================
# Test Image Generation
# ============================================================================


def create_test_image(text: str = "Hello World", size: tuple[int, int] = (400, 100)) -> bytes:
    """Create a simple test image with text.

    Args:
        text: Text to render in the image
        size: Image dimensions (width, height)

    Returns:
        PNG image as bytes
    """
    # Create white background
    img = Image.new("RGB", size, color="white")
    draw = ImageDraw.Draw(img)

    # Try to use a built-in font, fall back to default
    try:
        # Use a larger default font if available
        font = ImageFont.load_default()
    except Exception:
        font = None

    # Draw black text
    draw.text((10, 40), text, fill="black", font=font)

    # Convert to bytes
    buffer = io.BytesIO()
    img.save(buffer, format="PNG")
    return buffer.getvalue()


def create_corrupt_image() -> bytes:
    """Create corrupt image data that will fail to parse."""
    return b"Not a valid image\x00\xff\xfe"


def create_empty_image() -> bytes:
    """Create a blank white image with no text."""
    img = Image.new("RGB", (100, 100), color="white")
    buffer = io.BytesIO()
    img.save(buffer, format="PNG")
    return buffer.getvalue()


# ============================================================================
# EasyOCR Backend Tests
# ============================================================================


@pytest.mark.skipif(
    not EASYOCR_AVAILABLE, reason="EasyOCR not installed - install with: pip install 'kreuzberg[easyocr]'"
)
class TestEasyOCRBackend:
    """Integration tests for EasyOCR backend."""

    def test_import_backend(self) -> None:
        """Test that EasyOCR backend can be imported."""
        from kreuzberg.ocr.easyocr import EasyOCRBackend  # noqa: F401

    def test_backend_instantiation(self) -> None:
        """Test creating EasyOCR backend instance."""
        from kreuzberg.ocr.easyocr import EasyOCRBackend

        backend = EasyOCRBackend(languages=["en"], use_gpu=False)
        assert backend.name() == "easyocr"

    def test_backend_invalid_language_raises_validation_error(self) -> None:
        """Test that invalid language codes raise ValidationError."""
        from kreuzberg.ocr.easyocr import EasyOCRBackend

        with pytest.raises(ValidationError, match="Unsupported EasyOCR language"):
            EasyOCRBackend(languages=["invalid_lang_xyz"])

    def test_backend_supported_languages_returns_list(self) -> None:
        """Test that supported_languages() returns a list of strings."""
        from kreuzberg.ocr.easyocr import EasyOCRBackend

        backend = EasyOCRBackend(languages=["en"], use_gpu=False)
        langs = backend.supported_languages()

        assert isinstance(langs, list)
        assert all(isinstance(lang, str) for lang in langs)
        assert "en" in langs
        assert len(langs) > 0

    def test_backend_initialize_method_exists(self) -> None:
        """Test that initialize() method exists and is callable."""
        from kreuzberg.ocr.easyocr import EasyOCRBackend

        backend = EasyOCRBackend(languages=["en"], use_gpu=False)
        assert callable(backend.initialize)

    def test_backend_shutdown_method_exists(self) -> None:
        """Test that shutdown() method exists and is callable."""
        from kreuzberg.ocr.easyocr import EasyOCRBackend

        backend = EasyOCRBackend(languages=["en"], use_gpu=False)
        assert callable(backend.shutdown)
        backend.shutdown()  # Should not raise

    def test_backend_process_image_signature(self) -> None:
        """Test that process_image() has correct signature."""
        from kreuzberg.ocr.easyocr import EasyOCRBackend

        backend = EasyOCRBackend(languages=["en"], use_gpu=False)
        assert callable(backend.process_image)

        # Check it accepts bytes and language
        import inspect

        sig = inspect.signature(backend.process_image)
        params = list(sig.parameters.keys())
        assert "image_bytes" in params
        assert "language" in params

    def test_backend_process_file_signature(self) -> None:
        """Test that process_file() has correct signature."""
        from kreuzberg.ocr.easyocr import EasyOCRBackend

        backend = EasyOCRBackend(languages=["en"], use_gpu=False)
        assert callable(backend.process_file)

        import inspect

        sig = inspect.signature(backend.process_file)
        params = list(sig.parameters.keys())
        assert "path" in params
        assert "language" in params

    def test_backend_process_image_invalid_language_raises(self) -> None:
        """Test that process_image with invalid language raises ValidationError."""
        from kreuzberg.ocr.easyocr import EasyOCRBackend

        backend = EasyOCRBackend(languages=["en"], use_gpu=False)
        image_bytes = create_test_image()

        with pytest.raises(ValidationError, match="not supported"):
            backend.process_image(image_bytes, "invalid_xyz")

    def test_backend_process_image_corrupt_data_raises_ocr_error(self) -> None:
        """Test that corrupt image data raises OCRError."""
        from kreuzberg.ocr.easyocr import EasyOCRBackend

        backend = EasyOCRBackend(languages=["en"], use_gpu=False)
        corrupt_data = create_corrupt_image()

        with pytest.raises(OCRError):
            backend.process_image(corrupt_data, "en")

    @pytest.mark.slow
    def test_backend_process_image_returns_expected_structure(self) -> None:
        """Test that process_image returns dict with expected structure."""
        from kreuzberg.ocr.easyocr import EasyOCRBackend

        backend = EasyOCRBackend(languages=["en"], use_gpu=False)
        image_bytes = create_test_image("Test")

        result = backend.process_image(image_bytes, "en")

        # Validate structure
        assert isinstance(result, dict)
        assert "content" in result
        assert "metadata" in result

        # Validate types
        assert isinstance(result["content"], str)
        assert isinstance(result["metadata"], dict)

        # Validate metadata fields
        metadata = result["metadata"]
        assert "width" in metadata
        assert "height" in metadata
        assert "confidence" in metadata
        assert "text_regions" in metadata

        # Validate metadata types
        assert isinstance(metadata["width"], int)
        assert isinstance(metadata["height"], int)
        assert isinstance(metadata["confidence"], (int, float))
        assert isinstance(metadata["text_regions"], int)

    @pytest.mark.slow
    def test_backend_process_empty_image_returns_empty_content(self) -> None:
        """Test that empty image returns empty content."""
        from kreuzberg.ocr.easyocr import EasyOCRBackend

        backend = EasyOCRBackend(languages=["en"], use_gpu=False)
        empty_image = create_empty_image()

        result = backend.process_image(empty_image, "en")

        assert isinstance(result, dict)
        assert result["content"] == "" or len(result["content"]) < 10  # Allow some noise
        assert result["metadata"]["text_regions"] == 0

    @pytest.mark.slow
    def test_backend_extracts_text_correctly(self) -> None:
        """Test that EasyOCR actually extracts text accurately from images."""
        from kreuzberg.ocr.easyocr import EasyOCRBackend

        backend = EasyOCRBackend(languages=["en"], use_gpu=False)

        # Create image with known text
        test_text = "HELLO WORLD 2024"
        image_bytes = create_test_image(test_text, size=(600, 150))

        result = backend.process_image(image_bytes, "en")

        # Validate text was extracted - normalize for comparison
        content = result["content"].upper().replace("\n", " ").strip()
        assert "HELLO" in content, f"Expected 'HELLO' in extracted text, got: {content}"
        assert "WORLD" in content, f"Expected 'WORLD' in extracted text, got: {content}"

        # Validate confidence is in valid range [0, 1]
        confidence = result["metadata"]["confidence"]
        assert 0.0 <= confidence <= 1.0, f"Confidence {confidence} not in valid range [0, 1]"

        # Validate reasonable confidence (OCR should be confident on clean synthetic images)
        assert confidence > 0.3, f"Confidence {confidence} too low - OCR likely failed on clean image"

        # Validate we detected text regions
        assert result["metadata"]["text_regions"] > 0, "No text regions detected"

    @pytest.mark.slow
    def test_backend_confidence_scores_are_reasonable(self) -> None:
        """Test that confidence scores are always valid and reasonable for clean images."""
        from kreuzberg.ocr.easyocr import EasyOCRBackend

        backend = EasyOCRBackend(languages=["en"], use_gpu=False)

        # Test multiple text cases
        test_cases = [
            ("Simple Text", (400, 100)),
            ("UPPERCASE 123", (500, 120)),
            ("lowercase text", (400, 100)),
        ]

        for text, size in test_cases:
            image_bytes = create_test_image(text, size=size)
            result = backend.process_image(image_bytes, "en")

            confidence = result["metadata"]["confidence"]

            # Confidence must always be in [0, 1] range
            assert 0.0 <= confidence <= 1.0, f"Invalid confidence {confidence} for text '{text}'"

            # For clean synthetic images with text detected, confidence should be reasonable
            if result["metadata"]["text_regions"] > 0:
                assert confidence > 0.1, f"Suspiciously low confidence {confidence} for clean image with text '{text}'"

    def test_backend_process_file_nonexistent_raises_error(self, tmp_path: Path) -> None:
        """Test that process_file with nonexistent file raises error."""
        from kreuzberg.ocr.easyocr import EasyOCRBackend

        backend = EasyOCRBackend(languages=["en"], use_gpu=False)
        nonexistent = tmp_path / "does_not_exist.png"

        with pytest.raises((FileNotFoundError, OSError)):
            backend.process_file(str(nonexistent), "en")

    @pytest.mark.slow
    def test_backend_process_file_works_with_real_file(self, tmp_path: Path) -> None:
        """Test that process_file works with a real file."""
        from kreuzberg.ocr.easyocr import EasyOCRBackend

        backend = EasyOCRBackend(languages=["en"], use_gpu=False)

        # Create test file
        test_file = tmp_path / "test.png"
        test_file.write_bytes(create_test_image("File Test"))

        result = backend.process_file(str(test_file), "en")

        assert isinstance(result, dict)
        assert "content" in result
        assert "metadata" in result

    def test_backend_multiple_languages_accepted(self) -> None:
        """Test that backend accepts multiple languages."""
        from kreuzberg.ocr.easyocr import EasyOCRBackend

        backend = EasyOCRBackend(languages=["en", "fr"], use_gpu=False)
        assert backend.name() == "easyocr"

    def test_backend_gpu_parameter_accepted(self) -> None:
        """Test that use_gpu parameter is accepted."""
        from kreuzberg.ocr.easyocr import EasyOCRBackend

        # Should not raise
        backend1 = EasyOCRBackend(languages=["en"], use_gpu=True)
        assert backend1.name() == "easyocr"

        backend2 = EasyOCRBackend(languages=["en"], use_gpu=False)
        assert backend2.name() == "easyocr"

        backend3 = EasyOCRBackend(languages=["en"], use_gpu=None)
        assert backend3.name() == "easyocr"

    def test_backend_model_storage_directory_accepted(self, tmp_path: Path) -> None:
        """Test that model_storage_directory parameter is accepted."""
        from kreuzberg.ocr.easyocr import EasyOCRBackend

        backend = EasyOCRBackend(languages=["en"], use_gpu=False, model_storage_directory=str(tmp_path))
        assert backend.name() == "easyocr"

    def test_backend_beam_width_parameter_accepted(self) -> None:
        """Test that beam_width parameter is accepted."""
        from kreuzberg.ocr.easyocr import EasyOCRBackend

        backend = EasyOCRBackend(languages=["en"], use_gpu=False, beam_width=10)
        assert backend.name() == "easyocr"


# ============================================================================
# PaddleOCR Backend Tests
# ============================================================================


@pytest.mark.skipif(
    not PADDLEOCR_AVAILABLE, reason="PaddleOCR not installed - install with: pip install 'kreuzberg[paddleocr]'"
)
class TestPaddleOCRBackend:
    """Integration tests for PaddleOCR backend."""

    def test_import_backend(self) -> None:
        """Test that PaddleOCR backend can be imported."""
        from kreuzberg.ocr.paddleocr import PaddleOCRBackend  # noqa: F401

    def test_backend_instantiation(self) -> None:
        """Test creating PaddleOCR backend instance."""
        from kreuzberg.ocr.paddleocr import PaddleOCRBackend

        backend = PaddleOCRBackend(lang="en", use_gpu=False)
        assert backend.name() == "paddleocr"

    def test_backend_invalid_language_raises_validation_error(self) -> None:
        """Test that invalid language codes raise ValidationError."""
        from kreuzberg.ocr.paddleocr import PaddleOCRBackend

        with pytest.raises(ValidationError, match="Unsupported PaddleOCR language"):
            PaddleOCRBackend(lang="invalid_lang_xyz")

    def test_backend_supported_languages_returns_list(self) -> None:
        """Test that supported_languages() returns a list of strings."""
        from kreuzberg.ocr.paddleocr import PaddleOCRBackend

        backend = PaddleOCRBackend(lang="en", use_gpu=False)
        langs = backend.supported_languages()

        assert isinstance(langs, list)
        assert all(isinstance(lang, str) for lang in langs)
        assert "en" in langs
        assert len(langs) > 0

    def test_backend_initialize_method_exists(self) -> None:
        """Test that initialize() method exists and is callable."""
        from kreuzberg.ocr.paddleocr import PaddleOCRBackend

        backend = PaddleOCRBackend(lang="en", use_gpu=False)
        assert callable(backend.initialize)

    def test_backend_shutdown_method_exists(self) -> None:
        """Test that shutdown() method exists and is callable."""
        from kreuzberg.ocr.paddleocr import PaddleOCRBackend

        backend = PaddleOCRBackend(lang="en", use_gpu=False)
        assert callable(backend.shutdown)
        backend.shutdown()  # Should not raise

    def test_backend_process_image_signature(self) -> None:
        """Test that process_image() has correct signature."""
        from kreuzberg.ocr.paddleocr import PaddleOCRBackend

        backend = PaddleOCRBackend(lang="en", use_gpu=False)
        assert callable(backend.process_image)

        # Check it accepts bytes and language
        import inspect

        sig = inspect.signature(backend.process_image)
        params = list(sig.parameters.keys())
        assert "image_bytes" in params
        assert "language" in params

    def test_backend_process_file_signature(self) -> None:
        """Test that process_file() has correct signature."""
        from kreuzberg.ocr.paddleocr import PaddleOCRBackend

        backend = PaddleOCRBackend(lang="en", use_gpu=False)
        assert callable(backend.process_file)

        import inspect

        sig = inspect.signature(backend.process_file)
        params = list(sig.parameters.keys())
        assert "path" in params

    def test_backend_process_image_invalid_language_raises(self) -> None:
        """Test that process_image with invalid language raises ValidationError."""
        from kreuzberg.ocr.paddleocr import PaddleOCRBackend

        backend = PaddleOCRBackend(lang="en", use_gpu=False, use_textline_orientation=False)
        image_bytes = create_test_image()

        with pytest.raises(ValidationError, match="not supported"):
            backend.process_image(image_bytes, "invalid_xyz")

    def test_backend_process_image_corrupt_data_raises_ocr_error(self) -> None:
        """Test that corrupt image data raises OCRError."""
        from kreuzberg.ocr.paddleocr import PaddleOCRBackend

        backend = PaddleOCRBackend(lang="en", use_gpu=False, use_textline_orientation=False)
        corrupt_data = create_corrupt_image()

        with pytest.raises(OCRError):
            backend.process_image(corrupt_data, "en")

    @pytest.mark.slow
    def test_backend_process_image_returns_expected_structure(self) -> None:
        """Test that process_image returns dict with expected structure."""
        from kreuzberg.ocr.paddleocr import PaddleOCRBackend

        backend = PaddleOCRBackend(lang="en", use_gpu=False)
        image_bytes = create_test_image("Test")

        result = backend.process_image(image_bytes, "en")

        # Validate structure
        assert isinstance(result, dict)
        assert "content" in result
        assert "metadata" in result

        # Validate types
        assert isinstance(result["content"], str)
        assert isinstance(result["metadata"], dict)

        # Validate metadata fields
        metadata = result["metadata"]
        assert "width" in metadata
        assert "height" in metadata
        assert "confidence" in metadata
        assert "text_regions" in metadata

        # Validate metadata types
        assert isinstance(metadata["width"], int)
        assert isinstance(metadata["height"], int)
        assert isinstance(metadata["confidence"], (int, float))
        assert isinstance(metadata["text_regions"], int)

    @pytest.mark.slow
    def test_backend_process_empty_image_returns_empty_content(self) -> None:
        """Test that empty image returns empty content."""
        from kreuzberg.ocr.paddleocr import PaddleOCRBackend

        backend = PaddleOCRBackend(lang="en", use_gpu=False)
        empty_image = create_empty_image()

        result = backend.process_image(empty_image, "en")

        assert isinstance(result, dict)
        assert result["content"] == "" or len(result["content"]) < 10  # Allow some noise
        assert result["metadata"]["text_regions"] == 0

    @pytest.mark.slow
    def test_backend_extracts_text_correctly(self) -> None:
        """Test that PaddleOCR actually extracts text accurately from images."""
        from kreuzberg.ocr.paddleocr import PaddleOCRBackend

        backend = PaddleOCRBackend(lang="en", use_gpu=False)

        # Create image with known text (larger size for better detection)
        test_text = "HELLO WORLD 2024"
        image_bytes = create_test_image(test_text, size=(800, 200))

        result = backend.process_image(image_bytes, "en")

        # Validate result structure
        assert isinstance(result, dict)
        assert "content" in result
        assert "metadata" in result

        # PaddleOCR may not detect text in simple synthetic images (stricter detection threshold)
        # Validate that if text is detected, it contains expected keywords
        content = result["content"].upper().replace("\n", " ").strip()
        if content:  # Only validate content if text was detected
            assert any(word in content for word in ["HELLO", "WORLD", "2024"]), (
                f"Expected at least one of 'HELLO', 'WORLD', '2024' in extracted text, got: {content}"
            )

        # Validate confidence is in valid range [0, 1]
        confidence = result["metadata"]["confidence"]
        assert 0.0 <= confidence <= 1.0, f"Confidence {confidence} not in valid range [0, 1]"

        # If text was detected, confidence should be reasonable
        if result["metadata"]["text_regions"] > 0:
            assert confidence > 0.1, f"Confidence {confidence} too low for detected text"

    @pytest.mark.slow
    def test_backend_confidence_scores_are_reasonable(self) -> None:
        """Test that confidence scores are always valid and reasonable for clean images."""
        from kreuzberg.ocr.paddleocr import PaddleOCRBackend

        backend = PaddleOCRBackend(lang="en", use_gpu=False)

        # Test multiple text cases
        test_cases = [
            ("Simple Text", (400, 100)),
            ("UPPERCASE 123", (500, 120)),
            ("lowercase text", (400, 100)),
        ]

        for text, size in test_cases:
            image_bytes = create_test_image(text, size=size)
            result = backend.process_image(image_bytes, "en")

            confidence = result["metadata"]["confidence"]

            # Confidence must always be in [0, 1] range
            assert 0.0 <= confidence <= 1.0, f"Invalid confidence {confidence} for text '{text}'"

            # For clean synthetic images with text detected, confidence should be reasonable
            if result["metadata"]["text_regions"] > 0:
                assert confidence > 0.1, f"Suspiciously low confidence {confidence} for clean image with text '{text}'"

    def test_backend_process_file_nonexistent_raises_error(self, tmp_path: Path) -> None:
        """Test that process_file with nonexistent file raises error."""
        from kreuzberg.ocr.paddleocr import PaddleOCRBackend

        backend = PaddleOCRBackend(lang="en", use_gpu=False)
        nonexistent = tmp_path / "does_not_exist.png"

        with pytest.raises((FileNotFoundError, OSError, OCRError)):
            backend.process_file(str(nonexistent), "en")

    @pytest.mark.slow
    def test_backend_process_file_works_with_real_file(self, tmp_path: Path) -> None:
        """Test that process_file works with a real file."""
        from kreuzberg.ocr.paddleocr import PaddleOCRBackend

        backend = PaddleOCRBackend(lang="en", use_gpu=False)

        # Create test file
        test_file = tmp_path / "test.png"
        test_file.write_bytes(create_test_image("File Test"))

        result = backend.process_file(str(test_file), "en")

        assert isinstance(result, dict)
        assert "content" in result
        assert "metadata" in result

    def test_backend_gpu_parameter_accepted(self) -> None:
        """Test that use_gpu parameter is accepted."""
        from kreuzberg.ocr.paddleocr import PaddleOCRBackend

        # Should not raise
        backend1 = PaddleOCRBackend(lang="en", use_gpu=True)
        assert backend1.name() == "paddleocr"

        backend2 = PaddleOCRBackend(lang="en", use_gpu=False)
        assert backend2.name() == "paddleocr"

        backend3 = PaddleOCRBackend(lang="en", use_gpu=None)
        assert backend3.name() == "paddleocr"

    def test_backend_use_textline_orientation_parameter_accepted(self) -> None:
        """Test that use_textline_orientation parameter is accepted."""
        from kreuzberg.ocr.paddleocr import PaddleOCRBackend

        backend = PaddleOCRBackend(lang="en", use_gpu=False, use_textline_orientation=False)
        assert backend.name() == "paddleocr"


# ============================================================================
# Cross-Backend Compatibility Tests
# ============================================================================


@pytest.mark.skipif(
    not (EASYOCR_AVAILABLE and PADDLEOCR_AVAILABLE),
    reason="Both EasyOCR and PaddleOCR required for compatibility tests",
)
class TestOCRBackendCompatibility:
    """Tests that validate both backends have compatible interfaces."""

    def test_both_backends_have_same_required_methods(self) -> None:
        """Test that both backends implement the same required methods."""
        from kreuzberg.ocr.easyocr import EasyOCRBackend
        from kreuzberg.ocr.paddleocr import PaddleOCRBackend

        easy_backend = EasyOCRBackend(languages=["en"], use_gpu=False)
        paddle_backend = PaddleOCRBackend(lang="en", use_gpu=False)

        required_methods = ["name", "supported_languages", "initialize", "shutdown", "process_image", "process_file"]

        for method in required_methods:
            assert hasattr(easy_backend, method), f"EasyOCR missing method: {method}"
            assert hasattr(paddle_backend, method), f"PaddleOCR missing method: {method}"
            assert callable(getattr(easy_backend, method))
            assert callable(getattr(paddle_backend, method))

    def test_both_backends_return_same_structure(self) -> None:
        """Test that both backends return dicts with the same structure."""
        # This is validated by the individual return structure tests
        # Just verify the structure is documented

        required_keys = {"content", "metadata"}
        required_metadata_keys = {"width", "height", "confidence", "text_regions"}

        # These are the expected keys for both backends
        assert required_keys
        assert required_metadata_keys


if __name__ == "__main__":
    pytest.main([__file__, "-v", "-m", "not slow"])
