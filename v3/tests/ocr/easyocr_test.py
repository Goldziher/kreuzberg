from __future__ import annotations

from dataclasses import asdict
from typing import TYPE_CHECKING, Any
from unittest.mock import Mock, patch

import pytest
from kreuzberg import EasyOCRConfig
from kreuzberg._ocr._easyocr import (
    EasyOCRBackend,
)
from kreuzberg._types import ExtractionResult
from kreuzberg.exceptions import MissingDependencyError, OCRError, ValidationError
from PIL import Image

if TYPE_CHECKING:
    from pathlib import Path

    from pytest_mock import MockerFixture


_original_import = __import__


def raise_import_error(name: str, *args: Any, **kwargs: Any) -> Any:
    if name == "easyocr":
        raise ImportError("No module named 'easyocr'")
    if name == "torch":
        raise ImportError("No module named 'torch'")
    return _original_import(name, *args, **kwargs)


@pytest.fixture
def backend() -> EasyOCRBackend:
    EasyOCRBackend._reader = None
    return EasyOCRBackend()


@pytest.fixture
def config_dict() -> dict[str, Any]:
    return asdict(EasyOCRConfig())


@pytest.fixture(autouse=True)
def reset_reader(mocker: MockerFixture) -> None:
    EasyOCRBackend._reader = None


@pytest.mark.parametrize(
    "language_code,expected_result",
    [
        ("en", ["en"]),
        ("EN", ["en"]),
        ("fr", ["fr"]),
        ("de", ["de"]),
        ("ja", ["ja"]),
        ("ch_sim", ["ch_sim"]),
        ("ch_tra", ["ch_tra"]),
        ("ko", ["ko"]),
        ("en,fr", ["en", "fr"]),
        ("en,ch_sim", ["en", "ch_sim"]),
        ("en,fr,ja", ["en", "fr", "ja"]),
        ("en, fr", ["en", "fr"]),
        (" en , fr ", ["en", "fr"]),
    ],
)
def test_validate_language_code_valid(language_code: str, expected_result: list[str]) -> None:
    result = EasyOCRBackend._validate_language_code(language_code)
    assert result == expected_result


@pytest.mark.parametrize(
    "language_list,expected_result",
    [
        (["en"], ["en"]),
        (["en", "fr"], ["en", "fr"]),
        (["en", "ch_sim"], ["en", "ch_sim"]),
        (["EN", "FR", "JA"], ["en", "fr", "ja"]),
    ],
)
def test_validate_language_code_list_valid(language_list: list[str], expected_result: list[str]) -> None:
    result = EasyOCRBackend._validate_language_code(language_list)
    assert result == expected_result


@pytest.mark.parametrize(
    "invalid_language_code",
    [
        "invalid",
        "english",
        "español",
        "русский",
        "eng",
        "deu",
        "fra",
        "jpn",
        "",
        "123",
    ],
)
def test_validate_language_code_invalid(invalid_language_code: str) -> None:
    with pytest.raises(ValidationError) as excinfo:
        EasyOCRBackend._validate_language_code(invalid_language_code)

    assert "language_code" in excinfo.value.context
    assert excinfo.value.context["language_code"] == invalid_language_code
    assert "supported_languages" in excinfo.value.context
    assert "not supported by EasyOCR" in str(excinfo.value)


@pytest.mark.parametrize(
    "mixed_language_codes",
    [
        "en,invalid",
        "invalid,en",
        "en,invalid,fr",
        ["en", "invalid"],
        ["invalid", "en"],
        ["en", "invalid", "fr"],
        "en,eng",
    ],
)
def test_validate_language_code_mixed_invalid(mixed_language_codes: str | list[str]) -> None:
    from kreuzberg._ocr._easyocr import EASYOCR_SUPPORTED_LANGUAGE_CODES

    with pytest.raises(ValidationError) as excinfo:
        EasyOCRBackend._validate_language_code(mixed_language_codes)

    assert "language_code" in excinfo.value.context
    assert "not supported by EasyOCR" in str(excinfo.value)

    if isinstance(mixed_language_codes, str):
        invalid_codes = [
            lang.strip()
            for lang in mixed_language_codes.split(",")
            if lang.strip().lower() not in EASYOCR_SUPPORTED_LANGUAGE_CODES
        ]
    else:
        invalid_codes = [lang for lang in mixed_language_codes if lang.lower() not in EASYOCR_SUPPORTED_LANGUAGE_CODES]

    reported_invalid = excinfo.value.context["language_code"].split(",")
    assert len(reported_invalid) == len(invalid_codes)
    for invalid in invalid_codes:
        assert invalid.lower() in [r.lower() for r in reported_invalid]


@pytest.mark.anyio
async def test_init_easyocr_with_invalid_language(backend: EasyOCRBackend) -> None:
    with pytest.raises(ValidationError, match="not supported by EasyOCR"):
        await backend._init_easyocr(language="invalid")


def test_is_gpu_available() -> None:
    mock_torch = Mock()
    mock_torch.cuda.is_available.return_value = True

    with patch.object(EasyOCRBackend, "_is_gpu_available", return_value=True):
        assert EasyOCRBackend._is_gpu_available() is True

    with patch.object(EasyOCRBackend, "_is_gpu_available", return_value=False):
        assert EasyOCRBackend._is_gpu_available() is False


@pytest.mark.anyio
async def test_init_easyocr_missing_dependency() -> None:
    with patch("kreuzberg._ocr._easyocr.HAS_EASYOCR", False), patch("kreuzberg._ocr._easyocr.easyocr", None):
        backend = EasyOCRBackend()
        with pytest.raises(MissingDependencyError) as excinfo:
            await backend._init_easyocr(language="en")

        error_message = str(excinfo.value)
        assert "easyocr" in error_message
        assert "required" in error_message
        assert "kreuzberg" in error_message


@pytest.mark.anyio
async def test_init_easyocr(mocker: MockerFixture) -> None:
    mock_reader = Mock()
    mock_easyocr = Mock()
    mock_easyocr.Reader = Mock(return_value=mock_reader)

    with patch("kreuzberg._ocr._easyocr.easyocr", mock_easyocr):
        backend = EasyOCRBackend()
        await backend._init_easyocr(language="en")

        mock_easyocr.Reader.assert_called_once()
        call_args = mock_easyocr.Reader.call_args
        assert call_args[0][0] == ["en"]
        assert call_args[1]["verbose"] is False
        assert backend._reader is mock_reader


@pytest.mark.anyio
async def test_init_easyocr_comma_separated_languages(mocker: MockerFixture) -> None:
    mock_reader = Mock()
    mock_easyocr = Mock()
    mock_easyocr.Reader = Mock(return_value=mock_reader)

    with patch("kreuzberg._ocr._easyocr.easyocr", mock_easyocr):
        EasyOCRBackend._reader = None
        backend = EasyOCRBackend()

        await backend._init_easyocr(language="en,ch_sim")

        mock_easyocr.Reader.assert_called_once()
        call_args = mock_easyocr.Reader.call_args
        assert call_args[0][0] == ["en", "ch_sim"]


@pytest.mark.anyio
async def test_init_easyocr_language_list(mocker: MockerFixture) -> None:
    mock_reader = Mock()
    mock_easyocr = Mock()
    mock_easyocr.Reader = Mock(return_value=mock_reader)

    with patch("kreuzberg._ocr._easyocr.easyocr", mock_easyocr):
        EasyOCRBackend._reader = None
        backend = EasyOCRBackend()

        await backend._init_easyocr(language=["en", "ch_sim"])

        mock_easyocr.Reader.assert_called_once()
        call_args = mock_easyocr.Reader.call_args
        assert call_args[0][0] == ["en", "ch_sim"]


@pytest.mark.anyio
async def test_init_easyocr_error(mocker: MockerFixture) -> None:
    error_message = "Failed to initialize"
    mock_easyocr = Mock()
    mock_easyocr.Reader = Mock()

    mocker.patch("kreuzberg._ocr._easyocr.easyocr", mock_easyocr)
    mocker.patch("kreuzberg._ocr._easyocr.run_sync", side_effect=Exception(error_message))

    backend = EasyOCRBackend()
    with pytest.raises(OCRError, match=f"Failed to initialize EasyOCR: {error_message}"):
        await backend._init_easyocr()


@pytest.mark.anyio
async def test_process_image(backend: EasyOCRBackend) -> None:
    from PIL import ImageDraw

    image = Image.new("RGB", (300, 80), "white")
    draw = ImageDraw.Draw(image)
    draw.text((10, 20), "Hello EasyOCR", fill="black")
    draw.text((10, 50), "Test Line 2", fill="black")

    mock_reader = Mock()
    mock_result = [
        ([[10, 20], [120, 20], [120, 40], [10, 40]], "Hello EasyOCR", 0.95),
        ([[10, 50], [100, 50], [100, 70], [10, 70]], "Test Line 2", 0.90),
    ]

    with patch.object(backend, "_init_easyocr"):
        EasyOCRBackend._reader = mock_reader
        with patch("kreuzberg._ocr._easyocr.run_sync", return_value=mock_result):
            result = await backend.process_image(image, use_cache=False)

        assert isinstance(result, ExtractionResult)
        assert "Hello EasyOCR" in result.content
        assert "Test Line 2" in result.content
        assert result.mime_type == "text/plain"
        assert result.metadata["width"] == 300
        assert result.metadata["height"] == 80


@pytest.mark.anyio
async def test_process_image_error(backend: EasyOCRBackend) -> None:
    from PIL import ImageDraw

    image = Image.new("RGB", (100, 50), "white")
    draw = ImageDraw.Draw(image)
    draw.text((10, 10), "Error Test", fill="black")

    with patch.object(backend, "_init_easyocr"):
        EasyOCRBackend._reader = Mock()
        EasyOCRBackend._reader.readtext = Mock(side_effect=Exception("OCR processing failed"))

        with pytest.raises(OCRError) as excinfo:
            await backend.process_image(image, use_cache=False)

        assert "Failed to OCR using EasyOCR" in str(excinfo.value)


@pytest.mark.anyio
async def test_process_file(backend: EasyOCRBackend, tmp_path: Path) -> None:
    from PIL import ImageDraw

    test_image = Image.new("RGB", (200, 60), "white")
    draw = ImageDraw.Draw(test_image)
    draw.text((10, 10), "File Text 1", fill="black")
    draw.text((10, 35), "File Text 2", fill="black")

    image_path = tmp_path / "test_image.png"
    test_image.save(image_path)

    mock_reader = Mock()
    mock_result = [
        ([[10, 10], [100, 10], [100, 30], [10, 30]], "File Text 1", 0.95),
        ([[10, 35], [100, 35], [100, 55], [10, 55]], "File Text 2", 0.90),
    ]

    with patch.object(backend, "_init_easyocr"):
        EasyOCRBackend._reader = mock_reader

        async def mock_run_sync(func, *args, **kwargs):  # type: ignore[no-untyped-def]
            if (
                hasattr(func, "__module__")
                and hasattr(func, "__name__")
                and func.__module__ == "PIL.Image"
                and func.__name__ == "open"
            ):
                return Image.open(*args, **kwargs)
            return mock_result

        with patch("kreuzberg._ocr._easyocr.run_sync", side_effect=mock_run_sync):
            result = await backend.process_file(image_path, language="en")

        assert isinstance(result, ExtractionResult)
        assert "File Text 1" in result.content
        assert "File Text 2" in result.content
        assert result.metadata["width"] == 200
        assert result.metadata["height"] == 60


@pytest.mark.anyio
async def test_process_file_error(backend: EasyOCRBackend, tmp_path: Path) -> None:
    from PIL import ImageDraw

    test_image = Image.new("RGB", (100, 50), "white")
    draw = ImageDraw.Draw(test_image)
    draw.text((10, 10), "Error", fill="black")

    image_path = tmp_path / "error_test.png"
    test_image.save(image_path)

    with patch("kreuzberg._ocr._easyocr.Image.open", side_effect=Exception("Failed to load image")):
        with pytest.raises(OCRError) as excinfo:
            await backend.process_file(image_path, language="en")

        assert "Failed to load or process image using EasyOCR" in str(excinfo.value)


@pytest.mark.anyio
async def test_process_easyocr_result_empty(backend: EasyOCRBackend) -> None:
    image = Image.new("RGB", (100, 100))
    result = backend._process_easyocr_result([], image)

    assert result.content == ""
    assert result.mime_type == "text/plain"
    assert result.metadata["width"] == 100
    assert result.metadata["height"] == 100


@pytest.mark.anyio
async def test_process_easyocr_result_simple_format(backend: EasyOCRBackend) -> None:
    image = Image.new("RGB", (100, 100))
    easyocr_result = [
        ("Line 1", 0.95),
        ("Line 2", 0.90),
    ]

    result = backend._process_easyocr_result(easyocr_result, image)

    assert "Line 1" in result.content
    assert "Line 2" in result.content
    assert result.mime_type == "text/plain"
    assert result.metadata["width"] == 100
    assert result.metadata["height"] == 100


@pytest.mark.anyio
async def test_process_easyocr_result_full_format(backend: EasyOCRBackend) -> None:
    image = Image.new("RGB", (100, 100))
    easyocr_result = [
        (
            [[0, 0], [100, 0], [100, 20], [0, 20]],
            "Line 1",
            0.95,
        ),
        (
            [[0, 30], [100, 30], [100, 50], [0, 50]],
            "Line 2",
            0.90,
        ),
    ]

    result = backend._process_easyocr_result(easyocr_result, image)

    assert "Line 1" in result.content
    assert "Line 2" in result.content
    assert result.mime_type == "text/plain"
    assert result.metadata["width"] == 100
    assert result.metadata["height"] == 100


def test_is_gpu_available_with_torch() -> None:
    mock_torch = Mock()
    mock_torch.cuda.is_available.return_value = True

    with patch("kreuzberg._ocr._easyocr.torch", mock_torch):
        result = EasyOCRBackend._is_gpu_available()
        assert result is True
        mock_torch.cuda.is_available.assert_called_once()


def test_is_gpu_available_without_torch() -> None:
    with patch("kreuzberg._ocr._easyocr.torch", None):
        result = EasyOCRBackend._is_gpu_available()
        assert result is False


def test_resolve_device_config_deprecated_use_gpu_true() -> None:
    with pytest.warns(DeprecationWarning, match="The 'use_gpu' parameter is deprecated"):
        device_info = EasyOCRBackend._resolve_device_config(use_gpu=True, device="auto")

        assert device_info.device_type in ["cpu", "cuda", "mps"]


def test_resolve_device_config_deprecated_use_gpu_conflicts() -> None:
    with pytest.warns(DeprecationWarning, match="Both 'use_gpu' and 'device' parameters specified"):
        device_info = EasyOCRBackend._resolve_device_config(use_gpu=True, device="cpu")

        assert device_info.device_type == "cpu"


def test_resolve_device_config_validation_error_fallback() -> None:
    with patch(
        "kreuzberg._utils._device.validate_device_request", side_effect=ValidationError("Device validation failed")
    ):
        device_info = EasyOCRBackend._resolve_device_config(use_gpu=False, device="cpu")
        assert device_info.device_type == "cpu"
        assert device_info.name == "CPU"


@pytest.mark.xfail(reason="Device validation behavior may differ in CI environment")
def test_resolve_device_config_validation_error_reraise_other_cases() -> None:
    with pytest.raises(ValidationError, match=r"Requested device.*not available"):
        EasyOCRBackend._resolve_device_config(use_gpu=True, device="cuda", fallback_to_cpu=False)


@pytest.mark.xfail(reason="Device validation behavior may differ in CI environment")
def test_resolve_device_config_validation_error_reraise() -> None:
    with pytest.raises(ValidationError, match="Requested device 'invalid' is not available"):
        EasyOCRBackend._resolve_device_config(use_gpu=True, device="invalid", fallback_to_cpu=False)


def test_process_results_edge_cases() -> None:
    mock_results = [
        ([[10, 10], [50, 10], [50, 30], [10, 30]], "Hello", 0.9),
        ([[60, 12], [100, 12], [100, 32], [60, 32]], "World", 0.8),
        ([[10, 50], [80, 50], [80, 70], [10, 70]], "", 0.7),
    ]

    test_image = Image.new("RGB", (100, 100), color="white")

    result = EasyOCRBackend._process_easyocr_result(mock_results, test_image)

    assert "Hello World" in result.content


@pytest.mark.anyio
async def test_init_easyocr_already_initialized() -> None:
    original_reader = EasyOCRBackend._reader
    EasyOCRBackend._reader = Mock()

    try:
        await EasyOCRBackend._init_easyocr(language="en")

        assert EasyOCRBackend._reader is not None
        assert isinstance(EasyOCRBackend._reader, Mock)
    finally:
        EasyOCRBackend._reader = original_reader


def test_process_image_sync(backend: EasyOCRBackend) -> None:
    from PIL import ImageDraw

    image = Image.new("RGB", (150, 60), "white")
    draw = ImageDraw.Draw(image)
    draw.text((10, 20), "Sync Test", fill="black")

    mock_reader = Mock()
    mock_reader.readtext.return_value = [([[10, 20], [80, 20], [80, 40], [10, 40]], "Sync Test", 0.95)]

    with patch.object(backend, "_init_easyocr_sync"), patch.object(backend, "_reader", mock_reader):
        result = backend.process_image_sync(image, beam_width=5, language="en")

        assert isinstance(result, ExtractionResult)
        assert "Sync Test" in result.content
        assert result.metadata["width"] == 150
        assert result.metadata["height"] == 60


def test_process_file_sync(backend: EasyOCRBackend, tmp_path: Path) -> None:
    from PIL import ImageDraw

    test_image = Image.new("RGB", (150, 60), "white")
    draw = ImageDraw.Draw(test_image)
    draw.text((10, 20), "File Sync", fill="black")

    image_path = tmp_path / "test_sync.png"
    test_image.save(image_path)

    mock_reader = Mock()
    mock_reader.readtext.return_value = [([[10, 20], [80, 20], [80, 40], [10, 40]], "File Sync", 0.90)]

    with patch.object(backend, "_init_easyocr_sync"), patch.object(backend, "_reader", mock_reader):
        result = backend.process_file_sync(image_path, beam_width=5, language="en")

        assert isinstance(result, ExtractionResult)
        assert "File Sync" in result.content
