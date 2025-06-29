"""
Tests for language detection functionality.
"""
from __future__ import annotations

from unittest.mock import Mock, patch
from typing import TYPE_CHECKING

import pytest

from kreuzberg._language_detection import detect_languages, is_language_detection_available
from kreuzberg.exceptions import MissingDependencyError

if TYPE_CHECKING:
    from pytest_mock import MockerFixture


class TestLanguageDetection:
    """Test language detection functionality."""

    def test_is_language_detection_available_with_fast_langdetect(self) -> None:
        """Test that language detection availability is correctly detected when fast-langdetect is available."""
        with patch('kreuzberg._language_detection.detect_langs') as mock_detect_langs:
            mock_detect_langs.return_value = [Mock(lang='en')]
            assert is_language_detection_available() is True

    def test_is_language_detection_available_without_fast_langdetect(self) -> None:
        """Test that language detection availability is correctly detected when fast-langdetect is not available."""
        with patch('kreuzberg._language_detection.detect_langs', None):
            assert is_language_detection_available() is False

    def test_detect_languages_success(self) -> None:
        """Test successful language detection."""
        mock_result = [Mock(lang='en'), Mock(lang='de'), Mock(lang='fr')]
        with patch('kreuzberg._language_detection.detect_langs') as mock_detect_langs:
            mock_detect_langs.return_value = mock_result
            result = detect_languages("This is English text with some German words.")
            assert result == ['en', 'de', 'fr']

    def test_detect_languages_with_top_n_limit(self) -> None:
        """Test language detection with top_n parameter."""
        mock_result = [Mock(lang='en'), Mock(lang='de'), Mock(lang='fr'), Mock(lang='es')]
        with patch('kreuzberg._language_detection.detect_langs') as mock_detect_langs:
            mock_detect_langs.return_value = mock_result
            result = detect_languages("This is English text with some German words.", top_n=2)
            assert result == ['en', 'de']

    def test_detect_languages_exception_handling(self) -> None:
        """Test that exceptions in language detection are handled gracefully."""
        with patch('kreuzberg._language_detection.detect_langs') as mock_detect_langs:
            mock_detect_langs.side_effect = Exception("Detection failed")
            result = detect_languages("Some text")
            assert result == []

    def test_detect_languages_missing_dependency(self) -> None:
        """Test that MissingDependencyError is raised when fast-langdetect is not available."""
        with patch('kreuzberg._language_detection.detect_langs', None):
            with pytest.raises(MissingDependencyError, match="fast-langdetect is required"):
                detect_languages("Some text")

    def test_detect_languages_caching(self) -> None:
        """Test that language detection results are cached."""
        mock_result = [Mock(lang='en')]
        with patch('kreuzberg._language_detection.detect_langs') as mock_detect_langs:
            mock_detect_langs.return_value = mock_result
            
            # First call
            result1 = detect_languages("This is English text.")
            # Second call with same text
            result2 = detect_languages("This is English text.")
            
            assert result1 == result2
            # Should only be called once due to caching
            assert mock_detect_langs.call_count == 1

    def test_detect_languages_different_texts_not_cached(self) -> None:
        """Test that different texts are not cached together."""
        mock_result1 = [Mock(lang='en')]
        mock_result2 = [Mock(lang='de')]
        with patch('kreuzberg._language_detection.detect_langs') as mock_detect_langs:
            mock_detect_langs.side_effect = [mock_result1, mock_result2]
            
            result1 = detect_languages("This is English text.")
            result2 = detect_languages("Das ist deutscher Text.")
            
            assert result1 == ['en']
            assert result2 == ['de']
            assert mock_detect_langs.call_count == 2


class TestLanguageDetectionIntegration:
    """Test language detection integration with extraction."""

    @pytest.mark.anyio
    async def test_extract_file_with_language_detection(self, tmp_path) -> None:
        """Test that language detection works with extract_file."""
        from kreuzberg import extract_file, ExtractionConfig
        
        # Create a test file with English text
        test_file = tmp_path / "test.txt"
        test_file.write_text("This is English text for testing language detection.")
        
        with patch('kreuzberg._language_detection.detect_langs') as mock_detect_langs:
            mock_detect_langs.return_value = [Mock(lang='en')]
            
            config = ExtractionConfig(auto_detect_language=True)
            result = await extract_file(test_file, config=config)
            
            assert result.detected_languages == ['en']

    @pytest.mark.anyio
    async def test_extract_file_without_language_detection(self, tmp_path) -> None:
        """Test that language detection is not performed when disabled."""
        from kreuzberg import extract_file, ExtractionConfig
        
        test_file = tmp_path / "test.txt"
        test_file.write_text("This is English text.")
        
        with patch('kreuzberg._language_detection.detect_langs') as mock_detect_langs:
            config = ExtractionConfig(auto_detect_language=False)
            result = await extract_file(test_file, config=config)
            
            assert result.detected_languages is None
            mock_detect_langs.assert_not_called()

    @pytest.mark.anyio
    async def test_extract_file_missing_dependency(self, tmp_path) -> None:
        """Test that MissingDependencyError is raised when language detection is enabled but library is missing."""
        from kreuzberg import extract_file, ExtractionConfig
        from kreuzberg.exceptions import MissingDependencyError
        
        test_file = tmp_path / "test.txt"
        test_file.write_text("This is English text.")
        
        with patch('kreuzberg._language_detection.is_language_detection_available', return_value=False):
            config = ExtractionConfig(auto_detect_language=True)
            
            with pytest.raises(MissingDependencyError, match="fast-langdetect is not installed"):
                await extract_file(test_file, config=config)

    def test_extract_file_sync_with_language_detection(self, tmp_path) -> None:
        """Test that language detection works with extract_file_sync."""
        from kreuzberg import extract_file_sync, ExtractionConfig
        
        test_file = tmp_path / "test.txt"
        test_file.write_text("This is English text for testing language detection.")
        
        with patch('kreuzberg._language_detection.detect_langs') as mock_detect_langs:
            mock_detect_langs.return_value = [Mock(lang='en')]
            
            config = ExtractionConfig(auto_detect_language=True)
            result = extract_file_sync(test_file, config=config)
            
            assert result.detected_languages == ['en']


class TestOCRBackendIntegration:
    """Test language detection integration with OCR backends."""

    @pytest.mark.anyio
    async def test_image_extractor_with_detected_languages(self, tmp_path) -> None:
        """Test that ImageExtractor uses detected languages for OCR."""
        from kreuzberg import ExtractionConfig
        from kreuzberg._extractors._image import ImageExtractor
        
        # Create a mock image file
        image_file = tmp_path / "test.png"
        image_file.write_bytes(b"fake image data")
        
        config = ExtractionConfig(
            ocr_backend="tesseract",
            auto_detect_language=True
        )
        # Simulate detected languages
        config.detected_languages = ['en', 'de']
        
        extractor = ImageExtractor(mime_type="image/png", config=config)
        
        with patch('kreuzberg._ocr.get_ocr_backend') as mock_get_backend:
            mock_backend = Mock()
            mock_backend.process_file.return_value = Mock(
                content="Extracted text",
                mime_type="text/plain",
                metadata={},
                chunks=[]
            )
            mock_get_backend.return_value = mock_backend
            
            await extractor.extract_path_async(image_file)
            
            # Verify that the backend was called with the correct language configuration
            mock_backend.process_file.assert_called_once()
            call_args = mock_backend.process_file.call_args[1]
            assert call_args['language'] == 'en+de'

    @pytest.mark.anyio
    async def test_pdf_extractor_with_detected_languages(self, tmp_path) -> None:
        """Test that PDFExtractor uses detected languages for OCR."""
        from kreuzberg import ExtractionConfig
        from kreuzberg._extractors._pdf import PDFExtractor
        
        # Create a mock PDF file
        pdf_file = tmp_path / "test.pdf"
        pdf_file.write_bytes(b"fake pdf data")
        
        config = ExtractionConfig(
            ocr_backend="tesseract",
            auto_detect_language=True,
            force_ocr=True  # Force OCR to test the OCR path
        )
        # Simulate detected languages
        config.detected_languages = ['en', 'fr']
        
        extractor = PDFExtractor(mime_type="application/pdf", config=config)
        
        with patch('kreuzberg._ocr.get_ocr_backend') as mock_get_backend:
            mock_backend = Mock()
            mock_backend.process_image.return_value = Mock(
                content="Extracted text",
                mime_type="text/plain",
                metadata={},
                chunks=[]
            )
            mock_get_backend.return_value = mock_backend
            
            with patch('kreuzberg._extractors._pdf.PDFExtractor._convert_pdf_to_images') as mock_convert:
                mock_convert.return_value = [Mock()]  # Mock image
                
                await extractor.extract_path_async(pdf_file)
                
                # Verify that the backend was called with the correct language configuration
                mock_backend.process_image.assert_called_once()
                call_args = mock_backend.process_image.call_args[1]
                assert call_args['language'] == 'en+fr' 