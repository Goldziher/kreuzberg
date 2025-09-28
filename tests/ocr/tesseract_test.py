from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING, Any
from unittest.mock import Mock, patch

import anyio
import pytest
from PIL import Image

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
            elif "hocr" in command:
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
    from PIL import ImageDraw

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
    from PIL import ImageDraw

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
            if "hocr" in command:
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

    cache.mark_processing(image_hash=image_hash, config="test_config")

    async def complete_processing(event: anyio.Event) -> None:
        await anyio.sleep(0.1)
        cache.mark_complete(image_hash=image_hash, config="test_config")

        cache.set(
            ExtractionResult(content="cached text", mime_type="text/plain", metadata={}, chunks=[], tables=[]),
            image_hash=image_hash,
            config="test_config",
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
    from PIL import ImageDraw

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
    from PIL import ImageDraw

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
            if "hocr" in command:
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
    from PIL import ImageDraw, ImageFont

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
    result = await backend.process_file(image_path, **config.to_dict())

    assert r"\-\-" not in result.content
    assert r"\|" not in result.content
    assert r"\=" not in result.content
    assert r"\+" not in result.content
    assert r"\[" not in result.content
    assert r"\]" not in result.content

    assert "--" in result.content
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
    from PIL import ImageDraw

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
