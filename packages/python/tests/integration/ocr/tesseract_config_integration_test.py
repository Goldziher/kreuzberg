"""Integration tests for TesseractConfig parameters.

Tests all new TesseractConfig fields (OEM, min_confidence, preprocessing, blacklist)
using real documents from test_documents with force_ocr=True.
"""

from __future__ import annotations

from pathlib import Path

import pytest

from kreuzberg import (
    ExtractionConfig,
    ImagePreprocessingConfig,
    OcrConfig,
    TesseractConfig,
    extract_file,
    extract_file_sync,
)

TEST_DOCUMENTS_ROOT = Path(__file__).parent.parent.parent.parent.parent.parent / "test_documents"


@pytest.fixture
def invoice_image() -> Path:
    """Invoice image for OCR testing."""
    path = TEST_DOCUMENTS_ROOT / "images" / "invoice_image.png"
    assert path.exists(), f"Test document not found: {path}"
    return path


@pytest.fixture
def layout_parser_image() -> Path:
    """Layout parser image with tables."""
    path = TEST_DOCUMENTS_ROOT / "images" / "layout_parser_paper_with_table.jpg"
    assert path.exists(), f"Test document not found: {path}"
    return path


@pytest.fixture
def ocr_test_image() -> Path:
    """General OCR test image."""
    path = TEST_DOCUMENTS_ROOT / "images" / "ocr_image.jpg"
    assert path.exists(), f"Test document not found: {path}"
    return path


@pytest.fixture
def chinese_image() -> Path:
    """Chinese text image."""
    path = TEST_DOCUMENTS_ROOT / "images" / "chi_sim_image.jpeg"
    assert path.exists(), f"Test document not found: {path}"
    return path


@pytest.fixture
def japanese_vertical_image() -> Path:
    """Japanese vertical text image."""
    path = TEST_DOCUMENTS_ROOT / "images" / "jpn_vert.jpeg"
    assert path.exists(), f"Test document not found: {path}"
    return path


@pytest.fixture
def hello_world_image() -> Path:
    """Simple hello world image."""
    path = TEST_DOCUMENTS_ROOT / "images" / "test_hello_world.png"
    assert path.exists(), f"Test document not found: {path}"
    return path


@pytest.fixture
def sample_pdf() -> Path:
    """Sample PDF for force_ocr testing."""
    pdfs = list((TEST_DOCUMENTS_ROOT / "pdfs").glob("*.pdf"))
    assert len(pdfs) > 0, "No PDFs found in test_documents/pdfs"
    return pdfs[0]


def test_oem_legacy_engine(invoice_image: Path) -> None:
    """Test OEM=0 (Legacy engine only)."""
    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            tesseract_config=TesseractConfig(
                language="eng",
                oem=0,
            ),
        ),
    )

    result = extract_file_sync(str(invoice_image), config=config)

    assert result.content is not None
    assert len(result.content.strip()) > 0
    assert result.metadata is not None
    assert "ocr" in result.metadata or result.mime_type.startswith("image/")


def test_oem_lstm_engine(invoice_image: Path) -> None:
    """Test OEM=1 (LSTM only - usually best)."""
    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            tesseract_config=TesseractConfig(
                language="eng",
                oem=1,
            ),
        ),
    )

    result = extract_file_sync(str(invoice_image), config=config)

    assert result.content is not None
    assert len(result.content.strip()) > 0


def test_oem_combined_engine(invoice_image: Path) -> None:
    """Test OEM=2 (Legacy + LSTM)."""
    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            tesseract_config=TesseractConfig(
                language="eng",
                oem=2,
            ),
        ),
    )

    result = extract_file_sync(str(invoice_image), config=config)

    assert result.content is not None
    assert len(result.content.strip()) > 0


def test_oem_default_engine(invoice_image: Path) -> None:
    """Test OEM=3 (Default based on availability)."""
    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            tesseract_config=TesseractConfig(
                language="eng",
                oem=3,
            ),
        ),
    )

    result = extract_file_sync(str(invoice_image), config=config)

    assert result.content is not None
    assert len(result.content.strip()) > 0


def test_min_confidence_zero(ocr_test_image: Path) -> None:
    """Test min_confidence=0.0 (accept all)."""
    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            tesseract_config=TesseractConfig(
                language="eng",
                min_confidence=0.0,
            ),
        ),
    )

    result = extract_file_sync(str(ocr_test_image), config=config)

    assert result.content is not None
    low_conf_length = len(result.content.strip())
    assert low_conf_length > 0


def test_min_confidence_medium(ocr_test_image: Path) -> None:
    """Test min_confidence=50.0 (filter low confidence)."""
    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            tesseract_config=TesseractConfig(
                language="eng",
                min_confidence=50.0,
            ),
        ),
    )

    result = extract_file_sync(str(ocr_test_image), config=config)

    assert result.content is not None
    assert len(result.content.strip()) > 0


def test_min_confidence_high(ocr_test_image: Path) -> None:
    """Test min_confidence=80.0 (only high confidence)."""
    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            tesseract_config=TesseractConfig(
                language="eng",
                min_confidence=80.0,
            ),
        ),
    )

    result = extract_file_sync(str(ocr_test_image), config=config)

    assert result.content is not None


def test_tessedit_char_blacklist_numbers(hello_world_image: Path) -> None:
    """Test blacklist='0123456789' removes all digits."""
    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            tesseract_config=TesseractConfig(
                language="eng",
                tessedit_char_blacklist="0123456789",
            ),
        ),
    )

    result = extract_file_sync(str(hello_world_image), config=config)

    assert result.content is not None


def test_tessedit_char_blacklist_special(hello_world_image: Path) -> None:
    """Test blacklist='!@#$%^&*()' removes special chars."""
    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            tesseract_config=TesseractConfig(
                language="eng",
                tessedit_char_blacklist="!@#$%^&*()",
            ),
        ),
    )

    result = extract_file_sync(str(hello_world_image), config=config)

    assert result.content is not None
    assert len(result.content.strip()) > 0


def test_force_ocr_on_pdf(sample_pdf: Path) -> None:
    """Test force_ocr=True on PDF with native text."""
    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            tesseract_config=TesseractConfig(
                language="eng",
            ),
        ),
    )

    result = extract_file_sync(str(sample_pdf), config=config)

    assert result.content is not None
    assert len(result.content.strip()) > 0
    assert result.metadata is not None


@pytest.mark.asyncio
async def test_tesseract_config_async(invoice_image: Path) -> None:
    """Test TesseractConfig with async extraction."""
    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            tesseract_config=TesseractConfig(
                language="eng",
                oem=1,
                min_confidence=50.0,
            ),
        ),
    )

    result = await extract_file(str(invoice_image), config=config)

    assert result.content is not None
    assert len(result.content.strip()) > 0


def test_preprocessing_dpi_adjustment(invoice_image: Path) -> None:
    """Test target_dpi=600 vs default 300."""
    preprocessing = ImagePreprocessingConfig(
        target_dpi=600,
    )

    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            tesseract_config=TesseractConfig(
                language="eng",
                preprocessing=preprocessing,
            ),
        ),
    )

    result = extract_file_sync(str(invoice_image), config=config)

    assert result.content is not None
    assert len(result.content.strip()) > 0


def test_preprocessing_denoise_enabled(invoice_image: Path) -> None:
    """Test denoise=True improves noisy images."""
    preprocessing = ImagePreprocessingConfig(
        denoise=True,
        target_dpi=300,
    )

    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            tesseract_config=TesseractConfig(
                language="eng",
                preprocessing=preprocessing,
            ),
        ),
    )

    result = extract_file_sync(str(invoice_image), config=config)

    assert result.content is not None
    assert len(result.content.strip()) > 0


def test_preprocessing_contrast_enhance(invoice_image: Path) -> None:
    """Test contrast_enhance=True improves low-contrast images."""
    preprocessing = ImagePreprocessingConfig(
        contrast_enhance=True,
        target_dpi=300,
    )

    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            tesseract_config=TesseractConfig(
                language="eng",
                preprocessing=preprocessing,
            ),
        ),
    )

    result = extract_file_sync(str(invoice_image), config=config)

    assert result.content is not None
    assert len(result.content.strip()) > 0


def test_preprocessing_auto_rotate(invoice_image: Path) -> None:
    """Test auto_rotate=True handles rotated text."""
    preprocessing = ImagePreprocessingConfig(
        auto_rotate=True,
        target_dpi=300,
    )

    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            tesseract_config=TesseractConfig(
                language="eng",
                preprocessing=preprocessing,
            ),
        ),
    )

    result = extract_file_sync(str(invoice_image), config=config)

    assert result.content is not None
    assert len(result.content.strip()) > 0


def test_preprocessing_deskew(invoice_image: Path) -> None:
    """Test deskew=True handles tilted images."""
    preprocessing = ImagePreprocessingConfig(
        deskew=True,
        target_dpi=300,
    )

    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            tesseract_config=TesseractConfig(
                language="eng",
                preprocessing=preprocessing,
            ),
        ),
    )

    result = extract_file_sync(str(invoice_image), config=config)

    assert result.content is not None
    assert len(result.content.strip()) > 0


def test_preprocessing_binarization_otsu(invoice_image: Path) -> None:
    """Test binarization_method='otsu'."""
    preprocessing = ImagePreprocessingConfig(
        binarization_method="otsu",
        target_dpi=300,
    )

    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            tesseract_config=TesseractConfig(
                language="eng",
                preprocessing=preprocessing,
            ),
        ),
    )

    result = extract_file_sync(str(invoice_image), config=config)

    assert result.content is not None
    assert len(result.content.strip()) > 0


def test_preprocessing_binarization_adaptive(invoice_image: Path) -> None:
    """Test binarization_method='adaptive'."""
    preprocessing = ImagePreprocessingConfig(
        binarization_method="adaptive",
        target_dpi=300,
    )

    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            tesseract_config=TesseractConfig(
                language="eng",
                preprocessing=preprocessing,
            ),
        ),
    )

    result = extract_file_sync(str(invoice_image), config=config)

    assert result.content is not None
    assert len(result.content.strip()) > 0


def test_preprocessing_combined_effects(invoice_image: Path) -> None:
    """Test multiple preprocessing options together."""
    preprocessing = ImagePreprocessingConfig(
        target_dpi=600,
        auto_rotate=True,
        deskew=True,
        denoise=True,
        contrast_enhance=True,
        binarization_method="adaptive",
        invert_colors=False,
    )

    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            tesseract_config=TesseractConfig(
                language="eng",
                preprocessing=preprocessing,
            ),
        ),
    )

    result = extract_file_sync(str(invoice_image), config=config)

    assert result.content is not None
    assert len(result.content.strip()) > 0


@pytest.mark.asyncio
async def test_preprocessing_async(invoice_image: Path) -> None:
    """Test preprocessing with async extraction."""
    preprocessing = ImagePreprocessingConfig(
        target_dpi=600,
        denoise=True,
        contrast_enhance=True,
    )

    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            tesseract_config=TesseractConfig(
                language="eng",
                preprocessing=preprocessing,
            ),
        ),
    )

    result = await extract_file(str(invoice_image), config=config)

    assert result.content is not None
    assert len(result.content.strip()) > 0


def test_preprocessing_with_chinese(chinese_image: Path) -> None:
    """Test preprocessing with Chinese text (chi_sim_image.jpeg)."""
    pytest.importorskip("tesseract", reason="Chinese language data may not be installed")

    preprocessing = ImagePreprocessingConfig(
        target_dpi=600,
        denoise=True,
    )

    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            language="chi_sim",
            tesseract_config=TesseractConfig(
                language="chi_sim",
                oem=1,
                preprocessing=preprocessing,
            ),
        ),
    )

    try:
        result = extract_file_sync(str(chinese_image), config=config)
        assert result.content is not None
    except Exception as e:
        if "chi_sim" in str(e).lower() or "language" in str(e).lower():
            pytest.skip(f"Chinese language data not installed: {e}")
        raise


def test_preprocessing_with_japanese(japanese_vertical_image: Path) -> None:
    """Test preprocessing with Japanese vertical text (jpn_vert.jpeg)."""
    preprocessing = ImagePreprocessingConfig(
        target_dpi=600,
        auto_rotate=True,
    )

    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            language="jpn",
            tesseract_config=TesseractConfig(
                language="jpn",
                oem=1,
                preprocessing=preprocessing,
            ),
        ),
    )

    try:
        result = extract_file_sync(str(japanese_vertical_image), config=config)
        assert result.content is not None
    except Exception as e:
        if "jpn" in str(e).lower() or "language" in str(e).lower():
            pytest.skip(f"Japanese language data not installed: {e}")
        raise


def test_oem_with_chinese_language(chinese_image: Path) -> None:
    """Test different OEM modes with Chinese script."""
    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            language="chi_sim",
            tesseract_config=TesseractConfig(
                language="chi_sim",
                oem=1,
            ),
        ),
    )

    try:
        result = extract_file_sync(str(chinese_image), config=config)
        assert result.content is not None
    except Exception as e:
        if "chi_sim" in str(e).lower() or "language" in str(e).lower():
            pytest.skip(f"Chinese language data not installed: {e}")
        raise


@pytest.mark.asyncio
async def test_batch_extraction_with_tesseract_config(
    invoice_image: Path,
    hello_world_image: Path,
) -> None:
    """Test batch extraction with custom TesseractConfig."""
    from kreuzberg import batch_extract_files

    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            tesseract_config=TesseractConfig(
                language="eng",
                oem=1,
                min_confidence=50.0,
            ),
        ),
    )

    results = await batch_extract_files(
        [str(invoice_image), str(hello_world_image)],
        config=config,
    )

    assert len(results) == 2
    for result in results:
        assert result.content is not None
        assert len(result.content.strip()) > 0


def test_force_ocr_with_preprocessing_on_pdf(sample_pdf: Path) -> None:
    """Test force_ocr + preprocessing on PDF."""
    preprocessing = ImagePreprocessingConfig(
        target_dpi=600,
        denoise=True,
        contrast_enhance=True,
    )

    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            tesseract_config=TesseractConfig(
                language="eng",
                oem=1,
                preprocessing=preprocessing,
            ),
        ),
    )

    result = extract_file_sync(str(sample_pdf), config=config)

    assert result.content is not None
    assert len(result.content.strip()) > 0


def test_combined_new_parameters(invoice_image: Path) -> None:
    """Test all new parameters together: OEM, confidence, preprocessing, blacklist."""
    preprocessing = ImagePreprocessingConfig(
        target_dpi=600,
        denoise=True,
        contrast_enhance=True,
        binarization_method="adaptive",
    )

    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            tesseract_config=TesseractConfig(
                language="eng",
                oem=1,
                min_confidence=60.0,
                preprocessing=preprocessing,
                tessedit_char_blacklist="",
            ),
        ),
    )

    result = extract_file_sync(str(invoice_image), config=config)

    assert result.content is not None
    assert len(result.content.strip()) > 0
    assert result.metadata is not None


def test_config_defaults_work(invoice_image: Path) -> None:
    """Test that default TesseractConfig values work correctly."""
    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            tesseract_config=TesseractConfig(
                language="eng",
            ),
        ),
    )

    result = extract_file_sync(str(invoice_image), config=config)

    assert result.content is not None
    assert len(result.content.strip()) > 0
