from __future__ import annotations

from pathlib import Path
from typing import Any

import anyio
import pytest
from PIL import Image, ImageDraw, ImageFont

from kreuzberg import PSMMode
from kreuzberg._ocr._tesseract import (
    TesseractBackend,
)
from kreuzberg._types import ExtractionResult
from kreuzberg.exceptions import ValidationError


@pytest.fixture(scope="session")
def backend() -> TesseractBackend:
    return TesseractBackend()


def create_test_image(text: str) -> Image.Image:
    img = Image.new("RGB", (400, 100), "white")
    draw = ImageDraw.Draw(img)
    font = ImageFont.load_default()
    draw.text((10, 30), text, fill="black", font=font)
    return img


@pytest.mark.anyio
async def test_validate_tesseract_version(backend: TesseractBackend) -> None:
    TesseractBackend._version_checked = False
    await backend._ensure_version_checked()
    assert TesseractBackend._version_checked is True


@pytest.mark.anyio
async def test_process_file(backend: TesseractBackend, ocr_image: Path) -> None:
    result = await backend.process_file(ocr_image, language="eng", psm=PSMMode.AUTO)
    assert isinstance(result, ExtractionResult)
    assert result.mime_type == "text/markdown"
    assert len(result.content.strip()) > 0
    assert result.content.strip() not in ["[No text detected]", "[OCR processing failed]"]


@pytest.mark.anyio
async def test_process_file_with_options(backend: TesseractBackend, ocr_image: Path) -> None:
    result = await backend.process_file(ocr_image, language="eng", psm=PSMMode.SINGLE_BLOCK)
    assert isinstance(result, ExtractionResult)
    assert result.mime_type == "text/markdown"
    assert len(result.content.strip()) > 0
    assert result.content.strip() not in ["[No text detected]", "[OCR processing failed]"]


@pytest.mark.anyio
async def test_process_file_error(backend: TesseractBackend, fresh_cache: None) -> None:
    nonexistent_file = Path("/nonexistent/path/file.png")

    with pytest.raises(OSError):  # noqa: PT011
        await backend.process_file(nonexistent_file, language="eng", psm=PSMMode.AUTO)


@pytest.mark.anyio
async def test_process_file_runtime_error(backend: TesseractBackend, fresh_cache: None) -> None:
    import tempfile

    with tempfile.NamedTemporaryFile(suffix=".png", delete=False) as f:
        f.write(b"This is not a valid image file")
        invalid_file = Path(f.name)

    try:
        with pytest.raises(RuntimeError):
            await backend.process_file(invalid_file, language="eng", psm=PSMMode.AUTO)
    finally:
        invalid_file.unlink(missing_ok=True)


@pytest.mark.anyio
async def test_process_image(backend: TesseractBackend) -> None:
    image = Image.new("RGB", (400, 100), "white")
    draw = ImageDraw.Draw(image)
    draw.text((10, 30), "Hello World Test", fill="black")

    result = await backend.process_image(image, language="eng", psm=PSMMode.AUTO)
    assert isinstance(result, ExtractionResult)
    assert result.mime_type == "text/markdown"
    assert len(result.content.strip()) > 0
    assert result.content.strip() not in ["[No text detected]", "[OCR processing failed]"]


@pytest.mark.anyio
async def test_process_image_with_tesseract_pillow(backend: TesseractBackend) -> None:
    image = Image.new("RGB", (400, 100), "white")
    draw = ImageDraw.Draw(image)
    draw.text((10, 30), "Test Document", fill="black")

    result = await backend.process_image(image)
    assert isinstance(result, ExtractionResult)
    assert result.mime_type == "text/markdown"
    assert len(result.content.strip()) > 0
    assert result.content.strip() not in ["[No text detected]", "[OCR processing failed]"]


@pytest.mark.anyio
async def test_integration_process_file(backend: TesseractBackend, ocr_image: Path) -> None:
    result = await backend.process_file(ocr_image, language="eng", psm=PSMMode.AUTO)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip()


@pytest.mark.anyio
async def test_process_file_with_invalid_language(backend: TesseractBackend, ocr_image: Path) -> None:
    with pytest.raises(ValidationError, match="not supported"):
        await backend.process_file(ocr_image, language="invalid", psm=PSMMode.AUTO)


@pytest.mark.parametrize(
    "language_code,expected_result",
    [
        ("eng", "eng"),
        ("ENG", "eng"),
        ("deu", "deu"),
        ("fra", "fra"),
        ("spa", "spa"),
        ("jpn", "jpn"),
        ("chi_sim", "chi_sim"),
        ("chi_tra", "chi_tra"),
    ],
)
def test_validate_language_code_valid(language_code: str, expected_result: str) -> None:
    from kreuzberg._internal_bindings import validate_language_code

    result = validate_language_code(language_code)
    assert result == expected_result


@pytest.mark.parametrize(
    "invalid_language_code",
    [
        "invalid",
        "english",
        "español",
        "русский",
        "en",
        "de",
        "fr",
        "zh",
        "",
        "123",
    ],
)
def test_validate_language_code_invalid(invalid_language_code: str) -> None:
    from kreuzberg._internal_bindings import validate_language_code

    with pytest.raises((ValidationError, ValueError)) as excinfo:
        validate_language_code(invalid_language_code)

    assert "not supported" in str(excinfo.value).lower() or "language" in str(excinfo.value).lower()


@pytest.mark.anyio
async def test_integration_process_image(backend: TesseractBackend, ocr_image: Path) -> None:
    image = Image.open(ocr_image)
    with image:
        result = await backend.process_image(image, language="eng", psm=PSMMode.AUTO)
        assert isinstance(result, ExtractionResult)
        assert result.content.strip()


def test_validate_language_code_error() -> None:
    from kreuzberg._internal_bindings import validate_language_code

    with pytest.raises((ValidationError, ValueError)):
        validate_language_code("invalid_language_code_that_is_too_long_and_invalid")


@pytest.mark.anyio
async def test_process_image_validation_error(backend: TesseractBackend) -> None:
    test_image = Image.new("RGB", (1, 1), color="white")

    with pytest.raises(ValidationError, match="not supported"):
        await backend.process_image(test_image, language="invalid_lang_code")


@pytest.mark.anyio
async def test_process_file_validation_error(backend: TesseractBackend, tmp_path: Path) -> None:
    test_file = tmp_path / "test.png"
    test_image = Image.new("RGB", (100, 50), color="white")
    test_image.save(test_file)

    with pytest.raises(ValidationError, match="not supported"):
        await backend.process_file(test_file, language="invalid_lang_code")


def test_process_image_sync(backend: TesseractBackend) -> None:
    image = Image.new("RGB", (200, 100), "white")
    draw = ImageDraw.Draw(image)
    draw.text((10, 30), "Sync Test", fill="black")

    result = backend.process_image_sync(image, language="eng")

    assert isinstance(result, ExtractionResult)
    assert result.mime_type == "text/markdown"
    assert len(result.content.strip()) > 0
    assert result.content.strip() not in ["[No text detected]", "[OCR processing failed]"]


def test_process_file_sync(backend: TesseractBackend, ocr_image: Path) -> None:
    result = backend.process_file_sync(ocr_image, language="eng")

    assert isinstance(result, ExtractionResult)
    assert result.mime_type == "text/markdown"
    assert len(result.content.strip()) > 0
    assert result.content.strip() not in ["[No text detected]", "[OCR processing failed]"]


def test_tesseract_config_validation_tesseract_config_all_parameters() -> None:
    from kreuzberg._types import TesseractConfig

    config = TesseractConfig(
        language="eng+deu",
        psm=PSMMode.SINGLE_BLOCK,
        tessedit_char_whitelist="0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
        tessedit_enable_dict_correction=False,
        language_model_ngram_on=True,
        textord_space_size_is_variable=False,
        tessedit_dont_blkrej_good_wds=True,
        tessedit_dont_rowrej_good_wds=False,
        tessedit_use_primary_params_model=True,
        classify_use_pre_adapted_templates=False,
        thresholding_method=True,
    )

    assert config.language == "eng+deu"
    assert config.psm == PSMMode.SINGLE_BLOCK
    assert "0123456789" in config.tessedit_char_whitelist
    assert config.tessedit_enable_dict_correction is False
    assert config.language_model_ngram_on is True
    assert config.textord_space_size_is_variable is False
    assert config.tessedit_dont_blkrej_good_wds is True
    assert config.tessedit_dont_rowrej_good_wds is False
    assert config.tessedit_use_primary_params_model is True
    assert config.classify_use_pre_adapted_templates is False
    assert config.thresholding_method is True


def test_tesseract_config_validation_tesseract_config_default_values() -> None:
    from kreuzberg._types import TesseractConfig

    config = TesseractConfig()

    assert config.language == "eng"
    assert config.psm == PSMMode.AUTO
    assert config.tessedit_char_whitelist == ""
    assert config.tessedit_enable_dict_correction is True
    assert config.language_model_ngram_on is False
    assert config.textord_space_size_is_variable is True
    assert config.tessedit_dont_blkrej_good_wds is True
    assert config.tessedit_dont_rowrej_good_wds is True
    assert config.tessedit_use_primary_params_model is True
    assert config.classify_use_pre_adapted_templates is True
    assert config.thresholding_method is False


@pytest.mark.parametrize(
    "psm_mode",
    [
        PSMMode.OSD_ONLY,
        PSMMode.AUTO_OSD,
        PSMMode.AUTO_ONLY,
        PSMMode.AUTO,
        PSMMode.SINGLE_COLUMN,
        PSMMode.SINGLE_BLOCK_VERTICAL,
        PSMMode.SINGLE_BLOCK,
        PSMMode.SINGLE_LINE,
        PSMMode.SINGLE_WORD,
        PSMMode.CIRCLE_WORD,
        PSMMode.SINGLE_CHAR,
    ],
)
def test_tesseract_config_validation_psm_mode_values(psm_mode: PSMMode) -> None:
    from kreuzberg._types import TesseractConfig

    config = TesseractConfig(psm=psm_mode)
    assert config.psm == psm_mode
    assert isinstance(psm_mode.value, int)
    assert 0 <= psm_mode.value <= 10


@pytest.mark.parametrize(
    "language_code",
    [
        "afr",
        "amh",
        "ara",
        "asm",
        "aze",
        "aze_cyrl",
        "bel",
        "ben",
        "bod",
        "bos",
        "bre",
        "bul",
        "cat",
        "ceb",
        "ces",
        "chi_sim",
        "chi_tra",
        "chr",
        "cos",
        "cym",
        "dan",
        "deu",
        "dzo",
        "ell",
        "eng",
        "enm",
        "epo",
        "est",
        "eus",
        "fao",
        "fas",
        "fil",
        "fin",
        "fra",
        "frk",
        "frm",
        "fry",
        "gla",
        "gle",
        "glg",
        "grc",
        "guj",
        "hat",
        "heb",
        "hin",
        "hrv",
        "hun",
        "hye",
        "iku",
        "ind",
        "isl",
        "ita",
        "ita_old",
        "jav",
        "jpn",
        "kan",
        "kat",
        "kat_old",
        "kaz",
        "khm",
        "kir",
        "kor",
        "kur",
        "lao",
        "lat",
        "lav",
        "lit",
        "ltz",
        "mal",
        "mar",
        "mkd",
        "mlt",
        "mon",
        "mri",
        "msa",
        "mya",
        "nep",
        "nld",
        "nor",
        "oci",
        "ori",
        "pan",
        "pol",
        "por",
        "pus",
        "que",
        "ron",
        "rus",
        "san",
        "sin",
        "slk",
        "slv",
        "snd",
        "spa",
        "spa_old",
        "sqi",
        "srp",
        "srp_latn",
        "sun",
        "swa",
        "swe",
        "syr",
        "tam",
        "tat",
        "tel",
        "tgk",
        "tha",
        "tir",
        "ton",
        "tur",
        "uig",
        "ukr",
        "urd",
        "uzb",
        "uzb_cyrl",
        "vie",
        "yid",
        "yor",
    ],
)
def test_tesseract_language_validation_all_supported_language_codes(language_code: str) -> None:
    from kreuzberg._internal_bindings import validate_language_code

    result = validate_language_code(language_code)
    assert result == language_code.lower()


@pytest.mark.anyio
async def test_tesseract_image_processing_process_image_with_different_modes(backend: TesseractBackend) -> None:
    modes = ["RGB", "RGBA", "L", "P", "CMYK"]

    for mode in modes:
        if mode == "CMYK":
            image = Image.new("RGB", (200, 100), "white")
            from PIL import ImageDraw

            draw = ImageDraw.Draw(image)
            draw.text((10, 40), "TEST", fill="black")
            image = image.convert("CMYK")
        else:
            image = Image.new(mode, (200, 100), "white")
            from PIL import ImageDraw

            draw = ImageDraw.Draw(image)
            draw.text((10, 40), "TEST", fill="black")

        result = await backend.process_image(image, language="eng")
        assert isinstance(result, ExtractionResult)
        assert len(result.content) > 0


@pytest.mark.anyio
async def test_tesseract_error_handling_process_file_file_not_found(backend: TesseractBackend) -> None:
    nonexistent_file = Path("/nonexistent/file.png")

    with pytest.raises(OSError):  # noqa: PT011
        await backend.process_file(nonexistent_file, language="eng")


@pytest.mark.anyio
async def test_tesseract_error_handling_process_image_invalid_format(backend: TesseractBackend) -> None:
    import tempfile

    with tempfile.NamedTemporaryFile(suffix=".png", delete=False) as f:
        f.write(b"This is not a valid PNG file")
        invalid_path = Path(f.name)

    try:
        with pytest.raises(RuntimeError):
            await backend.process_file(invalid_path, language="eng")
    finally:
        invalid_path.unlink()


def test_tesseract_error_handling_sync_process_image_temp_file_error(backend: TesseractBackend) -> None:
    image = Image.new("RGB", (1, 1), "white")

    result = backend.process_image_sync(image, language="eng")

    assert isinstance(result, ExtractionResult)
    assert result.mime_type == "text/markdown"
    assert result.content is not None


def test_tesseract_config_edge_cases_empty_whitelist() -> None:
    from kreuzberg._types import TesseractConfig

    config = TesseractConfig(tessedit_char_whitelist="")
    assert config.tessedit_char_whitelist == ""


def test_tesseract_config_edge_cases_very_long_whitelist() -> None:
    from kreuzberg._types import TesseractConfig

    long_whitelist = "".join(chr(i) for i in range(32, 127))

    config = TesseractConfig(tessedit_char_whitelist=long_whitelist)
    assert len(config.tessedit_char_whitelist) > 90


def test_tesseract_config_edge_cases_unicode_language_combinations() -> None:
    from kreuzberg._internal_bindings import validate_language_code

    valid_combinations = [
        "ara+eng",
        "chi_sim+eng+deu",
        "jpn+kor+eng",
        "rus+ukr+eng",
        "hin+pan+urd+eng",
    ]

    for combo in valid_combinations:
        result = validate_language_code(combo)
        assert result == combo.lower()


@pytest.mark.parametrize(
    "test_image_path,expected_content_keywords,description",
    [
        (
            "test_documents/images/ocr_image.jpg",
            ["Nasdaq", "AMEX", "Stock", "Track"],
            "Financial newspaper table with stock data",
        ),
        (
            "test_documents/images/layout_parser_ocr.jpg",
            ["LayoutParser", "Table", "Dataset", "document"],
            "Academic paper with tables and technical content",
        ),
        (
            "test_documents/tables/simple_table.png",
            ["Product", "Price", "Quantity", "Apple", "Banana"],
            "Simple product table with clear borders",
        ),
        (
            "test_documents/images/invoice_image.png",
            [],
            "Invoice document image",
        ),
        ("test_documents/images/test_hello_world.png", ["Hello", "World"], "Simple text image"),
    ],
)
@pytest.mark.anyio
async def test_markdown_extraction_diverse_documents(
    backend: TesseractBackend, test_image_path: str, expected_content_keywords: list[str], description: str
) -> None:
    image_path = Path(test_image_path)

    if not image_path.exists():
        pytest.skip(f"Test image {test_image_path} not found")

    try:
        result = await backend.process_file(image_path, language="eng", psm=PSMMode.AUTO)

        assert isinstance(result, ExtractionResult)
        assert result.mime_type == "text/markdown"

        content = result.content.strip()
        assert len(content) > 0
        assert content not in ["[No text detected]", "[OCR processing failed]"]

        if expected_content_keywords:
            content_lower = content.lower()
            found_keywords = [kw for kw in expected_content_keywords if kw.lower() in content_lower]
            assert len(found_keywords) > 0, (
                f"Expected keywords {expected_content_keywords} not found in content: {content[:200]}..."
            )

        assert "language" in result.metadata
        assert "output_format" in result.metadata

        if "table_count" in result.metadata:
            tables_count = int(result.metadata["table_count"])
            assert isinstance(tables_count, int)
            assert tables_count >= 0

    except Exception as e:
        pytest.fail(f"Failed to process {description} ({test_image_path}): {e}")


@pytest.mark.parametrize(
    "test_image_path,description",
    [
        ("test_documents/tables/simple_table.png", "Simple table with clear borders"),
        ("test_documents/images/ocr_image.jpg", "Financial data table"),
    ],
)
@pytest.mark.anyio
async def test_markdown_extraction_with_table_detection(
    backend: TesseractBackend, test_image_path: str, description: str
) -> None:
    image_path = Path(test_image_path)

    if not image_path.exists():
        pytest.skip(f"Test image {test_image_path} not found")

    result = await backend.process_file(
        image_path,
        language="eng",
        psm=PSMMode.AUTO,
        enable_table_detection=True,
        table_min_confidence=20.0,
    )

    assert isinstance(result, ExtractionResult)
    assert result.mime_type == "text/markdown"

    content = result.content.strip()
    assert len(content) > 0
    assert content not in ["[No text detected]", "[OCR processing failed]"]

    if "table_count" in result.metadata:
        tables_count = int(result.metadata["table_count"])
        assert isinstance(tables_count, int)

        if tables_count > 0:
            assert "|" in content


@pytest.mark.anyio
async def test_html_to_markdown_config_defaults() -> None:
    from kreuzberg._types import HTMLToMarkdownConfig

    config = HTMLToMarkdownConfig()

    assert config.escape_misc is False
    assert config.escape_asterisks is False
    assert config.escape_underscores is False
    assert config.extract_metadata is True


@pytest.mark.anyio
async def test_tesseract_concurrent_processing(backend: TesseractBackend) -> None:
    images = []
    for i in range(3):  # ~keep Reduce to 3 for faster testing
        img = Image.new("RGB", (200, 100), "white")
        draw = ImageDraw.Draw(img)
        draw.text((10, 40), f"TEXT{i}", fill="black")
        images.append(img)

    async def process_image(img: Any) -> ExtractionResult:
        return await backend.process_image(img, language="eng")

    results = []
    async with anyio.create_task_group() as tg:
        for img in images:

            async def process_and_append(image: Any) -> None:
                result = await process_image(image)
                results.append(result)

            tg.start_soon(process_and_append, img)

    assert len(results) == 3
    for result in results:
        assert isinstance(result, ExtractionResult)
        assert len(result.content) >= 0


@pytest.mark.anyio
async def test_tesseract_output_format_tsv() -> None:
    backend = TesseractBackend()
    img = create_test_image("Test TSV Output")

    result = await backend.process_image(img, output_format="tsv")
    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


@pytest.mark.anyio
async def test_tesseract_output_format_hocr() -> None:
    backend = TesseractBackend()
    img = create_test_image("Test HOCR Output")

    result = await backend.process_image(img, output_format="hocr")
    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


@pytest.mark.anyio
async def test_tesseract_output_format_text() -> None:
    backend = TesseractBackend()
    img = create_test_image("Test Text Output")

    result = await backend.process_image(img, output_format="text")
    assert isinstance(result, ExtractionResult)
    assert "Test Text Output" in result.content


@pytest.mark.anyio
async def test_tesseract_table_detection_with_tsv() -> None:
    backend = TesseractBackend()

    img = Image.new("RGB", (400, 200), "white")
    draw = ImageDraw.Draw(img)
    font = ImageFont.load_default()

    draw.text((10, 10), "Name    Age    City", fill="black", font=font)
    draw.text((10, 40), "Alice   30     NYC", fill="black", font=font)
    draw.text((10, 70), "Bob     25     LA", fill="black", font=font)

    result = await backend.process_image(img, enable_table_detection=True)
    assert isinstance(result, ExtractionResult)


def test_process_batch_sync_empty_list() -> None:
    backend = TesseractBackend()
    results = backend.process_batch_sync([])
    assert results == []


def test_process_batch_sync_single_image(tmp_path: Path) -> None:
    backend = TesseractBackend()

    img = create_test_image("Batch Test 1")
    img_path = tmp_path / "test1.png"
    img.save(img_path)

    results = backend.process_batch_sync([img_path])
    assert len(results) == 1
    assert isinstance(results[0], ExtractionResult)


def test_process_batch_sync_multiple_images(tmp_path: Path) -> None:
    backend = TesseractBackend()

    paths = []
    for i in range(3):
        img = create_test_image(f"Batch Test {i + 1}")
        img_path = tmp_path / f"test{i}.png"
        img.save(img_path)
        paths.append(img_path)

    results = backend.process_batch_sync(paths)
    assert len(results) == 3
    for result in results:
        assert isinstance(result, ExtractionResult)


def test_process_batch_sync_with_invalid_image(tmp_path: Path) -> None:
    backend = TesseractBackend()

    valid_img = create_test_image("Valid")
    valid_path = tmp_path / "valid.png"
    valid_img.save(valid_path)

    invalid_path = tmp_path / "invalid.png"
    invalid_path.write_text("not an image")

    results = backend.process_batch_sync([valid_path, invalid_path])
    assert len(results) == 2
    assert isinstance(results[0], ExtractionResult)
    assert "[OCR error:" in results[1].content


@pytest.mark.anyio
async def test_tesseract_tsv_table_extraction_edge_cases() -> None:
    backend = TesseractBackend()

    img = Image.new("RGB", (100, 100), "white")
    draw = ImageDraw.Draw(img)
    draw.text((10, 10), "Just text", fill="black")

    result = await backend.process_image(img, enable_table_detection=True)
    assert isinstance(result, ExtractionResult)


@pytest.mark.anyio
async def test_tesseract_extract_text_from_tsv_error_handling() -> None:
    backend = TesseractBackend()

    img = create_test_image("TSV Test")
    result = await backend.process_image(img, output_format="tsv")

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


@pytest.mark.integration
@pytest.mark.anyio
async def test_tesseract_hocr_with_tables_integration() -> None:
    backend = TesseractBackend()

    img = Image.new("RGB", (800, 300), "white")
    draw = ImageDraw.Draw(img)
    try:
        font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 24)
    except Exception:
        font = ImageFont.load_default()  # type: ignore[assignment]

    draw.text((50, 30), "Product", fill="black", font=font)
    draw.text((300, 30), "Price", fill="black", font=font)
    draw.text((500, 30), "Quantity", fill="black", font=font)

    draw.text((50, 100), "Apple", fill="black", font=font)
    draw.text((300, 100), "150", fill="black", font=font)
    draw.text((500, 100), "10", fill="black", font=font)

    draw.text((50, 170), "Banana", fill="black", font=font)
    draw.text((300, 170), "75", fill="black", font=font)
    draw.text((500, 170), "15", fill="black", font=font)

    result = await backend.process_image(img, output_format="hocr", enable_table_detection=True)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0
    content_lower = result.content.lower()
    assert "product" in content_lower or "apple" in content_lower or "banana" in content_lower


@pytest.mark.integration
@pytest.mark.anyio
async def test_tesseract_tsv_with_table_reconstruction_integration() -> None:
    backend = TesseractBackend()

    img = Image.new("RGB", (1000, 400), "white")
    draw = ImageDraw.Draw(img)

    try:
        font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 28)
    except Exception:
        font = ImageFont.load_default()  # type: ignore[assignment]

    y_pos = 40
    draw.text((80, y_pos), "Name", fill="black", font=font)
    draw.text((320, y_pos), "Age", fill="black", font=font)
    draw.text((520, y_pos), "City", fill="black", font=font)
    draw.text((750, y_pos), "Score", fill="black", font=font)

    y_pos += 80
    draw.text((80, y_pos), "Alice", fill="black", font=font)
    draw.text((320, y_pos), "30", fill="black", font=font)
    draw.text((520, y_pos), "NYC", fill="black", font=font)
    draw.text((750, y_pos), "95", fill="black", font=font)

    y_pos += 80
    draw.text((80, y_pos), "Bob", fill="black", font=font)
    draw.text((320, y_pos), "25", fill="black", font=font)
    draw.text((520, y_pos), "LA", fill="black", font=font)
    draw.text((750, y_pos), "88", fill="black", font=font)

    y_pos += 80
    draw.text((80, y_pos), "Charlie", fill="black", font=font)
    draw.text((320, y_pos), "35", fill="black", font=font)
    draw.text((520, y_pos), "Chicago", fill="black", font=font)
    draw.text((750, y_pos), "92", fill="black", font=font)

    result = await backend.process_image(img, output_format="tsv", enable_table_detection=True)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0
    content_lower = result.content.lower()
    matches = sum([name in content_lower for name in ["alice", "bob", "charlie"]])
    assert matches >= 2, f"Expected at least 2 names in OCR output, but found {matches}"


@pytest.mark.integration
def test_process_batch_sync_integration(tmp_path: Path) -> None:
    backend = TesseractBackend()

    expected_texts = ["BATCH001", "BATCH002", "BATCH003"]
    paths = []

    for i, text in enumerate(expected_texts):
        img = Image.new("RGB", (600, 150), "white")
        draw = ImageDraw.Draw(img)

        try:
            font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 36)
        except Exception:
            font = ImageFont.load_default()  # type: ignore[assignment]

        draw.text((100, 50), text, fill="black", font=font)

        img_path = tmp_path / f"batch_test_{i}.png"
        img.save(img_path)
        paths.append(img_path)

    results = backend.process_batch_sync(paths)

    assert len(results) == 3
    for i, result in enumerate(results):
        assert isinstance(result, ExtractionResult)
        assert len(result.content) > 0
        content_upper = result.content.upper().replace(" ", "")
        assert "BATCH" in content_upper, f"Expected 'BATCH' in result {i}, got: {result.content}"


def test_tesseract_sync_process_file_integration(tmp_path: Path) -> None:
    backend = TesseractBackend()

    img = Image.new("RGB", (700, 150), "white")
    draw = ImageDraw.Draw(img)

    try:
        font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 36)
    except Exception:
        font = ImageFont.load_default()  # type: ignore[assignment]

    test_text = "HELLO WORLD"
    draw.text((100, 50), test_text, fill="black", font=font)

    img_path = tmp_path / "sync_test.png"
    img.save(img_path)

    result = backend.process_file_sync(img_path)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0
    content_upper = result.content.upper().replace(" ", "")
    assert "HELLO" in content_upper or "WORLD" in content_upper


def test_tesseract_sync_process_image_integration() -> None:
    backend = TesseractBackend()

    img = Image.new("RGB", (700, 150), "white")
    draw = ImageDraw.Draw(img)

    try:
        font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 48)
    except Exception:
        font = ImageFont.load_default()  # type: ignore[assignment]

    test_text = "12345"
    draw.text((150, 40), test_text, fill="black", font=font)

    result = backend.process_image_sync(img)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0
    content_cleaned = result.content.replace(" ", "").replace("\n", "")
    assert any(digit in content_cleaned for digit in "12345")


def test_tesseract_sync_process_image_with_cache_hit() -> None:
    from kreuzberg._utils._cache import get_ocr_cache

    backend = TesseractBackend()

    img = create_test_image("SYNC CACHE IMAGE 789")

    cache = get_ocr_cache()
    cache.clear()

    result1 = backend.process_image_sync(img)
    assert isinstance(result1, ExtractionResult)
    assert len(result1.content) > 0

    result2 = backend.process_image_sync(img)
    assert isinstance(result2, ExtractionResult)
    assert result2.content == result1.content


def test_tesseract_sync_process_file_with_cache_hit(tmp_path: Path) -> None:
    from kreuzberg._utils._cache import get_ocr_cache

    backend = TesseractBackend()

    img = create_test_image("SYNC CACHE FILE 456")
    img_path = tmp_path / "sync_cache_file.png"
    img.save(img_path)

    cache = get_ocr_cache()
    cache.clear()

    result1 = backend.process_file_sync(img_path)
    assert isinstance(result1, ExtractionResult)
    assert len(result1.content) > 0

    result2 = backend.process_file_sync(img_path)
    assert isinstance(result2, ExtractionResult)
    assert result2.content == result1.content


def test_tesseract_sync_process_file_with_hocr_output(tmp_path: Path) -> None:
    backend = TesseractBackend()

    img = create_test_image("SYNC HOCR TEST")
    img_path = tmp_path / "sync_hocr.png"
    img.save(img_path)

    result = backend.process_file_sync(img_path, output_format="hocr")

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


def test_tesseract_sync_process_file_with_markdown_output(tmp_path: Path) -> None:
    backend = TesseractBackend()

    img = create_test_image("SYNC MARKDOWN TEST")
    img_path = tmp_path / "sync_markdown.png"
    img.save(img_path)

    result = backend.process_file_sync(img_path, output_format="markdown")

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


def test_tesseract_sync_process_file_with_tsv_output(tmp_path: Path) -> None:
    backend = TesseractBackend()

    img = create_test_image("SYNC TSV TEST")
    img_path = tmp_path / "sync_tsv.png"
    img.save(img_path)

    result = backend.process_file_sync(img_path, output_format="tsv")

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


def test_tesseract_sync_process_file_with_tsv_and_table_detection(tmp_path: Path) -> None:
    backend = TesseractBackend()

    img = create_test_image("SYNC TSV TABLE TEST")
    img_path = tmp_path / "sync_tsv_table.png"
    img.save(img_path)

    result = backend.process_file_sync(img_path, output_format="tsv", enable_table_detection=True)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


@pytest.mark.anyio
async def test_tesseract_text_output_format(tmp_path: Path) -> None:
    backend = TesseractBackend()
    img = create_test_image("PLAIN TEXT TEST")
    img_path = tmp_path / "plain_text.png"
    img.save(img_path)

    result = await backend.process_file(img_path, output_format="text")
    assert isinstance(result, ExtractionResult)
    assert result.mime_type == "text/plain"


def test_tesseract_sync_image_mode_conversion() -> None:
    backend = TesseractBackend()

    img = Image.new("CMYK", (400, 100), "white")

    result = backend.process_image_sync(img)
    assert isinstance(result, ExtractionResult)


def test_tesseract_table_prefix_kwargs(tmp_path: Path) -> None:
    backend = TesseractBackend()
    img = create_test_image("TABLE PREFIX TEST")
    img_path = tmp_path / "table_prefix.png"
    img.save(img_path)

    result = backend.process_file_sync(img_path, table_column_threshold=10, table_row_threshold_ratio=0.5)
    assert isinstance(result, ExtractionResult)


def test_tesseract_batch_processing_error(tmp_path: Path) -> None:
    backend = TesseractBackend()

    valid_img = create_test_image("BATCH TEST")
    valid_path = tmp_path / "valid.png"
    valid_img.save(valid_path)

    invalid_path = tmp_path / "nonexistent.png"

    results = backend.process_batch_sync([valid_path, invalid_path])

    assert len(results) == 2
    assert isinstance(results[0], ExtractionResult)
    assert "[OCR error:" in results[1].content
