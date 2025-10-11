from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING, Any

import pytest
from PIL import Image, ImageDraw, ImageFont

from kreuzberg import PSMMode, TesseractConfig
from kreuzberg._ocr._tesseract import TesseractBackend
from kreuzberg._types import ExtractionResult
from kreuzberg.exceptions import OCRError, ValidationError

if TYPE_CHECKING:
    from PIL.Image import Image as PILImage


def create_test_image(text: str, width: int = 400, height: int = 100) -> PILImage:
    img = Image.new("RGB", (width, height), "white")
    draw = ImageDraw.Draw(img)

    try:
        font: Any = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 24)
    except (OSError, AttributeError):
        font = ImageFont.load_default()

    draw.text((20, height // 3), text, fill="black", font=font)
    return img


@pytest.fixture(scope="session")
def backend() -> TesseractBackend:
    return TesseractBackend()


@pytest.mark.parametrize(
    "output_format,expected_mime_type",
    [
        ("text", "text/plain"),
        ("markdown", "text/markdown"),
        ("hocr", "text/html"),
        ("tsv", "text/plain"),
    ],
)
@pytest.mark.anyio
async def test_output_format_returns_expected_mime_type(
    backend: TesseractBackend, output_format: str, expected_mime_type: str
) -> None:
    image = create_test_image("Sample Text")

    result = await backend.process_image(image, output_format=output_format)

    assert isinstance(result, ExtractionResult)
    assert result.mime_type == expected_mime_type
    assert len(result.content) > 0


def test_output_format_sync_text(backend: TesseractBackend) -> None:
    image = create_test_image("Hello World")

    result = backend.process_image_sync(image, output_format="text")

    assert isinstance(result, ExtractionResult)
    assert result.mime_type == "text/plain"
    assert "Hello" in result.content or "World" in result.content


def test_output_format_sync_markdown(backend: TesseractBackend) -> None:
    image = create_test_image("Markdown Test")

    result = backend.process_image_sync(image, output_format="markdown")

    assert isinstance(result, ExtractionResult)
    assert result.mime_type == "text/markdown"
    assert len(result.content) > 0


@pytest.mark.parametrize(
    "psm_mode",
    [
        PSMMode.AUTO,
        PSMMode.SINGLE_BLOCK,
        PSMMode.SINGLE_LINE,
        PSMMode.SINGLE_WORD,
    ],
)
@pytest.mark.anyio
async def test_psm_mode_produces_output(backend: TesseractBackend, psm_mode: PSMMode) -> None:
    image = create_test_image("PSM Test Text")

    result = await backend.process_image(image, psm=psm_mode)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


@pytest.mark.parametrize(
    "language_code",
    [
        "eng",
        "deu",
        "fra",
        "spa",
        "chi_sim",
    ],
)
@pytest.mark.anyio
async def test_language_code_produces_output(backend: TesseractBackend, language_code: str) -> None:
    image = create_test_image("Language Test")

    result = await backend.process_image(image, language=language_code)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


@pytest.mark.anyio
async def test_multi_language_code_produces_output(backend: TesseractBackend) -> None:
    image = create_test_image("Multi Language Test")

    result = await backend.process_image(image, language="eng+deu")

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


@pytest.mark.anyio
async def test_invalid_language_code_raises_error(backend: TesseractBackend) -> None:
    image = create_test_image("Invalid Language")

    with pytest.raises(ValidationError, match="not supported by Tesseract"):
        await backend.process_image(image, language="invalid_lang_code")


@pytest.mark.parametrize(
    "image_mode",
    ["RGB", "RGBA", "L", "LA", "P", "1"],
)
@pytest.mark.anyio
async def test_image_mode_produces_output(backend: TesseractBackend, image_mode: str) -> None:
    img = Image.new(image_mode, (400, 100), "white")
    draw = ImageDraw.Draw(img)
    draw.text((20, 30), "Mode Test", fill="black")

    result = await backend.process_image(img)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


@pytest.mark.anyio
async def test_cmyk_image_mode_converts_and_processes(backend: TesseractBackend) -> None:
    img = Image.new("RGB", (400, 100), "white")
    draw = ImageDraw.Draw(img)
    draw.text((20, 30), "CMYK Test", fill="black")
    img = img.convert("CMYK")

    result = await backend.process_image(img)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


@pytest.mark.anyio
async def test_file_path_processing(backend: TesseractBackend, tmp_path: Path) -> None:
    image = create_test_image("File Path Test")
    img_path = tmp_path / "test.png"
    image.save(img_path)

    result = await backend.process_file(img_path)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


def test_file_path_processing_sync(backend: TesseractBackend, tmp_path: Path) -> None:
    image = create_test_image("Sync File Test")
    img_path = tmp_path / "test_sync.png"
    image.save(img_path)

    result = backend.process_file_sync(img_path)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


@pytest.mark.parametrize(
    "width,height",
    [
        (100, 50),
        (400, 100),
        (800, 200),
        (1600, 400),
    ],
)
@pytest.mark.anyio
async def test_image_size_produces_output(backend: TesseractBackend, width: int, height: int) -> None:
    image = create_test_image("Size Test", width, height)

    result = await backend.process_image(image)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


@pytest.mark.anyio
async def test_table_detection_enabled_returns_metadata(backend: TesseractBackend) -> None:
    image = create_test_image("Product  Price  Quantity\nApple    1.50   10\nBanana   0.75   15", width=600, height=150)

    result = await backend.process_image(image, enable_table_detection=True)

    assert isinstance(result, ExtractionResult)
    assert "tables_detected" in result.metadata
    assert isinstance(result.metadata["tables_detected"], int)


@pytest.mark.anyio
async def test_table_detection_disabled_returns_metadata(backend: TesseractBackend) -> None:
    image = create_test_image("Product  Price  Quantity\nApple    1.50   10", width=600, height=100)

    result = await backend.process_image(image, enable_table_detection=False, output_format="text")

    assert isinstance(result, ExtractionResult)
    assert result.mime_type == "text/plain"


@pytest.mark.anyio
async def test_nonexistent_file_raises_error(backend: TesseractBackend) -> None:
    nonexistent_path = Path("/nonexistent/file/path.png")

    with pytest.raises((OCRError, RuntimeError, OSError)):
        await backend.process_file(nonexistent_path)


def test_nonexistent_file_raises_error_sync(backend: TesseractBackend) -> None:
    nonexistent_path = Path("/nonexistent/file/path.png")

    with pytest.raises((OCRError, RuntimeError, OSError)):
        backend.process_file_sync(nonexistent_path)


@pytest.mark.anyio
async def test_corrupted_image_file_raises_error(backend: TesseractBackend, tmp_path: Path) -> None:
    corrupted_file = tmp_path / "corrupted.png"
    corrupted_file.write_bytes(b"This is not a valid PNG file")

    with pytest.raises((OCRError, RuntimeError, OSError)):
        await backend.process_file(corrupted_file)


def test_corrupted_image_file_raises_error_sync(backend: TesseractBackend, tmp_path: Path) -> None:
    corrupted_file = tmp_path / "corrupted_sync.png"
    corrupted_file.write_bytes(b"Not a valid image")

    with pytest.raises((OCRError, RuntimeError, OSError)):
        backend.process_file_sync(corrupted_file)


def test_tesseract_config_default_values() -> None:
    config = TesseractConfig()

    assert config.language == "eng"
    assert config.psm == PSMMode.AUTO
    assert config.output_format == "markdown"
    assert config.enable_table_detection is True


def test_tesseract_config_custom_values() -> None:
    config = TesseractConfig(
        language="deu",
        psm=PSMMode.SINGLE_BLOCK,
        output_format="text",
        enable_table_detection=False,
    )

    assert config.language == "deu"
    assert config.psm == PSMMode.SINGLE_BLOCK
    assert config.output_format == "text"
    assert config.enable_table_detection is False


@pytest.mark.anyio
async def test_cache_enabled_produces_consistent_results(backend: TesseractBackend, tmp_path: Path) -> None:
    from kreuzberg._utils._cache import get_ocr_cache

    cache = get_ocr_cache()
    cache.clear()

    image = create_test_image("Cache Test 12345")
    img_path = tmp_path / "cache_test.png"
    image.save(img_path)

    result1 = await backend.process_file(img_path)
    result2 = await backend.process_file(img_path)

    assert result1.content == result2.content
    assert result1.mime_type == result2.mime_type


def test_cache_enabled_produces_consistent_results_sync(backend: TesseractBackend, tmp_path: Path) -> None:
    from kreuzberg._utils._cache import get_ocr_cache

    cache = get_ocr_cache()
    cache.clear()

    image = create_test_image("Sync Cache Test 67890")
    img_path = tmp_path / "sync_cache_test.png"
    image.save(img_path)

    result1 = backend.process_file_sync(img_path)
    result2 = backend.process_file_sync(img_path)

    assert result1.content == result2.content
    assert result1.mime_type == result2.mime_type


@pytest.mark.anyio
async def test_cache_disabled_produces_consistent_results(backend: TesseractBackend) -> None:
    image = create_test_image("No Cache Test")

    result1 = await backend.process_image(image)
    result2 = await backend.process_image(image)

    assert result1.mime_type == result2.mime_type


def test_batch_processing_empty_list(backend: TesseractBackend) -> None:
    results = backend.process_batch_sync([])

    assert results == []


def test_batch_processing_multiple_images(backend: TesseractBackend, tmp_path: Path) -> None:
    paths = []
    for i in range(3):
        img = create_test_image(f"Batch Image {i}")
        img_path = tmp_path / f"batch_{i}.png"
        img.save(img_path)
        paths.append(img_path)

    results = backend.process_batch_sync(paths)

    assert len(results) == 3
    for result in results:
        assert isinstance(result, ExtractionResult)
        assert len(result.content) > 0


def test_batch_processing_with_error_continues(backend: TesseractBackend, tmp_path: Path) -> None:
    valid_img = create_test_image("Valid Image")
    valid_path = tmp_path / "valid.png"
    valid_img.save(valid_path)

    invalid_path = tmp_path / "invalid.png"
    invalid_path.write_text("Not an image")

    results = backend.process_batch_sync([valid_path, invalid_path])

    assert len(results) == 2
    assert isinstance(results[0], ExtractionResult)
    assert len(results[0].content) > 0
    assert "[OCR error:" in results[1].content


@pytest.mark.anyio
async def test_clean_text_produces_expected_output(backend: TesseractBackend) -> None:
    image = create_test_image("Hello World 123", width=600, height=150)

    result = await backend.process_image(image)

    content = result.content.replace(" ", "").replace("\n", "").lower()
    assert "hello" in content or "world" in content or "123" in content


@pytest.mark.anyio
async def test_numbers_detected_accurately(backend: TesseractBackend) -> None:
    image = create_test_image("123456789", width=600, height=150)

    result = await backend.process_image(image)

    content = result.content.replace(" ", "").replace("\n", "")
    assert any(digit in content for digit in "123456789")


@pytest.mark.anyio
async def test_markdown_output_includes_source_format_metadata(backend: TesseractBackend) -> None:
    image = create_test_image("Metadata Test")

    result = await backend.process_image(image, output_format="markdown")

    assert "source_format" in result.metadata
    assert result.metadata["source_format"] == "hocr"


@pytest.mark.anyio
async def test_markdown_output_includes_tables_detected_metadata(backend: TesseractBackend) -> None:
    image = create_test_image("Tables Metadata Test")

    result = await backend.process_image(image, output_format="markdown")

    assert "tables_detected" in result.metadata
    assert isinstance(result.metadata["tables_detected"], int)
    assert result.metadata["tables_detected"] >= 0


@pytest.mark.anyio
async def test_output_normalizes_whitespace(backend: TesseractBackend) -> None:
    image = create_test_image("Text  With   Extra    Spaces")

    result = await backend.process_image(image)

    assert "    " not in result.content


@pytest.mark.anyio
async def test_process_real_document_if_exists(backend: TesseractBackend) -> None:
    test_image_path = Path("test_documents/images/ocr_image.jpg")

    if not test_image_path.exists():
        pytest.skip("Test document not found")

    result = await backend.process_file(test_image_path)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 20
    assert result.mime_type == "text/markdown"


@pytest.mark.anyio
async def test_very_small_image(backend: TesseractBackend) -> None:
    image = create_test_image("X", width=50, height=50)

    result = await backend.process_image(image)

    assert isinstance(result, ExtractionResult)


@pytest.mark.anyio
async def test_very_large_image(backend: TesseractBackend) -> None:
    image = create_test_image("Large Image Test", width=3000, height=800)

    result = await backend.process_image(image)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0
