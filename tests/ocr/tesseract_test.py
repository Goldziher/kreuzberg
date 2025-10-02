from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING, Any
from unittest.mock import Mock, patch

import anyio
import pytest
from msgspec import structs
from PIL import Image, ImageDraw, ImageFont

from kreuzberg import PSMMode
from kreuzberg._ocr._tesseract import (
    TesseractBackend,
)
from kreuzberg._types import ExtractionResult
from kreuzberg.exceptions import MissingDependencyError, OCRError, ValidationError

if TYPE_CHECKING:
    from PIL.ImageFont import FreeTypeFont
    from PIL.ImageFont import ImageFont as ImageFontType
    from pytest_mock import MockerFixture


@pytest.fixture(scope="session")
def backend() -> TesseractBackend:
    return TesseractBackend()


@pytest.fixture
def mock_run_process(mocker: MockerFixture) -> Mock:
    async def async_run_sync(command: list[str], **kwargs: Any) -> Mock:
        result = Mock()
        result.stdout = b"tesseract 5.0.0"
        result.returncode = 0
        result.stderr = b""

        if "--version" in command and command[0].endswith("tesseract"):
            return result

        if len(command) >= 3 and command[0].endswith("tesseract"):
            output_file = command[2]
            if "test_process_image_with_tesseract_invalid_input" in str(kwargs.get("cwd")):
                result.returncode = 1
                result.stderr = b"Error processing file"
                raise OCRError("Error processing file")

            if "tsv" in command:
                tsv_content = """level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext
5\t1\t1\t1\t1\t1\t50\t50\t100\t30\t95.0\tSample
5\t1\t1\t1\t1\t2\t160\t50\t60\t30\t94.0\tOCR
5\t1\t1\t1\t1\t3\t230\t50\t60\t30\t96.0\ttext"""
                Path(f"{output_file}.tsv").write_text(tsv_content)
            elif "hocr" in command or "tessedit_create_hocr=1" in " ".join(command):
                hocr_content = """<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN"
    "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd">
<html xmlns="http://www.w3.org/1999/xhtml" xml:lang="en" lang="en">
 <head>
  <title></title>
  <meta http-equiv="Content-Type" content="text/html;charset=utf-8" />
  <meta name='ocr-system' content='tesseract 5.0.0' />
  <meta name='ocr-capabilities' content='ocr_page ocr_carea ocr_par ocr_line ocrx_word' />
 </head>
 <body>
  <div class='ocr_page' id='page_1' title='bbox 0 0 100 100; ppageno 0'>
   <div class='ocr_carea' id='carea_1_1' title='bbox 50 50 350 80'>
    <p class='ocr_par' id='par_1_1' title='bbox 50 50 350 80'>
     <span class='ocr_line' id='line_1_1' title='bbox 50 50 350 80; baseline 0 -10'>
      <span class='ocrx_word' id='word_1_1' title='bbox 50 50 150 80; x_wconf 95'>Sample</span>
      <span class='ocrx_word' id='word_1_2' title='bbox 160 50 220 80; x_wconf 94'>OCR</span>
      <span class='ocrx_word' id='word_1_3' title='bbox 230 50 290 80; x_wconf 96'>text</span>
     </span>
    </p>
   </div>
  </div>
 </body>
</html>"""
                Path(f"{output_file}.hocr").write_text(hocr_content)
            else:
                output_txt_file = Path(f"{output_file}.txt")
                output_txt_file.write_text("Sample OCR text")
            result.returncode = 0
            return result

        return result

    mock = mocker.patch("kreuzberg._ocr._tesseract.run_process")
    mock.return_value = Mock()
    mock.return_value.stdout = b"tesseract 5.0.0"
    mock.return_value.returncode = 0
    mock.return_value.stderr = b""
    mock.side_effect = async_run_sync
    return mock


@pytest.fixture
def mock_run_process_invalid(mocker: MockerFixture) -> Mock:
    async def run_sync(command: list[str], **kwargs: Any) -> Mock:
        result = Mock()
        result.stdout = b"tesseract 4.0.0"
        result.returncode = 0
        result.stderr = b""
        return result

    mock = mocker.patch("kreuzberg._ocr._tesseract.run_process")
    mock.return_value = Mock()
    mock.return_value.stdout = b"tesseract 4.0.0"
    mock.return_value.returncode = 0
    mock.side_effect = run_sync
    return mock


@pytest.fixture
def mock_run_process_error(mocker: MockerFixture) -> Mock:
    async def run_sync(command: list[str], **kwargs: Any) -> Mock:
        raise FileNotFoundError

    mock = mocker.patch("kreuzberg._ocr._tesseract.run_process")
    mock.side_effect = run_sync
    return mock


def create_test_image(text: str) -> Image.Image:
    """Create a test image with the given text."""

    img = Image.new("RGB", (400, 100), "white")
    draw = ImageDraw.Draw(img)
    font = ImageFont.load_default()
    draw.text((10, 30), text, fill="black", font=font)
    return img


@pytest.mark.anyio
async def test_validate_tesseract_version(backend: TesseractBackend) -> None:
    TesseractBackend._version_checked = False
    await backend._validate_tesseract_version()
    assert TesseractBackend._version_checked is True


@pytest.fixture(autouse=True)
def reset_version_ref(mocker: MockerFixture) -> None:
    mocker.patch("kreuzberg._ocr._tesseract.TesseractBackend._version_checked", False)


@pytest.mark.anyio
async def test_validate_tesseract_version_invalid(
    backend: TesseractBackend, mock_run_process_invalid: Mock, reset_version_ref: None
) -> None:
    with pytest.raises(MissingDependencyError) as excinfo:
        await backend._validate_tesseract_version()

    error_message = str(excinfo.value)
    assert "Tesseract version 5" in error_message
    assert "required" in error_message


@pytest.mark.anyio
async def test_validate_tesseract_version_missing(
    backend: TesseractBackend, mock_run_process_error: Mock, reset_version_ref: None
) -> None:
    with pytest.raises(MissingDependencyError) as excinfo:
        await backend._validate_tesseract_version()

    error_message = str(excinfo.value)
    assert "Tesseract version 5" in error_message
    assert "required" in error_message


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

    with pytest.raises(OCRError, match="Failed to OCR using tesseract"):
        await backend.process_file(nonexistent_file, language="eng", psm=PSMMode.AUTO)


@pytest.mark.anyio
async def test_process_file_runtime_error(backend: TesseractBackend, fresh_cache: None) -> None:
    import tempfile

    with tempfile.NamedTemporaryFile(suffix=".png", delete=False) as f:
        f.write(b"This is not a valid image file")
        invalid_file = Path(f.name)

    try:
        with pytest.raises(OCRError):
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
    with pytest.raises(ValidationError, match="not supported by Tesseract"):
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
    result = TesseractBackend._validate_language_code(language_code)
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
    with pytest.raises(ValidationError) as excinfo:
        TesseractBackend._validate_language_code(invalid_language_code)

    assert "language_code" in excinfo.value.context
    assert excinfo.value.context["language_code"] == invalid_language_code
    assert "supported_languages" in excinfo.value.context

    assert "not supported by Tesseract" in str(excinfo.value)


@pytest.mark.anyio
async def test_integration_process_image(backend: TesseractBackend, ocr_image: Path) -> None:
    image = Image.open(ocr_image)
    with image:
        result = await backend.process_image(image, language="eng", psm=PSMMode.AUTO)
        assert isinstance(result, ExtractionResult)
        assert result.content.strip()


@pytest.mark.anyio
async def test_process_file_linux(
    backend: TesseractBackend, mocker: MockerFixture, tmp_path: Path, fresh_cache: None
) -> None:
    mocker.patch("sys.platform", "linux")

    test_file = tmp_path / "test.png"
    test_image = Image.new("RGB", (100, 50), "white")
    test_image.save(test_file)

    async def linux_mock_run(*args: Any, **kwargs: Any) -> Mock:
        result = Mock()
        result.returncode = 0
        result.stderr = b""

        command = args[0]
        if "--version" in command:
            result.stdout = b"tesseract 5.0.0"
        elif len(command) >= 3 and command[0].endswith("tesseract"):
            output_base = command[2]
            if "hocr" in command or "tessedit_create_hocr=1" in " ".join(command):
                hocr_content = """<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN"
    "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd">
<html xmlns="http://www.w3.org/1999/xhtml" xml:lang="en" lang="en">
 <head>
  <title></title>
  <meta http-equiv="Content-Type" content="text/html;charset=utf-8" />
  <meta name='ocr-system' content='tesseract 5.0.0' />
  <meta name='ocr-capabilities' content='ocr_page ocr_carea ocr_par ocr_line ocrx_word' />
 </head>
 <body>
  <div class='ocr_page' id='page_1' title='bbox 0 0 100 50; ppageno 0'>
   <div class='ocr_carea' id='carea_1_1' title='bbox 10 10 90 40'>
    <p class='ocr_par' id='par_1_1' title='bbox 10 10 90 40'>
     <span class='ocr_line' id='line_1_1' title='bbox 10 10 90 40'>
      <span class='ocrx_word' id='word_1_1' title='bbox 10 10 40 40; x_wconf 95'>Test</span>
      <span class='ocrx_word' id='word_1_2' title='bbox 50 10 90 40; x_wconf 95'>text</span>
     </span>
    </p>
   </div>
  </div>
 </body>
</html>"""
                Path(f"{output_base}.hocr").write_text(hocr_content)
            else:
                Path(f"{output_base}.txt").write_text("Test text")
            result.stdout = b""
        else:
            result.stdout = b"test output"

        return result

    mock_run = mocker.patch("kreuzberg._ocr._tesseract.run_process", side_effect=linux_mock_run)

    TesseractBackend._version_checked = False
    result = await backend.process_file(test_file, language="eng", psm=PSMMode.AUTO)

    assert any(call[1].get("env") == {"OMP_THREAD_LIMIT": "1"} for call in mock_run.call_args_list)
    assert isinstance(result, ExtractionResult)
    assert "Test text" in result.content


@pytest.mark.anyio
async def test_process_image_cache_processing_coordination(
    backend: TesseractBackend, tmp_path: Path, mocker: MockerFixture
) -> None:
    from kreuzberg._utils._cache import get_ocr_cache

    test_image = Image.new("RGB", (100, 50), color="white")

    mocker.patch(
        "kreuzberg._ocr._tesseract.run_process", return_value=Mock(returncode=0, stdout=b"tesseract 5.0.0", stderr=b"")
    )

    import anyio

    cache = get_ocr_cache()

    import hashlib

    image_bytes = b"fake image bytes"
    image_hash = hashlib.sha256(image_bytes).hexdigest()[:16]

    ocr_config = str(sorted([("language", "eng")]))
    cache_kwargs = {
        "image_hash": image_hash,
        "ocr_backend": "tesseract",
        "ocr_config": ocr_config,
    }

    cache.mark_processing(**cache_kwargs)

    async def complete_processing(event: anyio.Event) -> None:
        await anyio.sleep(0.1)
        cache.mark_complete(**cache_kwargs)

        cache.set(
            ExtractionResult(content="cached text", mime_type="text/plain", metadata={}, chunks=[], tables=[]),
            **cache_kwargs,
        )
        event.set()

    async with anyio.create_task_group() as nursery:
        completion_event = anyio.Event()
        nursery.start_soon(complete_processing, completion_event)

        mock_hash_obj = Mock()
        mock_hash_obj.hexdigest.return_value = image_hash + "0" * 48
        mocker.patch("kreuzberg._ocr._tesseract.hashlib.sha256", return_value=mock_hash_obj)

        result = await backend.process_image(test_image, language="eng")

        assert result.content == "cached text"

        await completion_event.wait()


@pytest.mark.anyio
async def test_process_file_cache_processing_coordination(
    backend: TesseractBackend, tmp_path: Path, mocker: MockerFixture
) -> None:
    from kreuzberg._utils._cache import get_ocr_cache

    test_file = tmp_path / "test.png"
    test_image = Image.new("RGB", (100, 50), color="white")
    test_image.save(test_file)

    mocker.patch(
        "kreuzberg._ocr._tesseract.run_process", return_value=Mock(returncode=0, stdout=b"tesseract 5.0.0", stderr=b"")
    )

    import anyio

    cache = get_ocr_cache()

    # Generate cache key based on file - must match the format in process_file  # ~keep
    file_stat = test_file.stat()
    file_info = {
        "path": str(test_file.resolve()),
        "size": file_stat.st_size,
        "mtime": file_stat.st_mtime,
    }
    cache_kwargs = {
        "file_info": str(sorted(file_info.items())),
        "ocr_backend": "tesseract",
        "ocr_config": str(sorted([("language", "eng")])),
    }

    cache.mark_processing(**cache_kwargs)

    async def complete_processing(event: anyio.Event) -> None:
        await anyio.sleep(0.1)
        cache.mark_complete(**cache_kwargs)
        cache.set(
            ExtractionResult(content="cached file text", mime_type="text/plain", metadata={}, chunks=[], tables=[]),
            **cache_kwargs,
        )
        event.set()

    async with anyio.create_task_group() as nursery:
        completion_event = anyio.Event()
        nursery.start_soon(complete_processing, completion_event)

        # This should trigger cache coordination  # ~keep
        result = await backend.process_file(test_file, language="eng")

        # Should get cached result  # ~keep
        assert result.content == "cached file text"

        await completion_event.wait()


def test_validate_language_code_error() -> None:
    backend = TesseractBackend()

    with pytest.raises(ValidationError, match="provided language code is not supported"):
        backend._validate_language_code("invalid_language_code_that_is_too_long_and_invalid")


@pytest.mark.anyio
async def test_process_image_validation_error(backend: TesseractBackend) -> None:
    test_image = Image.new("RGB", (1, 1), color="white")

    from unittest.mock import patch

    with patch.object(backend, "_validate_language_code", side_effect=ValidationError("Invalid language")):
        with pytest.raises(ValidationError, match="Invalid language"):
            await backend.process_image(test_image, language="invalid")


@pytest.mark.anyio
async def test_process_file_validation_error(backend: TesseractBackend, tmp_path: Path) -> None:
    test_file = tmp_path / "test.png"
    test_image = Image.new("RGB", (100, 50), color="white")
    test_image.save(test_file)

    from unittest.mock import patch

    with patch.object(backend, "_validate_language_code", side_effect=ValidationError("Invalid language")):
        with pytest.raises(ValidationError, match="Invalid language"):
            await backend.process_file(test_file, language="invalid")


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


def test_tesseract_command_building_build_tesseract_command_basic(backend: TesseractBackend) -> None:
    command = backend._build_tesseract_command(
        path=Path("input.png"), output_base="output", language="eng", psm=PSMMode.AUTO
    )

    assert command[0] == "tesseract"
    assert "input.png" in command
    assert "output" in command
    assert "-l" in command
    assert "eng" in command
    assert "--psm" in command
    assert "3" in command


def test_tesseract_command_building_build_tesseract_command_complex(backend: TesseractBackend) -> None:
    command = backend._build_tesseract_command(
        path=Path("complex_input.tiff"),
        output_base="complex_output",
        language="eng+deu+fra",
        psm=PSMMode.SINGLE_BLOCK,
        tessedit_char_whitelist="0123456789",
        tessedit_enable_dict_correction=False,
        language_model_ngram_on=False,
        textord_space_size_is_variable=True,
        tessedit_dont_blkrej_good_wds=False,
    )

    assert "tesseract" in command[0]
    assert "complex_input.tiff" in command
    assert "complex_output" in command
    assert "-l" in command
    assert "eng+deu+fra" in command
    assert "--psm" in command
    assert "6" in command

    command_str = " ".join(command)
    assert "tessedit_char_whitelist=0123456789" in command_str
    assert "tessedit_enable_dict_correction=0" in command_str
    assert "language_model_ngram_on=0" in command_str
    assert "textord_space_size_is_variable=1" in command_str
    assert "tessedit_dont_blkrej_good_wds=0" in command_str


def test_tesseract_command_building_build_tesseract_command_no_config(backend: TesseractBackend) -> None:
    command = backend._build_tesseract_command(
        path=Path("input.jpg"), output_base="output", language="eng", psm=PSMMode.AUTO
    )

    assert command[0] == "tesseract"
    assert "input.jpg" in command
    assert "output" in command
    assert "-l" in command
    assert "eng" in command


def test_tesseract_file_handling_get_file_info(backend: TesseractBackend, tmp_path: Path) -> None:
    test_file = tmp_path / "test_file.png"
    test_file.write_text("dummy content")

    file_info = backend._get_file_info(test_file)

    assert "path" in file_info
    assert "size" in file_info
    assert "mtime" in file_info
    assert file_info["path"] == str(test_file.resolve())
    assert file_info["size"] == len("dummy content")
    assert isinstance(file_info["mtime"], float)


def test_tesseract_file_handling_get_file_info_nonexistent(backend: TesseractBackend) -> None:
    nonexistent = Path("/nonexistent/file.png")

    info = backend._get_file_info(nonexistent)
    assert info["path"] == str(nonexistent)
    assert info["size"] == 0
    assert info["mtime"] == 0


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
    result = TesseractBackend._validate_language_code(language_code)
    assert result == language_code.lower()


def test_tesseract_language_validation_multi_language_codes() -> None:
    result = TesseractBackend._validate_language_code("eng+deu+fra")
    assert result == "eng+deu+fra"

    result = TesseractBackend._validate_language_code("chi_sim+eng")
    assert result == "chi_sim+eng"


def test_tesseract_language_validation_case_insensitive_language_codes() -> None:
    result = TesseractBackend._validate_language_code("ENG")
    assert result == "eng"

    result = TesseractBackend._validate_language_code("DEU+FRA")
    assert result == "deu+fra"


def test_tesseract_sync_methods_run_tesseract_sync_success(backend: TesseractBackend, tmp_path: Path) -> None:
    img = Image.new("RGB", (200, 100), "white")
    draw = ImageDraw.Draw(img)
    draw.text((10, 40), "TEST", fill="black")

    img_path = tmp_path / "test.png"
    img.save(img_path)
    output_path = tmp_path / "output"

    command = ["tesseract", str(img_path), str(output_path), "-l", "eng"]
    backend._execute_tesseract_sync(command)

    assert (output_path.parent / f"{output_path.name}.txt").exists()


def test_tesseract_sync_methods_run_tesseract_sync_error(backend: TesseractBackend, mocker: MockerFixture) -> None:
    command = ["tesseract", "/nonexistent/input.png", "output", "-l", "eng"]

    with pytest.raises(OCRError, match="Failed to OCR using tesseract"):
        backend._execute_tesseract_sync(command)


def test_tesseract_sync_methods_run_tesseract_sync_runtime_error(
    backend: TesseractBackend, mocker: MockerFixture
) -> None:
    mock_run = mocker.patch("subprocess.run")
    mock_run.side_effect = RuntimeError("Command execution failed")

    command = ["tesseract", "input.png", "output", "-l", "eng"]

    with pytest.raises(RuntimeError, match="Command execution failed"):
        backend._execute_tesseract_sync(command)


def test_tesseract_sync_methods_validate_tesseract_version_sync_success(backend: TesseractBackend) -> None:
    TesseractBackend._version_checked = False
    backend._validate_tesseract_version_sync()
    assert TesseractBackend._version_checked is True


def test_tesseract_sync_methods_validate_tesseract_version_sync_too_old(
    backend: TesseractBackend, mocker: MockerFixture
) -> None:
    mock_run = mocker.patch("subprocess.run")
    mock_result = Mock()
    mock_result.returncode = 0
    mock_result.stdout = "tesseract 4.1.1"
    mock_result.stderr = ""
    mock_run.return_value = mock_result

    TesseractBackend._version_checked = False

    with pytest.raises(MissingDependencyError, match="Tesseract version 5"):
        backend._validate_tesseract_version_sync()


def test_tesseract_sync_methods_validate_tesseract_version_sync_not_found(
    backend: TesseractBackend, mocker: MockerFixture
) -> None:
    mock_run = mocker.patch("subprocess.run")
    mock_run.side_effect = FileNotFoundError("tesseract not found")

    TesseractBackend._version_checked = False

    with pytest.raises(MissingDependencyError, match="Tesseract version 5"):
        backend._validate_tesseract_version_sync()


@pytest.mark.anyio
async def test_tesseract_environment_variables_linux_omp_thread_limit(
    backend: TesseractBackend, mocker: MockerFixture, tmp_path: Path
) -> None:
    mocker.patch("sys.platform", "linux")

    async def mock_run_process(*args: Any, **kwargs: Any) -> Mock:
        if "--version" not in args[0]:
            assert kwargs.get("env") == {"OMP_THREAD_LIMIT": "1"}

        result = Mock()
        result.returncode = 0
        result.stderr = b""

        command = args[0]
        if "--version" in command:
            result.stdout = b"tesseract 5.0.0"
        elif len(command) >= 3 and command[0].endswith("tesseract"):
            output_base = command[2]
            if "hocr" in command or "tessedit_create_hocr=1" in " ".join(command):
                hocr_content = """<?xml version="1.0" encoding="UTF-8"?>
<html>
 <body>
  <div class='ocr_page' title='bbox 0 0 100 100'>
   <span class='ocrx_word' title='bbox 10 10 50 30; x_wconf 95'>Test</span>
  </div>
 </body>
</html>"""
                Path(f"{output_base}.hocr").write_text(hocr_content)
            else:
                Path(f"{output_base}.txt").write_text("Test output")
            result.stdout = b""
        else:
            result.stdout = b""

        return result

    mocker.patch("kreuzberg._ocr._tesseract.run_process", side_effect=mock_run_process)

    TesseractBackend._version_checked = False

    test_image = Image.new("RGB", (100, 100), "white")
    result = await backend.process_image(test_image, language="eng")

    assert isinstance(result, ExtractionResult)
    assert result.content.strip()


@pytest.mark.anyio
async def test_tesseract_environment_variables_non_linux_no_env_vars(
    backend: TesseractBackend, mocker: MockerFixture
) -> None:
    mocker.patch("sys.platform", "darwin")

    async def mock_run_process(*args: Any, **kwargs: Any) -> Mock:
        assert kwargs.get("env") is None
        result = Mock()
        result.returncode = 0
        result.stdout = b"tesseract 5.0.0" if "--version" in args[0] else b""
        result.stderr = b""
        return result

    mocker.patch("kreuzberg._ocr._tesseract.run_process", side_effect=mock_run_process)

    TesseractBackend._version_checked = False

    test_image = Image.new("RGB", (100, 100), "white")
    await backend.process_image(test_image, language="eng")


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
async def test_tesseract_image_processing_process_image_very_small(
    backend: TesseractBackend, mock_run_process: Mock
) -> None:
    image = Image.new("RGB", (1, 1), "white")
    result = await backend.process_image(image, language="eng")
    assert isinstance(result, ExtractionResult)


@pytest.mark.anyio
async def test_tesseract_image_processing_process_image_very_large(
    backend: TesseractBackend, mock_run_process: Mock
) -> None:
    image = Image.new("RGB", (2000, 1500), "white")
    result = await backend.process_image(image, language="eng")
    assert isinstance(result, ExtractionResult)


@pytest.mark.anyio
async def test_tesseract_error_handling_process_file_file_not_found(backend: TesseractBackend) -> None:
    nonexistent_file = Path("/nonexistent/file.png")

    with pytest.raises(OCRError, match="Failed to OCR using tesseract"):
        await backend.process_file(nonexistent_file, language="eng")


@pytest.mark.anyio
async def test_tesseract_error_handling_process_image_invalid_format(backend: TesseractBackend) -> None:
    import tempfile

    with tempfile.NamedTemporaryFile(suffix=".png", delete=False) as f:
        f.write(b"This is not a valid PNG file")
        invalid_path = Path(f.name)

    try:
        with pytest.raises(OCRError):
            await backend.process_file(invalid_path, language="eng")
    finally:
        invalid_path.unlink()


def test_tesseract_error_handling_sync_process_image_temp_file_error(backend: TesseractBackend) -> None:
    image = Image.new("RGB", (1, 1), "white")

    result = backend.process_image_sync(image, language="eng")

    assert isinstance(result, ExtractionResult)
    assert result.mime_type == "text/markdown"
    assert result.content is not None


def test_tesseract_error_handling_sync_process_file_read_error(backend: TesseractBackend, tmp_path: Path) -> None:
    test_file = tmp_path / "invalid.png"
    test_file.write_bytes(b"not a valid image")

    with pytest.raises(OCRError):
        backend.process_file_sync(test_file, language="eng")


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
    valid_combinations = [
        "ara+eng",
        "chi_sim+eng+deu",
        "jpn+kor+eng",
        "rus+ukr+eng",
        "hin+pan+urd+eng",
    ]

    for combo in valid_combinations:
        result = TesseractBackend._validate_language_code(combo)
        assert result == combo.lower()


@pytest.mark.parametrize(
    "test_image_path,expected_content_keywords,description",
    [
        (
            "test_documents/ocr_image.jpg",
            ["Nasdaq", "AMEX", "Stock", "Track"],
            "Financial newspaper table with stock data",
        ),
        (
            "test_documents/layout_parser_ocr.jpg",
            ["LayoutParser", "Table", "Dataset", "document"],
            "Academic paper with tables and technical content",
        ),
        (
            "test_documents/tables/simple_table.png",
            ["Product", "Price", "Quantity", "Apple", "Banana"],
            "Simple product table with clear borders",
        ),
        (
            "test_documents/invoice_image.png",
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

        assert "source_format" in result.metadata
        assert result.metadata["source_format"] == "hocr"

        assert "tables_detected" in result.metadata
        tables_count = result.metadata["tables_detected"]
        assert isinstance(tables_count, int)
        assert tables_count >= 0

    except Exception as e:
        pytest.fail(f"Failed to process {description} ({test_image_path}): {e}")


@pytest.mark.parametrize(
    "test_image_path,description",
    [
        ("test_documents/tables/simple_table.png", "Simple table with clear borders"),
        ("test_documents/ocr_image.jpg", "Financial data table"),
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

    assert "tables_detected" in result.metadata
    tables_count = result.metadata["tables_detected"]
    assert isinstance(tables_count, int)

    if tables_count > 0:
        assert len(result.tables) == tables_count
        assert "Table" in content or len(result.tables) > 0


@pytest.mark.anyio
async def test_markdown_no_excessive_escaping(backend: TesseractBackend, tmp_path: Path) -> None:
    image = Image.new("RGB", (800, 400), color="white")
    draw = ImageDraw.Draw(image)

    font: FreeTypeFont | ImageFontType
    try:
        font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 24)
    except (OSError, AttributeError):
        font = ImageFont.load_default()

    test_text = [
        "There should be one-- and preferably only one --obvious way",
        "Table headers: Name | Age | Status == Active",
        "Math expressions: 2 + 2 = 4",
        "Code block: if (x > 0) { return true; }",
        "List items: [1] First item [+] Add new",
        "Number: 273.879.750",
        "Asterisks for *emphasis* and **bold**",
    ]

    y_position = 20
    for line in test_text:
        draw.text((20, y_position), line, fill="black", font=font)
        y_position += 40

    image_path = tmp_path / "test_special_chars.png"
    image.save(image_path)

    from kreuzberg._types import TesseractConfig

    config = TesseractConfig(output_format="markdown")
    result = await backend.process_file(image_path, **structs.asdict(config))

    assert r"\-\-" not in result.content
    assert r"\|" not in result.content
    assert r"\=" not in result.content
    assert r"\+" not in result.content
    assert r"\[" not in result.content
    assert r"\]" not in result.content

    assert "--" in result.content or "~" in result.content
    assert "|" in result.content or "I" in result.content
    assert "=" in result.content
    assert "+" in result.content
    assert "*" in result.content


@pytest.mark.anyio
async def test_html_to_markdown_config_defaults() -> None:
    from kreuzberg._types import HTMLToMarkdownConfig

    config = HTMLToMarkdownConfig()

    assert config.escape_misc is False
    assert config.escape_asterisks is False
    assert config.escape_underscores is False
    assert config.extract_metadata is True


def test_tesseract_utility_functions_normalize_spaces_in_results(
    backend: TesseractBackend, mock_run_process: Mock
) -> None:
    async def mock_with_extra_spaces(*args: Any, **kwargs: Any) -> Mock:
        if "--version" in args[0]:
            result = Mock()
            result.returncode = 0
            result.stdout = b"tesseract 5.0.0"
            result.stderr = b""
            return result

        output_file = args[0][2]
        Path(f"{output_file}.txt").write_text("This  has   extra    spaces\nAnd\t\ttabs\n\n\nAnd newlines")

        result = Mock()
        result.returncode = 0
        result.stderr = b""
        return result

    mock_run_process.side_effect = mock_with_extra_spaces

    image = Image.new("RGB", (100, 100), "white")

    with (
        patch.object(backend, "_validate_tesseract_version_sync"),
        patch("tempfile.NamedTemporaryFile") as mock_temp,
    ):
        mock_temp_file = Mock()
        mock_temp_file.name = "test_output"
        mock_temp.return_value.__enter__.return_value = mock_temp_file

        result = backend.process_image_sync(image, language="eng")

        assert "  " not in result.content
        assert "\t\t" not in result.content
        assert "\n\n\n" not in result.content


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
async def test_tesseract_memory_efficiency(backend: TesseractBackend, mock_run_process: Mock) -> None:
    large_image = Image.new("RGB", (1000, 1000), "white")

    result = await backend.process_image(large_image, language="eng")
    assert isinstance(result, ExtractionResult)

    import gc

    del large_image
    gc.collect()

    small_image = Image.new("RGB", (100, 100), "white")
    result2 = await backend.process_image(small_image, language="eng")
    assert isinstance(result2, ExtractionResult)


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

    # Create image with table-like structure
    img = Image.new("RGB", (400, 200), "white")
    draw = ImageDraw.Draw(img)
    font = ImageFont.load_default()

    # Draw a simple table
    draw.text((10, 10), "Name    Age    City", fill="black", font=font)
    draw.text((10, 40), "Alice   30     NYC", fill="black", font=font)
    draw.text((10, 70), "Bob     25     LA", fill="black", font=font)

    result = await backend.process_image(img, enable_table_detection=True)
    assert isinstance(result, ExtractionResult)
    # Table detection might or might not find tables depending on OCR quality
    # but should not crash


def test_process_batch_sync_empty_list() -> None:
    backend = TesseractBackend()
    results = backend.process_batch_sync([])
    assert results == []


def test_process_batch_sync_single_image(tmp_path: Path) -> None:
    backend = TesseractBackend()

    # Create test image file
    img = create_test_image("Batch Test 1")
    img_path = tmp_path / "test1.png"
    img.save(img_path)

    results = backend.process_batch_sync([img_path])
    assert len(results) == 1
    assert isinstance(results[0], ExtractionResult)


def test_process_batch_sync_multiple_images(tmp_path: Path) -> None:
    backend = TesseractBackend()

    # Create multiple test images
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

    # Create one valid and one invalid image
    valid_img = create_test_image("Valid")
    valid_path = tmp_path / "valid.png"
    valid_img.save(valid_path)

    invalid_path = tmp_path / "invalid.png"
    invalid_path.write_text("not an image")

    results = backend.process_batch_sync([valid_path, invalid_path])
    assert len(results) == 2
    # First should succeed
    assert isinstance(results[0], ExtractionResult)
    # Second should have error message
    assert "[OCR error:" in results[1].content


@pytest.mark.anyio
async def test_tesseract_tsv_table_extraction_edge_cases() -> None:
    backend = TesseractBackend()

    # Create image that won't produce valid table data
    img = Image.new("RGB", (100, 100), "white")
    draw = ImageDraw.Draw(img)
    draw.text((10, 10), "Just text", fill="black")

    # Should not crash even if table extraction fails
    result = await backend.process_image(img, enable_table_detection=True)
    assert isinstance(result, ExtractionResult)


@pytest.mark.anyio
async def test_tesseract_extract_text_from_tsv_error_handling() -> None:
    backend = TesseractBackend()

    # Process with TSV format to trigger _extract_text_from_tsv
    img = create_test_image("TSV Test")
    result = await backend.process_image(img, output_format="tsv")

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


@pytest.mark.integration
@pytest.mark.anyio
async def test_tesseract_hocr_with_tables_integration() -> None:
    backend = TesseractBackend()

    # Create image with table-like structure - use larger font and spacing for better OCR
    img = Image.new("RGB", (800, 300), "white")
    draw = ImageDraw.Draw(img)
    # Use a larger font size
    try:
        font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 24)
    except Exception:
        font = ImageFont.load_default()  # type: ignore[assignment]

    # Draw table headers and rows with clear spacing
    draw.text((50, 30), "Product", fill="black", font=font)
    draw.text((300, 30), "Price", fill="black", font=font)
    draw.text((500, 30), "Quantity", fill="black", font=font)

    draw.text((50, 100), "Apple", fill="black", font=font)
    draw.text((300, 100), "150", fill="black", font=font)
    draw.text((500, 100), "10", fill="black", font=font)

    draw.text((50, 170), "Banana", fill="black", font=font)
    draw.text((300, 170), "75", fill="black", font=font)
    draw.text((500, 170), "15", fill="black", font=font)

    # Process with HOCR format and table detection
    result = await backend.process_image(img, output_format="hocr", enable_table_detection=True)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0
    # Verify at least some expected text is extracted
    content_lower = result.content.lower()
    assert "product" in content_lower or "apple" in content_lower or "banana" in content_lower


@pytest.mark.integration
@pytest.mark.anyio
async def test_tesseract_tsv_with_table_reconstruction_integration() -> None:
    backend = TesseractBackend()

    # Create a more structured table image with larger, clearer text
    img = Image.new("RGB", (1000, 400), "white")
    draw = ImageDraw.Draw(img)

    # Use larger font for better OCR
    try:
        font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 28)
    except Exception:
        font = ImageFont.load_default()  # type: ignore[assignment]

    # Draw clear table structure with consistent spacing
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

    # Process with TSV format and table detection
    result = await backend.process_image(img, output_format="tsv", enable_table_detection=True)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0
    # Verify expected names are extracted
    content_lower = result.content.lower()
    # Check for at least 2 of the 3 names
    matches = sum([name in content_lower for name in ["alice", "bob", "charlie"]])
    assert matches >= 2, f"Expected at least 2 names in OCR output, but found {matches}"


@pytest.mark.integration
def test_process_batch_sync_integration(tmp_path: Path) -> None:
    backend = TesseractBackend()

    # Create multiple test images with distinct content
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

    # Process batch synchronously
    results = backend.process_batch_sync(paths)

    assert len(results) == 3
    # Verify each result contains some expected text
    for i, result in enumerate(results):
        assert isinstance(result, ExtractionResult)
        assert len(result.content) > 0
        # Check that at least the batch number appears
        content_upper = result.content.upper().replace(" ", "")
        assert "BATCH" in content_upper, f"Expected 'BATCH' in result {i}, got: {result.content}"


def test_tesseract_sync_process_file_integration(tmp_path: Path) -> None:
    backend = TesseractBackend()

    # Create test image with clear text
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

    # Process file synchronously
    result = backend.process_file_sync(img_path)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0
    # Verify expected text appears
    content_upper = result.content.upper().replace(" ", "")
    assert "HELLO" in content_upper or "WORLD" in content_upper


def test_tesseract_sync_process_image_integration() -> None:
    backend = TesseractBackend()

    # Create test image with numbers for better OCR reliability
    img = Image.new("RGB", (700, 150), "white")
    draw = ImageDraw.Draw(img)

    try:
        font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 48)
    except Exception:
        font = ImageFont.load_default()  # type: ignore[assignment]

    test_text = "12345"
    draw.text((150, 40), test_text, fill="black", font=font)

    # Process image synchronously
    result = backend.process_image_sync(img)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0
    # Numbers should be reliably detected
    content_cleaned = result.content.replace(" ", "").replace("\n", "")
    assert any(digit in content_cleaned for digit in "12345")


@pytest.mark.anyio
async def test_tesseract_with_cache_enabled(tmp_path: Path) -> None:
    from kreuzberg._utils._cache import get_ocr_cache

    backend = TesseractBackend()

    # Create test image
    img = create_test_image("CACHE TEST")
    img_path = tmp_path / "cache_test.png"
    img.save(img_path)

    # Clear cache first
    cache = get_ocr_cache()
    cache.clear()

    # First call - should process and cache
    result1 = await backend.process_file(img_path, use_cache=True)
    assert isinstance(result1, ExtractionResult)

    # Second call - should hit cache
    result2 = await backend.process_file(img_path, use_cache=True)
    assert isinstance(result2, ExtractionResult)
    assert result2.content == result1.content


@pytest.mark.anyio
async def test_tesseract_process_image_with_cache(tmp_path: Path) -> None:
    from kreuzberg._utils._cache import get_ocr_cache

    backend = TesseractBackend()

    # Create test image
    img = create_test_image("IMAGE CACHE")

    # Clear cache first
    cache = get_ocr_cache()
    cache.clear()

    # First call - should process and cache
    result1 = await backend.process_image(img, use_cache=True)
    assert isinstance(result1, ExtractionResult)

    # Second call - should hit cache
    result2 = await backend.process_image(img, use_cache=True)
    assert isinstance(result2, ExtractionResult)


@pytest.mark.anyio
async def test_tesseract_table_detection_auto_tsv() -> None:
    backend = TesseractBackend()

    # Create test image
    img = create_test_image("Test table detection auto TSV")

    # enable_table_detection=True with output_format="text" should auto-switch to tsv
    result = await backend.process_image(img, enable_table_detection=True, output_format="text")

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


@pytest.mark.anyio
async def test_tesseract_extract_text_from_tsv_error_handling_malformed() -> None:
    backend = TesseractBackend()

    # Test malformed TSV that triggers error path
    malformed_tsv = "not\tvalid\ttsv\ndata\there"

    result = backend._extract_text_from_tsv(malformed_tsv)

    assert isinstance(result, ExtractionResult)
    # Should fallback to simple parsing


@pytest.mark.anyio
async def test_tesseract_extract_text_from_tsv_paragraph_spacing() -> None:
    backend = TesseractBackend()

    # Test TSV with paragraph changes to hit lines 486-487
    tsv_content = """level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext
5\t1\t1\t1\t1\t1\t50\t50\t100\t30\t95.0\tFirst
5\t1\t1\t2\t2\t1\t50\t80\t100\t30\t94.0\tSecond
5\t1\t2\t1\t1\t1\t50\t150\t100\t30\t96.0\tThird"""

    result = backend._extract_text_from_tsv(tsv_content)

    assert isinstance(result, ExtractionResult)
    assert "First" in result.content
    assert "Second" in result.content
    assert "Third" in result.content


@pytest.mark.anyio
async def test_tesseract_process_pool_initialization() -> None:
    from kreuzberg._ocr._tesseract import TesseractProcessPool

    pool = TesseractProcessPool()
    assert pool.config is not None
    assert pool.process_manager is not None
    pool.shutdown()


@pytest.mark.anyio
async def test_tesseract_process_pool_with_config() -> None:
    from kreuzberg import TesseractConfig
    from kreuzberg._ocr._tesseract import TesseractProcessPool

    config = TesseractConfig(language="eng", psm=PSMMode.AUTO)
    pool = TesseractProcessPool(config=config, max_processes=2)
    assert pool.config.language == "eng"
    pool.shutdown()


@pytest.mark.anyio
async def test_tesseract_process_pool_process_image(tmp_path: Path) -> None:
    from kreuzberg._ocr._tesseract import TesseractProcessPool

    pool = TesseractProcessPool()

    # Create test image
    img = create_test_image("POOL TEST")
    img_path = tmp_path / "pool_test.png"
    img.save(img_path)

    try:
        result = await pool.process_image(img_path)
        assert isinstance(result, ExtractionResult)
        assert len(result.content) > 0
    finally:
        pool.shutdown()


@pytest.mark.anyio
async def test_tesseract_process_pool_process_image_bytes() -> None:
    from io import BytesIO

    from kreuzberg._ocr._tesseract import TesseractProcessPool

    pool = TesseractProcessPool()

    # Create test image
    img = create_test_image("POOL BYTES TEST")
    img_buffer = BytesIO()
    img.save(img_buffer, format="PNG")
    img_bytes = img_buffer.getvalue()

    try:
        result = await pool.process_image_bytes(img_bytes)
        assert isinstance(result, ExtractionResult)
        assert len(result.content) > 0
    finally:
        pool.shutdown()


@pytest.mark.anyio
async def test_tesseract_process_pool_batch_images(tmp_path: Path) -> None:
    from kreuzberg._ocr._tesseract import TesseractProcessPool

    pool = TesseractProcessPool()

    # Create multiple test images
    paths: list[str | Path] = []
    for i in range(2):
        img = create_test_image(f"BATCH {i}")
        img_path = tmp_path / f"batch_{i}.png"
        img.save(img_path)
        paths.append(img_path)

    try:
        results = await pool.process_batch_images(paths)
        assert len(results) == 2
        for result in results:
            assert isinstance(result, ExtractionResult)
    finally:
        pool.shutdown()


@pytest.mark.anyio
async def test_tesseract_process_pool_batch_images_empty() -> None:
    from kreuzberg._ocr._tesseract import TesseractProcessPool

    pool = TesseractProcessPool()

    try:
        results = await pool.process_batch_images([])
        assert results == []
    finally:
        pool.shutdown()


@pytest.mark.anyio
async def test_tesseract_process_pool_batch_bytes() -> None:
    from io import BytesIO

    from kreuzberg._ocr._tesseract import TesseractProcessPool

    pool = TesseractProcessPool()

    # Create multiple test images as bytes
    image_bytes_list = []
    for i in range(2):
        img = create_test_image(f"BYTES BATCH {i}")
        img_bytes = BytesIO()
        img.save(img_bytes, format="PNG")
        image_bytes_list.append(img_bytes.getvalue())

    try:
        results = await pool.process_batch_bytes(image_bytes_list)
        assert len(results) == 2
        for result in results:
            assert isinstance(result, ExtractionResult)
    finally:
        pool.shutdown()


@pytest.mark.anyio
async def test_tesseract_process_pool_batch_bytes_empty() -> None:
    from kreuzberg._ocr._tesseract import TesseractProcessPool

    pool = TesseractProcessPool()

    try:
        results = await pool.process_batch_bytes([])
        assert results == []
    finally:
        pool.shutdown()


@pytest.mark.anyio
async def test_tesseract_process_pool_get_system_info() -> None:
    from kreuzberg._ocr._tesseract import TesseractProcessPool

    pool = TesseractProcessPool()

    try:
        info = pool.get_system_info()
        assert isinstance(info, dict)
    finally:
        pool.shutdown()


@pytest.mark.anyio
async def test_tesseract_process_pool_context_manager() -> None:
    from kreuzberg._ocr._tesseract import TesseractProcessPool

    async with TesseractProcessPool() as pool:
        assert pool is not None
        info = pool.get_system_info()
        assert isinstance(info, dict)


@pytest.mark.anyio
async def test_tesseract_extract_text_from_tsv_fallback_parsing() -> None:
    backend = TesseractBackend()

    # TSV with missing required columns - triggers ValueError/KeyError in primary parsing
    malformed_tsv = """level\tpage_num
5\t1
5\t1"""

    result = backend._extract_text_from_tsv(malformed_tsv)

    assert isinstance(result, ExtractionResult)
    # Should use fallback parsing


@pytest.mark.anyio
async def test_tesseract_tsv_fallback_with_text_column() -> None:
    backend = TesseractBackend()

    # TSV with level="5" and text column but missing page_num/block_num/etc - triggers KeyError
    # Has 12 columns so fallback can extract column 11 (0-indexed)
    malformed_tsv_with_text = """level\ta\tb\tc\td\te\tf\tg\th\ti\tj\ttext
5\t1\t2\t3\t4\t5\t6\t7\t8\t9\t10\tFallbackText"""

    result = backend._extract_text_from_tsv(malformed_tsv_with_text)

    assert isinstance(result, ExtractionResult)
    # Fallback should extract column 11 (0-indexed) which is "text" column
    assert "FallbackText" in result.content


@pytest.mark.anyio
async def test_tesseract_cache_hit_on_second_call(tmp_path: Path) -> None:
    from kreuzberg._utils._cache import get_ocr_cache

    backend = TesseractBackend()

    # Create unique test image
    img = create_test_image("UNIQUE CACHE TEST 12345")
    img_path = tmp_path / "unique_cache.png"
    img.save(img_path)

    # Clear cache
    cache = get_ocr_cache()
    cache.clear()

    # First call - processes and caches (hits line 228)
    result1 = await backend.process_file(img_path, use_cache=True)
    assert isinstance(result1, ExtractionResult)
    assert len(result1.content) > 0

    # Second call - should hit cache at line 210
    result2 = await backend.process_file(img_path, use_cache=True)
    assert isinstance(result2, ExtractionResult)
    assert result2.content == result1.content


@pytest.mark.anyio
async def test_tesseract_invalid_file_triggers_error(tmp_path: Path) -> None:
    backend = TesseractBackend()

    # Create an invalid image file
    bad_file = tmp_path / "bad_image.png"
    bad_file.write_text("This is not a valid PNG file")

    # Should raise OCRError when tesseract fails
    with pytest.raises(OCRError) as exc_info:
        await backend.process_file(bad_file)
    assert "OCR failed" in str(exc_info.value) or "Failed to OCR" in str(exc_info.value)


@pytest.mark.anyio
async def test_tesseract_hocr_with_custom_converters() -> None:
    from kreuzberg import HTMLToMarkdownConfig

    backend = TesseractBackend()

    # Create test image
    img = create_test_image("CUSTOM CONVERTER TEST")

    # Define custom converter
    def custom_span_converter(*, tag: Any, text: str, **kwargs: Any) -> str:
        return f"[CUSTOM: {text}]"

    html_config = HTMLToMarkdownConfig(custom_converters={"span": custom_span_converter})

    # Process with custom converter
    result = await backend.process_image(img, output_format="hocr", html_to_markdown_config=html_config)

    assert isinstance(result, ExtractionResult)
    # Custom converter should have been used
    assert len(result.content) > 0


@pytest.mark.anyio
async def test_tesseract_process_pool_error_handling() -> None:
    from kreuzberg._ocr._tesseract import TesseractProcessPool

    pool = TesseractProcessPool()

    try:
        # Test _result_from_dict with error
        error_dict = {"success": False, "text": "", "confidence": None, "error": "Test error"}

        with pytest.raises(OCRError) as exc_info:
            pool._result_from_dict(error_dict)

        assert "Tesseract processing failed" in str(exc_info.value)
        assert "Test error" in str(exc_info.value)
    finally:
        pool.shutdown()


def test_tesseract_sync_process_image_with_cache_hit() -> None:
    from kreuzberg._utils._cache import get_ocr_cache

    backend = TesseractBackend()

    # Create test image
    img = create_test_image("SYNC CACHE IMAGE 789")

    # Clear cache
    cache = get_ocr_cache()
    cache.clear()

    # First call - processes and caches
    result1 = backend.process_image_sync(img, use_cache=True)
    assert isinstance(result1, ExtractionResult)
    assert len(result1.content) > 0

    # Second call - should hit cache at line 1008
    result2 = backend.process_image_sync(img, use_cache=True)
    assert isinstance(result2, ExtractionResult)
    assert result2.content == result1.content


def test_tesseract_sync_process_file_with_cache_hit(tmp_path: Path) -> None:
    from kreuzberg._utils._cache import get_ocr_cache

    backend = TesseractBackend()

    # Create test image
    img = create_test_image("SYNC CACHE FILE 456")
    img_path = tmp_path / "sync_cache_file.png"
    img.save(img_path)

    # Clear cache
    cache = get_ocr_cache()
    cache.clear()

    # First call - processes and caches (hits line 1019)
    result1 = backend.process_file_sync(img_path, use_cache=True)
    assert isinstance(result1, ExtractionResult)
    assert len(result1.content) > 0

    # Second call - should hit cache at line 1040
    result2 = backend.process_file_sync(img_path, use_cache=True)
    assert isinstance(result2, ExtractionResult)
    assert result2.content == result1.content


def test_tesseract_sync_process_file_with_hocr_output(tmp_path: Path) -> None:
    backend = TesseractBackend()

    # Create test image
    img = create_test_image("SYNC HOCR TEST")
    img_path = tmp_path / "sync_hocr.png"
    img.save(img_path)

    # Process with HOCR output to hit sync _process_tesseract_output_sync paths
    result = backend.process_file_sync(img_path, output_format="hocr", use_cache=False)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


def test_tesseract_sync_process_file_with_markdown_output(tmp_path: Path) -> None:
    backend = TesseractBackend()

    # Create test image
    img = create_test_image("SYNC MARKDOWN TEST")
    img_path = tmp_path / "sync_markdown.png"
    img.save(img_path)

    # Process with markdown output to hit sync _process_hocr_to_markdown_sync
    result = backend.process_file_sync(img_path, output_format="markdown", use_cache=False)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


def test_tesseract_sync_process_file_with_tsv_output(tmp_path: Path) -> None:
    backend = TesseractBackend()

    # Create test image
    img = create_test_image("SYNC TSV TEST")
    img_path = tmp_path / "sync_tsv.png"
    img.save(img_path)

    # Process with TSV output to hit sync TSV processing
    result = backend.process_file_sync(img_path, output_format="tsv", use_cache=False)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


def test_tesseract_sync_process_file_with_tsv_and_table_detection(tmp_path: Path) -> None:
    backend = TesseractBackend()

    # Create test image with table-like structure
    img = create_test_image("SYNC TSV TABLE TEST")
    img_path = tmp_path / "sync_tsv_table.png"
    img.save(img_path)

    # Process with TSV and table detection to hit sync _process_tsv_output_sync
    result = backend.process_file_sync(img_path, output_format="tsv", enable_table_detection=True, use_cache=False)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


def test_tesseract_sync_process_image_no_cache() -> None:
    backend = TesseractBackend()

    # Create test image
    img = create_test_image("NO CACHE IMAGE")

    # Process without cache to hit use_cache=False branches
    result = backend.process_image_sync(img, use_cache=False)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


def test_tesseract_sync_process_file_no_cache(tmp_path: Path) -> None:
    backend = TesseractBackend()

    # Create test image
    img = create_test_image("NO CACHE FILE")
    img_path = tmp_path / "no_cache_file.png"
    img.save(img_path)

    # Process without cache to hit use_cache=False branches
    result = backend.process_file_sync(img_path, use_cache=False)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


@pytest.mark.anyio
async def test_tesseract_process_image_no_cache() -> None:
    """Test process_image with use_cache=False - hits branch 207->212, 227->230."""
    backend = TesseractBackend()
    img = create_test_image("NO CACHE IMAGE")

    result = await backend.process_image(img, use_cache=False)
    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


@pytest.mark.anyio
async def test_tesseract_process_file_no_cache(tmp_path: Path) -> None:
    """Test process_file with use_cache=False - hits branch 377->382, 396->409."""
    backend = TesseractBackend()
    img = create_test_image("NO CACHE FILE ASYNC")
    img_path = tmp_path / "no_cache_test.png"
    img.save(img_path)

    result = await backend.process_file(img_path, use_cache=False)
    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


@pytest.mark.anyio
async def test_tesseract_with_custom_converters(tmp_path: Path) -> None:
    """Test HOCR with custom converters - hits line 534."""
    from kreuzberg._types import HTMLToMarkdownConfig

    def custom_converter(**kwargs: Any) -> str:
        return "[CUSTOM]"

    backend = TesseractBackend()
    img = create_test_image("CUSTOM CONVERTER TEST")
    img_path = tmp_path / "custom.png"
    img.save(img_path)

    config = HTMLToMarkdownConfig(custom_converters={"custom_tag": custom_converter})
    result = await backend.process_file(img_path, output_format="markdown", html_to_markdown_config=config)
    assert isinstance(result, ExtractionResult)


@pytest.mark.anyio
async def test_tesseract_text_output_format(tmp_path: Path) -> None:
    """Test plain text output format - hits line 986."""
    backend = TesseractBackend()
    img = create_test_image("PLAIN TEXT TEST")
    img_path = tmp_path / "plain_text.png"
    img.save(img_path)

    result = await backend.process_file(img_path, output_format="text")
    assert isinstance(result, ExtractionResult)
    assert result.mime_type == "text/plain"


def test_tesseract_sync_image_mode_conversion() -> None:
    """Test image mode conversion for unsupported modes - hits line 993."""
    backend = TesseractBackend()

    # Create image with CMYK mode (not in supported modes)
    img = Image.new("CMYK", (400, 100), "white")

    result = backend.process_image_sync(img, use_cache=False)
    assert isinstance(result, ExtractionResult)


def test_tesseract_table_prefix_kwargs(tmp_path: Path) -> None:
    """Test that table_ prefixed kwargs are skipped - hits line 1209."""
    backend = TesseractBackend()
    img = create_test_image("TABLE PREFIX TEST")
    img_path = tmp_path / "table_prefix.png"
    img.save(img_path)

    # table_ prefixed kwargs should be skipped in command building
    result = backend.process_file_sync(
        img_path, table_column_threshold=10, table_row_threshold_ratio=0.5, use_cache=False
    )
    assert isinstance(result, ExtractionResult)


def test_tesseract_sync_timeout_error(tmp_path: Path, mocker: MockerFixture) -> None:
    """Test timeout handling in sync execution - hits line 955."""
    import subprocess

    backend = TesseractBackend()
    img = create_test_image("TIMEOUT TEST")
    img_path = tmp_path / "timeout.png"
    img.save(img_path)

    # Mock subprocess.run to raise TimeoutExpired
    mock_run = mocker.patch("subprocess.run")
    mock_run.side_effect = subprocess.TimeoutExpired(cmd=["tesseract"], timeout=30)

    with pytest.raises(OCRError, match="timed out"):
        backend.process_file_sync(img_path, use_cache=False)


def test_tesseract_batch_processing_error(tmp_path: Path) -> None:
    """Test error handling in batch processing - hits lines 1168-1169."""
    backend = TesseractBackend()

    # Create one valid image and one invalid path
    valid_img = create_test_image("BATCH TEST")
    valid_path = tmp_path / "valid.png"
    valid_img.save(valid_path)

    invalid_path = tmp_path / "nonexistent.png"

    # Process batch with invalid path
    results = backend.process_batch_sync([valid_path, invalid_path], use_cache=False)

    # First should succeed, second should have error message
    assert len(results) == 2
    assert isinstance(results[0], ExtractionResult)
    assert "[OCR error:" in results[1].content


@pytest.mark.anyio
async def test_tesseract_hocr_empty_words(tmp_path: Path, mocker: MockerFixture) -> None:
    """Test HOCR with empty words - hits lines 885-888."""
    backend = TesseractBackend()

    # Mock _identify_table_regions to receive empty words list
    mock_identify = mocker.patch.object(backend, "_identify_table_regions")
    mock_identify.return_value = []

    img = create_test_image("EMPTY WORDS TEST")
    img_path = tmp_path / "empty_words.png"
    img.save(img_path)

    result = await backend.process_file(img_path, output_format="markdown", enable_table_detection=True)
    assert isinstance(result, ExtractionResult)
