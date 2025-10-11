from __future__ import annotations

from typing import TYPE_CHECKING
from unittest.mock import Mock, patch

import pytest
from PIL import Image

from kreuzberg._ocr._paddleocr import PADDLEOCR_SUPPORTED_LANGUAGE_CODES, PaddleBackend
from kreuzberg._types import ExtractionResult
from kreuzberg.exceptions import MissingDependencyError, OCRError, ValidationError

if TYPE_CHECKING:
    from pathlib import Path

    from pytest_mock import MockerFixture


@pytest.fixture
def backend() -> PaddleBackend:
    return PaddleBackend()


@pytest.fixture
def mock_find_spec(mocker: MockerFixture) -> Mock:
    mock = mocker.patch("kreuzberg._ocr._paddleocr.find_spec")
    mock.return_value = True
    return mock


@pytest.fixture
def mock_find_spec_missing(mocker: MockerFixture) -> Mock:
    mock = mocker.patch("kreuzberg._ocr._paddleocr.find_spec")
    mock.return_value = None
    return mock


@pytest.mark.anyio
async def test_is_mkldnn_supported(mocker: MockerFixture) -> None:
    mocker.patch("platform.system", return_value="Linux")
    mocker.patch("platform.processor", return_value="x86_64")
    mocker.patch("platform.machine", return_value="x86_64")
    assert PaddleBackend._is_mkldnn_supported() is True

    mocker.patch("platform.system", return_value="Windows")
    mocker.patch("platform.processor", return_value="Intel64 Family 6")
    assert PaddleBackend._is_mkldnn_supported() is True

    mocker.patch("platform.system", return_value="Darwin")
    mocker.patch("platform.machine", return_value="x86_64")
    assert PaddleBackend._is_mkldnn_supported() is True

    mocker.patch("platform.system", return_value="Darwin")
    mocker.patch("platform.machine", return_value="arm64")
    assert PaddleBackend._is_mkldnn_supported() is False

    mocker.patch("platform.system", return_value="FreeBSD")
    assert PaddleBackend._is_mkldnn_supported() is False

    mocker.patch("platform.system", return_value="Windows")
    mocker.patch("platform.processor", return_value="AMD64")
    mocker.patch("platform.machine", return_value="AMD64")
    assert PaddleBackend._is_mkldnn_supported() is True

    mocker.patch("platform.system", return_value="Linux")
    mocker.patch("platform.processor", return_value="aarch64")
    mocker.patch("platform.machine", return_value="aarch64")
    assert PaddleBackend._is_mkldnn_supported() is False


@pytest.mark.anyio
async def test_init_paddle_ocr(backend: PaddleBackend, mock_find_spec: Mock, mocker: MockerFixture) -> None:
    PaddleBackend._paddle_ocr = None

    mock_paddleocr = Mock()
    mock_instance = Mock()
    mock_paddleocr.return_value = mock_instance

    mocker.patch("kreuzberg._ocr._paddleocr._import_paddleocr", return_value=(Mock(), mock_paddleocr))

    await backend._init_paddle_ocr()

    mock_paddleocr.assert_called_once()
    assert PaddleBackend._paddle_ocr is mock_instance

    mock_paddleocr.reset_mock()

    await backend._init_paddle_ocr()
    mock_paddleocr.assert_not_called()


@pytest.mark.anyio
async def test_init_paddle_ocr_with_gpu_package(
    backend: PaddleBackend, mock_find_spec: Mock, mocker: MockerFixture
) -> None:
    PaddleBackend._paddle_ocr = None

    mocker.patch("kreuzberg._ocr._paddleocr.find_spec", side_effect=lambda x: True if x == "paddlepaddle_gpu" else None)

    from kreuzberg._utils._device import DeviceInfo

    mock_device_info = DeviceInfo(device_type="cuda", device_id=0, name="NVIDIA GPU")
    mocker.patch("kreuzberg._ocr._paddleocr.validate_device_request", return_value=mock_device_info)

    mock_paddleocr = Mock()
    mock_instance = Mock()
    mock_paddleocr.return_value = mock_instance

    mocker.patch("kreuzberg._ocr._paddleocr._import_paddleocr", return_value=(Mock(), mock_paddleocr))

    await backend._init_paddle_ocr()

    mock_paddleocr.assert_called_once()
    _call_args, call_kwargs = mock_paddleocr.call_args

    assert call_kwargs.get("enable_mkldnn") is False


@pytest.mark.anyio
async def test_init_paddle_ocr_with_language(
    backend: PaddleBackend, mock_find_spec: Mock, mocker: MockerFixture
) -> None:
    PaddleBackend._paddle_ocr = None

    mock_paddleocr = Mock()
    mock_instance = Mock()
    mock_paddleocr.return_value = mock_instance

    mocker.patch("kreuzberg._ocr._paddleocr._import_paddleocr", return_value=(Mock(), mock_paddleocr))

    with patch.object(PaddleBackend, "_validate_language_code", return_value="french"):
        await backend._init_paddle_ocr(language="fra")

        mock_paddleocr.assert_called_once()
        _call_args, call_kwargs = mock_paddleocr.call_args
        assert call_kwargs.get("lang") == "french"


@pytest.mark.anyio
async def test_init_paddle_ocr_with_custom_options(
    backend: PaddleBackend, mock_find_spec: Mock, mocker: MockerFixture
) -> None:
    PaddleBackend._paddle_ocr = None

    mock_paddleocr = Mock()
    mock_instance = Mock()
    mock_paddleocr.return_value = mock_instance

    mocker.patch("kreuzberg._ocr._paddleocr._import_paddleocr", return_value=(Mock(), mock_paddleocr))

    custom_options = {
        "det_db_thresh": 0.4,
        "det_db_box_thresh": 0.6,
        "det_db_unclip_ratio": 2.0,
        "use_angle_cls": False,
        "det_algorithm": "EAST",
        "rec_algorithm": "SRN",
    }

    await backend._init_paddle_ocr(**custom_options)

    mock_paddleocr.assert_called_once()
    _call_args, call_kwargs = mock_paddleocr.call_args

    assert call_kwargs.get("text_det_thresh") == 0.4
    assert call_kwargs.get("text_det_box_thresh") == 0.6
    assert call_kwargs.get("text_det_unclip_ratio") == 2.0
    assert call_kwargs.get("use_textline_orientation") is False
    assert call_kwargs.get("det_algorithm") == "EAST"
    assert call_kwargs.get("rec_algorithm") == "SRN"

    assert "det_db_thresh" not in call_kwargs
    assert "det_db_box_thresh" not in call_kwargs
    assert "det_db_unclip_ratio" not in call_kwargs
    assert "use_angle_cls" not in call_kwargs


@pytest.mark.anyio
async def test_init_paddle_ocr_with_model_dirs(
    backend: PaddleBackend, mock_find_spec: Mock, mocker: MockerFixture
) -> None:
    PaddleBackend._paddle_ocr = None

    mock_paddleocr = Mock()
    mock_instance = Mock()
    mock_paddleocr.return_value = mock_instance

    mocker.patch("kreuzberg._ocr._paddleocr._import_paddleocr", return_value=(Mock(), mock_paddleocr))

    custom_options = {
        "det_model_dir": "/path/to/det/model",
        "rec_model_dir": "/path/to/rec/model",
    }

    await backend._init_paddle_ocr(**custom_options)

    mock_paddleocr.assert_called_once()
    _call_args, call_kwargs = mock_paddleocr.call_args

    assert call_kwargs.get("det_model_dir") == "/path/to/det/model"
    assert call_kwargs.get("rec_model_dir") == "/path/to/rec/model"


@pytest.mark.anyio
async def test_init_paddle_ocr_missing_dependency(backend: PaddleBackend, mock_find_spec_missing: Mock) -> None:
    PaddleBackend._paddle_ocr = None

    with patch("kreuzberg._ocr._paddleocr._import_paddleocr", return_value=(None, None)):
        with pytest.raises(MissingDependencyError) as excinfo:
            await backend._init_paddle_ocr()

        error_message = str(excinfo.value)
        assert "paddleocr" in error_message
        assert "missing" in error_message.lower() or "required" in error_message.lower()


@pytest.mark.anyio
async def test_init_paddle_ocr_initialization_error(
    backend: PaddleBackend, mock_find_spec: Mock, mocker: MockerFixture
) -> None:
    PaddleBackend._paddle_ocr = None

    mock_paddleocr = Mock()
    mock_paddleocr.side_effect = Exception("Initialization error")

    mocker.patch("kreuzberg._ocr._paddleocr._import_paddleocr", return_value=(Mock(), mock_paddleocr))

    with pytest.raises(OCRError) as excinfo:
        await backend._init_paddle_ocr()

    assert "Failed to initialize PaddleOCR" in str(excinfo.value)


@pytest.mark.anyio
async def test_process_image(backend: PaddleBackend) -> None:
    from PIL import ImageDraw

    image = Image.new("RGB", (400, 100), "white")
    draw = ImageDraw.Draw(image)
    draw.text((10, 30), "Hello World Test", fill="black")

    with patch.object(backend, "_init_paddle_ocr"):
        PaddleBackend._paddle_ocr = Mock()
        backend._paddle_ocr.ocr = Mock(
            return_value=[[[[[10, 30], [150, 30], [150, 50], [10, 50]], ("Hello World Test", 0.95)]]]
        )

        result = await backend.process_image(image, language="en", use_cache=False)

        assert isinstance(result, ExtractionResult)
        assert result.mime_type == "text/plain"
        assert "Hello World Test" in result.content
        assert result.metadata.get("width") == 400
        assert result.metadata.get("height") == 100


@pytest.mark.anyio
async def test_process_image_with_options(backend: PaddleBackend) -> None:
    from PIL import ImageDraw

    image = Image.new("RGB", (300, 80), "white")
    draw = ImageDraw.Draw(image)
    draw.text((10, 20), "Text Line 1", fill="black")
    draw.text((10, 50), "Text Line 2", fill="black")

    with patch.object(backend, "_init_paddle_ocr"):
        PaddleBackend._paddle_ocr = Mock()
        backend._paddle_ocr.ocr = Mock(
            return_value=[
                [
                    [[[10, 20], [100, 20], [100, 40], [10, 40]], ("Text Line 1", 0.95)],
                    [[[10, 50], [100, 50], [100, 70], [10, 70]], ("Text Line 2", 0.90)],
                ]
            ]
        )

        result = await backend.process_image(
            image,
            language="en",
            use_angle_cls=True,
            det_db_thresh=0.4,
            det_db_box_thresh=0.6,
            use_cache=False,
        )

        assert isinstance(result, ExtractionResult)
        assert "Text Line 1" in result.content
        assert "Text Line 2" in result.content


@pytest.mark.anyio
async def test_process_image_error(backend: PaddleBackend) -> None:
    from PIL import ImageDraw

    image = Image.new("RGB", (100, 50), "white")
    draw = ImageDraw.Draw(image)
    draw.text((10, 10), "Error Test", fill="black")

    with patch.object(backend, "_init_paddle_ocr"):
        PaddleBackend._paddle_ocr = Mock()
        backend._paddle_ocr.ocr = Mock(side_effect=Exception("OCR processing error"))

        with pytest.raises(OCRError) as excinfo:
            await backend.process_image(image, use_cache=False)

        assert "Failed to OCR using PaddleOCR" in str(excinfo.value)


@pytest.mark.anyio
async def test_process_file(backend: PaddleBackend, tmp_path: Path) -> None:
    from PIL import ImageDraw

    image = Image.new("RGB", (200, 60), "white")
    draw = ImageDraw.Draw(image)
    draw.text((10, 10), "File Line 1", fill="black")
    draw.text((10, 35), "File Line 2", fill="black")

    test_file = tmp_path / "test_ocr.png"
    image.save(test_file)

    with patch.object(backend, "_init_paddle_ocr"):
        PaddleBackend._paddle_ocr = Mock()
        backend._paddle_ocr.ocr = Mock(
            return_value=[
                [
                    [[[10, 10], [100, 10], [100, 30], [10, 30]], ("File Line 1", 0.95)],
                    [[[10, 35], [100, 35], [100, 55], [10, 55]], ("File Line 2", 0.90)],
                ]
            ]
        )

        result = await backend.process_file(test_file)

        assert isinstance(result, ExtractionResult)
        assert "File Line 1" in result.content
        assert "File Line 2" in result.content


@pytest.mark.anyio
async def test_process_file_with_options(backend: PaddleBackend, tmp_path: Path) -> None:
    from PIL import ImageDraw

    image = Image.new("RGB", (200, 60), "white")
    draw = ImageDraw.Draw(image)
    draw.text((10, 10), "Options Test 1", fill="black")
    draw.text((10, 35), "Options Test 2", fill="black")

    test_file = tmp_path / "test_options.png"
    image.save(test_file)

    with patch.object(backend, "_init_paddle_ocr") as mock_init:
        PaddleBackend._paddle_ocr = Mock()
        backend._paddle_ocr.ocr = Mock(
            return_value=[
                [
                    [[[10, 10], [120, 10], [120, 30], [10, 30]], ("Options Test 1", 0.95)],
                    [[[10, 35], [120, 35], [120, 55], [10, 55]], ("Options Test 2", 0.90)],
                ]
            ]
        )

        result = await backend.process_file(
            test_file,
            language="french",
            use_angle_cls=True,
            det_db_thresh=0.4,
        )

        assert mock_init.called
        kwargs = mock_init.call_args_list[-1][1]
        assert kwargs.get("language") == "french"
        assert kwargs.get("use_angle_cls") is True
        assert kwargs.get("det_db_thresh") == 0.4

        assert isinstance(result, ExtractionResult)
        assert "Options Test 1" in result.content
        assert "Options Test 2" in result.content


@pytest.mark.anyio
async def test_process_file_error(backend: PaddleBackend, tmp_path: Path) -> None:
    from PIL import ImageDraw

    image = Image.new("RGB", (100, 50), "white")
    draw = ImageDraw.Draw(image)
    draw.text((10, 10), "Error", fill="black")

    test_file = tmp_path / "error_test.png"
    image.save(test_file)

    with patch("kreuzberg._ocr._paddleocr.Image.open", side_effect=Exception("File processing error")):
        with pytest.raises(OCRError) as excinfo:
            await backend.process_file(test_file, use_cache=False)

        assert "Failed to load or process image using PaddleOCR" in str(excinfo.value)


@pytest.mark.anyio
async def test_process_paddle_result_empty() -> None:
    image = Mock(spec=Image.Image)
    image.configure_mock(size=(100, 100), width=100, height=100)

    result = PaddleBackend._process_paddle_result([], image)

    assert isinstance(result, ExtractionResult)
    assert result.content == ""

    assert isinstance(result.metadata, dict)
    assert result.metadata.get("width") == 100
    assert result.metadata.get("height") == 100


@pytest.mark.anyio
async def test_process_paddle_result_empty_page() -> None:
    image = Mock(spec=Image.Image)
    image.configure_mock(size=(100, 100), width=100, height=100)

    result = PaddleBackend._process_paddle_result([[]], image)

    assert isinstance(result, ExtractionResult)
    assert result.content == ""
    assert result.metadata.get("width") == 100
    assert result.metadata.get("height") == 100


@pytest.mark.anyio
async def test_process_paddle_result_complex() -> None:
    image = Mock(spec=Image.Image)
    image.configure_mock(size=(200, 200), width=200, height=200)

    paddle_result = [
        [
            [
                [[10, 10], [100, 10], [100, 30], [10, 30]],
                ("Line 1 Text 1", 0.95),
            ],
            [
                [[110, 10], [200, 10], [200, 30], [110, 30]],
                ("Line 1 Text 2", 0.90),
            ],
            [
                [[10, 50], [100, 50], [100, 70], [10, 70]],
                ("Line 2 Text 1", 0.85),
            ],
            [
                [[110, 50], [200, 50], [200, 70], [110, 70]],
                ("Line 2 Text 2", 0.80),
            ],
            [
                [[10, 90], [200, 90], [200, 110], [10, 110]],
                ("Line 3 Text", 0.75),
            ],
        ]
    ]

    result = PaddleBackend._process_paddle_result(paddle_result, image)

    assert isinstance(result, ExtractionResult)
    assert "Line 1 Text 1 Line 1 Text 2" in result.content
    assert "Line 2 Text 1 Line 2 Text 2" in result.content
    assert "Line 3 Text" in result.content

    assert isinstance(result.metadata, dict)
    assert result.metadata.get("width") == 200
    assert result.metadata.get("height") == 200


@pytest.mark.anyio
async def test_process_paddle_result_with_empty_text() -> None:
    image = Mock(spec=Image.Image)
    image.configure_mock(size=(100, 100), width=100, height=100)

    paddle_result = [
        [
            [
                [[10, 10], [100, 10], [100, 30], [10, 30]],
                ("", 0.95),
            ],
            [
                [[10, 50], [100, 50], [100, 70], [10, 70]],
                ("Valid text", 0.85),
            ],
            [
                [[10, 90], [100, 90], [100, 110], [10, 110]],
                ("", 0.70),
            ],
        ]
    ]

    result = PaddleBackend._process_paddle_result(paddle_result, image)

    assert isinstance(result, ExtractionResult)
    assert "Valid text" in result.content


@pytest.mark.anyio
async def test_process_paddle_result_with_close_lines() -> None:
    image = Mock(spec=Image.Image)
    image.size = (200, 100)

    paddle_result = [
        [
            [
                [[10, 10], [100, 10], [100, 30], [10, 30]],
                ("Same line 1", 0.95),
            ],
            [
                [[110, 15], [200, 15], [200, 35], [110, 35]],
                ("Same line 2", 0.90),
            ],
            [
                [[10, 60], [100, 60], [100, 80], [10, 80]],
                ("Different line", 0.85),
            ],
        ]
    ]

    result = PaddleBackend._process_paddle_result(paddle_result, image)

    assert isinstance(result, ExtractionResult)
    assert "Same line 1 Same line 2" in result.content
    assert "Different line" in result.content


@pytest.mark.anyio
async def test_integration_process_file(backend: PaddleBackend, ocr_image: Path) -> None:
    try:
        from paddleocr import PaddleOCR  # noqa: F401  # type: ignore[import-untyped]
    except ImportError:
        pytest.skip("PaddleOCR not installed")

    import platform

    if platform.system() == "Darwin" and platform.machine() == "arm64":
        pytest.skip("PaddleOCR not fully compatible with Mac M1/ARM architecture")

    result = await backend.process_file(ocr_image)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip()


@pytest.mark.anyio
async def test_integration_process_image(backend: PaddleBackend, ocr_image: Path) -> None:
    try:
        from paddleocr import PaddleOCR  # noqa: F401  # type: ignore[import-untyped]
    except ImportError:
        pytest.skip("PaddleOCR not installed")

    import platform

    if platform.system() == "Darwin" and platform.machine() == "arm64":
        pytest.skip("PaddleOCR not fully compatible with Mac M1/ARM architecture")

    image = Image.open(ocr_image)
    with image:
        result = await backend.process_image(image)
        assert isinstance(result, ExtractionResult)
        assert result.content.strip()


@pytest.mark.parametrize(
    "language_code,expected_result",
    [
        ("en", "en"),
        ("EN", "en"),
        ("ch", "ch"),
        ("french", "french"),
        ("german", "german"),
        ("japan", "japan"),
        ("korean", "korean"),
    ],
)
def test_validate_language_code_valid(language_code: str, expected_result: str) -> None:
    result = PaddleBackend._validate_language_code(language_code)
    assert result == expected_result


@pytest.mark.parametrize(
    "invalid_language_code",
    [
        "invalid",
        "español",
        "русский",
        "fra",
        "deu",
        "jpn",
        "kor",
        "zho",
        "",
        "123",
    ],
)
def test_validate_language_code_invalid(invalid_language_code: str) -> None:
    with pytest.raises(ValidationError) as excinfo:
        PaddleBackend._validate_language_code(invalid_language_code)

    assert "language_code" in excinfo.value.context
    assert excinfo.value.context["language_code"] == invalid_language_code
    assert "supported_languages" in excinfo.value.context

    assert "not supported by PaddleOCR" in str(excinfo.value)


@pytest.mark.anyio
async def test_process_image_grayscale_conversion() -> None:
    from PIL import ImageDraw

    backend = PaddleBackend()

    grayscale_image = Image.new("L", (200, 50), color=255)
    draw = ImageDraw.Draw(grayscale_image)
    draw.text((10, 10), "TEST", fill=0)

    with patch.object(backend, "_init_paddle_ocr"):
        PaddleBackend._paddle_ocr = Mock()
        backend._paddle_ocr.ocr = Mock(return_value=[[[[[10, 10], [50, 10], [50, 30], [10, 30]], ("TEST", 0.95)]]])

        result = await backend.process_image(grayscale_image, use_cache=False)

        backend._paddle_ocr.ocr.assert_called_once()

        np_array = backend._paddle_ocr.ocr.call_args[0][0]
        assert len(np_array.shape) == 3
        assert np_array.shape[2] == 3

        assert isinstance(result, ExtractionResult)
        assert "TEST" in result.content


@pytest.mark.anyio
async def test_process_paddle_result_current_line_handling() -> None:
    test_image = Image.new("RGB", (200, 100), color="white")

    mock_results = [
        [
            [[[10, 10], [50, 10], [50, 30], [10, 30]], ("Hello", 0.9)],
            [[[60, 12], [100, 12], [100, 32], [60, 32]], ("World", 0.8)],
            [[[10, 50], [80, 50], [80, 70], [10, 70]], ("Second", 0.7)],
            [[[90, 52], [150, 52], [150, 72], [90, 72]], ("Line", 0.6)],
        ]
    ]

    result = PaddleBackend._process_paddle_result(mock_results, test_image)

    assert "Hello World" in result.content
    assert "Second Line" in result.content


def test_process_paddle_result_image_size_fallback() -> None:
    from unittest.mock import Mock

    mock_image = Mock()
    del mock_image.width
    del mock_image.height
    mock_image.size = (300, 200)

    mock_results = [[[[[10, 10], [50, 10], [50, 30], [10, 30]], ("Test", 0.9)]]]

    result = PaddleBackend._process_paddle_result(mock_results, mock_image)

    assert result.metadata["width"] == 300
    assert result.metadata["height"] == 200


@pytest.mark.anyio
async def test_resolve_device_config_mps_warning() -> None:
    import warnings
    from unittest.mock import patch

    with warnings.catch_warnings(record=True) as w:
        warnings.simplefilter("always")

        with patch("kreuzberg._utils._device.validate_device_request") as mock_validate:
            mock_validate.return_value = Mock(device_type="cpu", name="CPU")

            PaddleBackend._resolve_device_config(device="mps")

            assert len(w) == 1
            assert issubclass(w[0].category, UserWarning)
            assert "PaddlePaddle does not support MPS" in str(w[0].message)


@pytest.mark.anyio
async def test_resolve_device_config_validation_error_fallback() -> None:
    from unittest.mock import patch

    with patch("kreuzberg._utils._device.validate_device_request") as mock_validate:
        mock_validate.side_effect = ValidationError("Device validation failed")

        result = PaddleBackend._resolve_device_config(use_gpu=False, device="cpu")

        assert result.device_type == "cpu"
        assert result.name == "CPU"


@pytest.mark.anyio
async def test_init_paddle_ocr_with_invalid_language(
    backend: PaddleBackend, mock_find_spec: Mock, mocker: MockerFixture
) -> None:
    PaddleBackend._paddle_ocr = None

    validation_error = ValidationError(
        "The provided language code is not supported by PaddleOCR",
        context={
            "language_code": "invalid_language",
            "supported_languages": ",".join(sorted(PADDLEOCR_SUPPORTED_LANGUAGE_CODES)),
        },
    )

    mocker.patch.object(PaddleBackend, "_validate_language_code", side_effect=validation_error)
    mocker.patch("kreuzberg._ocr._paddleocr._import_paddleocr", return_value=(Mock(), Mock()))

    with pytest.raises(ValidationError) as excinfo:
        await backend._init_paddle_ocr(language="invalid_language")

    assert "language_code" in excinfo.value.context
    assert excinfo.value.context["language_code"] == "invalid_language"
    assert "supported_languages" in excinfo.value.context

    assert "not supported by PaddleOCR" in str(excinfo.value)


def test_process_image_sync(backend: PaddleBackend) -> None:
    from unittest.mock import Mock, patch

    image = Image.new("RGB", (100, 100))

    mock_paddle = Mock()
    mock_paddle.ocr.return_value = [
        [
            [
                [[0, 0], [100, 0], [100, 20], [0, 20]],
                ("Sample OCR text", 0.95),
            ]
        ]
    ]

    with patch.object(backend, "_init_paddle_ocr_sync"), patch.object(backend, "_paddle_ocr", mock_paddle):
        result = backend.process_image_sync(image)

        assert isinstance(result, ExtractionResult)
        assert result.content.strip() == "Sample OCR text"
        assert result.metadata["width"] == 100
        assert result.metadata["height"] == 100


def test_process_file_sync(backend: PaddleBackend, tmp_path: Path) -> None:
    from unittest.mock import Mock, patch

    test_image = Image.new("RGB", (100, 100))
    image_path = tmp_path / "test_image.png"
    test_image.save(image_path)

    mock_paddle = Mock()
    mock_paddle.ocr.return_value = [
        [
            [
                [[0, 0], [100, 0], [100, 20], [0, 20]],
                ("Sample file text", 0.90),
            ]
        ]
    ]

    with patch.object(backend, "_init_paddle_ocr_sync"), patch.object(backend, "_paddle_ocr", mock_paddle):
        result = backend.process_file_sync(image_path)

        assert isinstance(result, ExtractionResult)
        assert result.content.strip() == "Sample file text"
