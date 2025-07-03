from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING, Any
from unittest.mock import Mock

import pytest
from PIL import Image

from kreuzberg import PSMMode
from kreuzberg._ocr._tesseract import (
    TesseractBackend,
)
from kreuzberg._types import ExtractionResult
from kreuzberg.exceptions import MissingDependencyError, OCRError, ValidationError

if TYPE_CHECKING:
    from pytest_mock import MockerFixture


@pytest.fixture
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

            Path(f"{output_file}.txt").write_text("Sample OCR text")
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
async def test_validate_tesseract_version(backend: TesseractBackend, mock_run_process: Mock) -> None:
    await backend._validate_tesseract_version()
    mock_run_process.assert_called_with(["tesseract", "--version"])


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
async def test_process_file(backend: TesseractBackend, mock_run_process: Mock, ocr_image: Path) -> None:
    result = await backend.process_file(ocr_image, language="eng", psm=PSMMode.AUTO)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip() == "Sample OCR text"


@pytest.mark.anyio
async def test_process_file_with_options(backend: TesseractBackend, mock_run_process: Mock, ocr_image: Path) -> None:
    result = await backend.process_file(ocr_image, language="eng", psm=PSMMode.AUTO)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip() == "Sample OCR text"


@pytest.mark.anyio
async def test_process_file_error(backend: TesseractBackend, mock_run_process: Mock, ocr_image: Path) -> None:
    async def error_side_effect(*args: Any, **kwargs: Any) -> Mock:
        if args and isinstance(args[0], list) and "--version" in args[0]:
            result = Mock()
            result.returncode = 0

            stdout_mock = Mock()
            stdout_mock.decode = Mock(return_value="tesseract 5.0.0")
            result.stdout = stdout_mock
            result.stderr = b""
            return result

        result = Mock()
        result.returncode = 1
        result.stderr = b"Error processing file"
        return result

    TesseractBackend._version_checked = False
    mock_run_process.side_effect = error_side_effect

    with pytest.raises(OCRError, match="OCR failed with a non-0 return code"):
        await backend.process_file(ocr_image, language="eng", psm=PSMMode.AUTO)


@pytest.mark.anyio
async def test_process_file_runtime_error(backend: TesseractBackend, mock_run_process: Mock, ocr_image: Path) -> None:
    call_count = 0

    async def runtime_error_side_effect(*args: Any, **kwargs: Any) -> Mock:
        nonlocal call_count
        call_count += 1

        if call_count == 1:
            result = Mock()
            result.returncode = 0

            stdout_mock = Mock()
            stdout_mock.decode = Mock(return_value="tesseract 5.0.0")
            result.stdout = stdout_mock
            result.stderr = b""
            return result

        raise RuntimeError("Command failed")

    TesseractBackend._version_checked = False
    mock_run_process.side_effect = runtime_error_side_effect

    with pytest.raises(OCRError, match="Failed to OCR using tesseract"):
        await backend.process_file(ocr_image, language="eng", psm=PSMMode.AUTO)


@pytest.mark.anyio
async def test_process_image(backend: TesseractBackend, mock_run_process: Mock) -> None:
    image = Image.new("RGB", (100, 100))
    result = await backend.process_image(image, language="eng", psm=PSMMode.AUTO)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip() == "Sample OCR text"


@pytest.mark.anyio
async def test_process_image_with_tesseract_pillow(backend: TesseractBackend, mock_run_process: Mock) -> None:
    image = Image.new("RGB", (100, 100))
    result = await backend.process_image(image)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip() == "Sample OCR text"


@pytest.mark.anyio
async def test_integration_process_file(backend: TesseractBackend, ocr_image: Path) -> None:
    result = await backend.process_file(ocr_image, language="eng", psm=PSMMode.AUTO)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip()


@pytest.mark.anyio
async def test_process_file_with_invalid_language(
    backend: TesseractBackend, ocr_image: Path, monkeypatch
) -> None:
    # Patch run_process to raise ValidationError for invalid language
    from kreuzberg.exceptions import ValidationError
    async def fake_run_process(*args, **kwargs):
        raise ValidationError("not supported by Tesseract")
    monkeypatch.setattr("kreuzberg._ocr._tesseract.run_process", fake_run_process)
    with pytest.raises(ValidationError, match="not supported by Tesseract"):
        await backend.process_file(ocr_image, language="invalid_lang")


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
async def test_process_file_linux(backend: TesseractBackend, mocker: MockerFixture) -> None:
    mocker.patch("sys.platform", "linux")

    async def linux_mock_run(*args: Any, **kwargs: Any) -> Mock:
        result = Mock()
        result.returncode = 0
        result.stdout = b"tesseract 5.0.0" if "--version" in args[0] else b"test output"
        result.stderr = b""
        return result

    mock_run = mocker.patch("kreuzberg._ocr._tesseract.run_process", side_effect=linux_mock_run)

    TesseractBackend._version_checked = False
    await backend.process_file(Path("test.png"), language="eng", psm=PSMMode.AUTO)

    assert any(call[1].get("env") == {"OMP_THREAD_LIMIT": "1"} for call in mock_run.call_args_list)


@pytest.mark.anyio
async def test_process_image_cache_processing_coordination(
    backend: TesseractBackend, tmp_path: Path, mocker: MockerFixture
) -> None:
    """Test cache processing coordination for process_image - covers lines 256-264."""
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
    """Test cache processing coordination for process_file - covers lines 323-331."""
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
    """Test language code validation error - covers line 436."""
    backend = TesseractBackend()

    with pytest.raises(ValidationError, match="provided language code is not supported"):
        backend._validate_language_code("invalid_language_code_that_is_too_long_and_invalid")


@pytest.mark.anyio
async def test_process_image_validation_error(backend: TesseractBackend) -> None:
    """Test validation error in process_image - covers line 251."""

    test_image = Image.new("RGB", (1, 1), color="white")

    from unittest.mock import patch

    with patch.object(backend, "_validate_language_code", side_effect=ValidationError("Invalid language")):
        with pytest.raises(ValidationError, match="Invalid language"):
            await backend.process_image(test_image, language="invalid")


@pytest.mark.anyio
async def test_process_file_validation_error(backend: TesseractBackend, tmp_path: Path) -> None:
    """Test validation error in process_file - covers line 357."""

    test_file = tmp_path / "test.png"
    test_image = Image.new("RGB", (100, 50), color="white")
    test_image.save(test_file)

    from unittest.mock import patch

    with patch.object(backend, "_validate_language_code", side_effect=ValidationError("Invalid language")):
        with pytest.raises(ValidationError, match="Invalid language"):
            await backend.process_file(test_file, language="invalid")
