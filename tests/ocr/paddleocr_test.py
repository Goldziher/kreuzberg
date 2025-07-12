from __future__ import annotations

from typing import TYPE_CHECKING, Any
from unittest.mock import Mock, patch

import numpy as np
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
def mock_paddleocr(mocker: MockerFixture) -> Mock:
    mock = mocker.patch("paddleocr.PaddleOCR")
    instance = mock.return_value

    instance.ocr.return_value = [
        [
            [
                [[10, 10], [100, 10], [100, 30], [10, 30]],
                ("Sample text 1", 0.95),
            ],
            [
                [[10, 40], [100, 40], [100, 60], [10, 60]],
                ("Sample text 2", 0.90),
            ],
        ]
    ]
    return mock


@pytest.fixture
def mock_run_sync(mocker: MockerFixture) -> Mock:
    async def mock_async_run_sync(func: Any, *args: Any, **kwargs: Any) -> Any:
        if isinstance(func, Mock) and kwargs.get("image_np") is not None:
            return [
                [
                    [
                        [[10, 10], [100, 10], [100, 30], [10, 30]],
                        ("Sample text 1", 0.95),
                    ],
                    [
                        [[10, 40], [100, 40], [100, 60], [10, 60]],
                        ("Sample text 2", 0.90),
                    ],
                ]
            ]

        if callable(func) and hasattr(func, "__name__") and func.__name__ == "open":
            img = Mock(spec=Image.Image)
            img.size = (100, 100)

            array_interface = {
                "shape": (100, 100, 3),
                "typestr": "|u1",
                "data": np.zeros((100, 100, 3), dtype=np.uint8).tobytes(),
                "strides": None,
                "version": 3,
            }
            type(img).__array_interface__ = array_interface
            return img

        if callable(func) and hasattr(func, "__name__") and func.__name__ == "PaddleOCR":
            paddle_ocr = Mock()
            paddle_ocr.ocr = Mock()
            paddle_ocr.ocr.return_value = [
                [
                    [
                        [[10, 10], [100, 10], [100, 30], [10, 30]],
                        ("Sample text 1", 0.95),
                    ],
                    [
                        [[10, 40], [100, 40], [100, 60], [10, 60]],
                        ("Sample text 2", 0.90),
                    ],
                ]
            ]
            return paddle_ocr
        return func(*args, **kwargs)

    return mocker.patch("kreuzberg._ocr._paddleocr.run_sync", side_effect=mock_async_run_sync)


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


@pytest.fixture
def mock_image() -> Mock:
    img = Mock(spec=Image.Image)

    img.configure_mock(size=(100, 100), width=100, height=100)

    img.convert.return_value = img

    array_interface = {
        "shape": (100, 100, 3),
        "typestr": "|u1",
        "data": np.zeros((100, 100, 3), dtype=np.uint8).tobytes(),
        "strides": None,
        "version": 3,
    }
    type(img).__array_interface__ = array_interface
    return img


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
async def test_init_paddle_ocr(
    backend: PaddleBackend, mock_paddleocr: Mock, mock_run_sync: Mock, mock_find_spec: Mock
) -> None:
    PaddleBackend._paddle_ocr = None

    await backend._init_paddle_ocr()

    mock_run_sync.assert_called_once()
    mock_paddleocr.assert_called_once()

    assert PaddleBackend._paddle_ocr is not None

    mock_run_sync.reset_mock()
    mock_paddleocr.reset_mock()

    await backend._init_paddle_ocr()
    mock_run_sync.assert_not_called()
    mock_paddleocr.assert_not_called()


@pytest.mark.anyio
async def test_init_paddle_ocr_with_gpu_package(
    backend: PaddleBackend, mock_paddleocr: Mock, mock_run_sync: Mock, mock_find_spec: Mock, mocker: MockerFixture
) -> None:
    PaddleBackend._paddle_ocr = None

    mocker.patch("kreuzberg._ocr._paddleocr.find_spec", side_effect=lambda x: True if x == "paddlepaddle_gpu" else None)

    from kreuzberg._utils._device import DeviceInfo

    mock_device_info = DeviceInfo(device_type="cuda", device_id=0, name="NVIDIA GPU")
    mocker.patch("kreuzberg._ocr._paddleocr.validate_device_request", return_value=mock_device_info)

    await backend._init_paddle_ocr()

    mock_paddleocr.assert_called_once()
    call_args, call_kwargs = mock_paddleocr.call_args

    assert call_kwargs.get("use_gpu") is True
    assert call_kwargs.get("enable_mkldnn") is False

    PaddleBackend._paddle_ocr = None
    mock_paddleocr.reset_mock()
    mock_run_sync.reset_mock()


@pytest.mark.anyio
async def test_init_paddle_ocr_with_language(
    backend: PaddleBackend, mock_paddleocr: Mock, mock_run_sync: Mock, mock_find_spec: Mock
) -> None:
    PaddleBackend._paddle_ocr = None

    with patch.object(PaddleBackend, "_validate_language_code", return_value="french"):
        await backend._init_paddle_ocr(language="fra")

        mock_paddleocr.assert_called_once()
        call_args, call_kwargs = mock_paddleocr.call_args
        assert call_kwargs.get("lang") == "french"


@pytest.mark.anyio
async def test_init_paddle_ocr_with_custom_options(
    backend: PaddleBackend, mock_paddleocr: Mock, mock_run_sync: Mock, mock_find_spec: Mock
) -> None:
    PaddleBackend._paddle_ocr = None

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
    call_args, call_kwargs = mock_paddleocr.call_args

    assert call_kwargs.get("det_db_thresh") == 0.4
    assert call_kwargs.get("det_db_box_thresh") == 0.6
    assert call_kwargs.get("det_db_unclip_ratio") == 2.0
    assert call_kwargs.get("use_angle_cls") is False
    assert call_kwargs.get("det_algorithm") == "EAST"
    assert call_kwargs.get("rec_algorithm") == "SRN"


@pytest.mark.anyio
async def test_init_paddle_ocr_with_model_dirs(
    backend: PaddleBackend, mock_paddleocr: Mock, mock_run_sync: Mock, mock_find_spec: Mock
) -> None:
    PaddleBackend._paddle_ocr = None

    custom_options = {
        "det_model_dir": "/path/to/det/model",
        "rec_model_dir": "/path/to/rec/model",
    }

    await backend._init_paddle_ocr(**custom_options)

    mock_paddleocr.assert_called_once()
    call_args, call_kwargs = mock_paddleocr.call_args

    assert call_kwargs.get("det_model_dir") == "/path/to/det/model"
    assert call_kwargs.get("rec_model_dir") == "/path/to/rec/model"


@pytest.mark.anyio
async def test_init_paddle_ocr_missing_dependency(backend: PaddleBackend, mock_find_spec_missing: Mock) -> None:
    PaddleBackend._paddle_ocr = None

    def mock_import(name: str, *args: Any, **kwargs: Any) -> Any:
        if name == "paddleocr":
            raise ImportError("No module named 'paddleocr'")
        return __import__(name, *args, **kwargs)

    with patch("builtins.__import__", side_effect=mock_import):
        with pytest.raises(MissingDependencyError) as excinfo:
            await backend._init_paddle_ocr()

        error_message = str(excinfo.value)
        assert "paddleocr" in error_message
        assert "missing" in error_message.lower() or "required" in error_message.lower()


@pytest.mark.anyio
async def test_init_paddle_ocr_initialization_error(backend: PaddleBackend, mock_find_spec: Mock) -> None:
    PaddleBackend._paddle_ocr = None

    async def mock_run_sync_error(*args: Any, **_: Any) -> None:
        if args and args[0].__name__ == "PaddleOCR":
            raise Exception("Initialization error")

    with patch("kreuzberg._ocr._paddleocr.run_sync", side_effect=mock_run_sync_error):
        with pytest.raises(OCRError) as excinfo:
            await backend._init_paddle_ocr()

        assert "Failed to initialize PaddleOCR" in str(excinfo.value)


@pytest.mark.anyio
async def test_process_image(
    backend: PaddleBackend, mock_image: Mock, mock_run_sync: Mock, mock_paddleocr: Mock
) -> None:
    paddle_mock = Mock()

    paddle_mock.ocr.return_value = [
        [
            [
                [[10, 10], [100, 10], [100, 30], [10, 30]],
                ("Sample text 1", 0.95),
            ],
            [
                [[10, 40], [100, 40], [100, 60], [10, 60]],
                ("Sample text 2", 0.90),
            ],
        ]
    ]
    PaddleBackend._paddle_ocr = paddle_mock

    result = await backend.process_image(mock_image)

    assert isinstance(result, ExtractionResult)
    assert result.content == "Sample text 1\nSample text 2"
    assert result.mime_type == "text/plain"
    assert result.metadata.get("width") == 100
    assert result.metadata.get("height") == 100


@pytest.mark.anyio
async def test_process_image_with_options(backend: PaddleBackend, mock_image: Mock, mock_run_sync: Mock) -> None:
    paddle_mock = Mock()

    paddle_mock.ocr.return_value = [
        [
            [
                [[10, 10], [100, 10], [100, 30], [10, 30]],
                ("Sample text 1", 0.95),
            ],
            [
                [[10, 40], [100, 40], [100, 60], [10, 60]],
                ("Sample text 2", 0.90),
            ],
        ]
    ]
    PaddleBackend._paddle_ocr = paddle_mock

    result = await backend.process_image(
        mock_image,
        language="german",
        use_angle_cls=True,
        det_db_thresh=0.4,
        det_db_box_thresh=0.6,
    )

    assert isinstance(result, ExtractionResult)
    assert result.content == "Sample text 1\nSample text 2"


@pytest.mark.anyio
async def test_process_image_error(backend: PaddleBackend, mock_image: Mock) -> None:
    paddle_mock = Mock()

    paddle_mock.ocr.return_value = [
        [
            [
                [[10, 10], [100, 10], [100, 30], [10, 30]],
                ("Sample text 1", 0.95),
            ],
            [
                [[10, 40], [100, 40], [100, 60], [10, 60]],
                ("Sample text 2", 0.90),
            ],
        ]
    ]
    PaddleBackend._paddle_ocr = paddle_mock

    with patch("kreuzberg._ocr._paddleocr.run_sync", side_effect=Exception("OCR processing error")):
        with pytest.raises(OCRError) as excinfo:
            await backend.process_image(mock_image)

        assert "Failed to OCR using PaddleOCR" in str(excinfo.value)


@pytest.mark.anyio
async def test_process_file(backend: PaddleBackend, mock_run_sync: Mock, ocr_image: Path) -> None:
    paddle_mock = Mock()

    paddle_mock.ocr.return_value = [
        [
            [
                [[10, 10], [100, 10], [100, 30], [10, 30]],
                ("Sample text 1", 0.95),
            ],
            [
                [[10, 40], [100, 40], [100, 60], [10, 60]],
                ("Sample text 2", 0.90),
            ],
        ]
    ]
    PaddleBackend._paddle_ocr = paddle_mock

    result = await backend.process_file(ocr_image)

    assert isinstance(result, ExtractionResult)
    assert result.content == "Sample text 1\nSample text 2"


@pytest.mark.anyio
async def test_process_file_with_options(backend: PaddleBackend, mock_run_sync: Mock, ocr_image: Path) -> None:
    paddle_mock = Mock()

    paddle_mock.ocr.return_value = [
        [
            [
                [[10, 10], [100, 10], [100, 30], [10, 30]],
                ("Sample text 1", 0.95),
            ],
            [
                [[10, 40], [100, 40], [100, 60], [10, 60]],
                ("Sample text 2", 0.90),
            ],
        ]
    ]
    PaddleBackend._paddle_ocr = paddle_mock

    result = await backend.process_file(
        ocr_image,
        language="french",
        use_angle_cls=True,
        det_db_thresh=0.4,
    )

    assert isinstance(result, ExtractionResult)
    assert result.content == "Sample text 1\nSample text 2"


@pytest.mark.anyio
async def test_process_file_error(backend: PaddleBackend, ocr_image: Path) -> None:
    paddle_mock = Mock()

    paddle_mock.ocr.return_value = [
        [
            [
                [[10, 10], [100, 10], [100, 30], [10, 30]],
                ("Sample text 1", 0.95),
            ],
            [
                [[10, 40], [100, 40], [100, 60], [10, 60]],
                ("Sample text 2", 0.90),
            ],
        ]
    ]
    PaddleBackend._paddle_ocr = paddle_mock

    with patch("kreuzberg._ocr._paddleocr.run_sync", side_effect=Exception("File processing error")):
        with pytest.raises(OCRError) as excinfo:
            await backend.process_file(ocr_image)

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
        from paddleocr import PaddleOCR  # noqa: F401
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
        from paddleocr import PaddleOCR  # noqa: F401
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
    """Test grayscale to RGB conversion - covers lines 130-133."""
    from unittest.mock import Mock, patch

    from PIL import Image

    backend = PaddleBackend()

    grayscale_image = Image.new("L", (100, 100), color=128)

    with patch.object(backend, "_init_paddle_ocr"):
        backend._paddle_ocr = Mock()  # type: ignore[misc]
        backend._paddle_ocr.ocr.return_value = [[]]

        with patch("kreuzberg._ocr._paddleocr.run_sync") as mock_run_sync:
            mock_run_sync.return_value = [[]]

            await backend.process_image(grayscale_image)

            mock_run_sync.assert_called_once()
            np_array_arg = mock_run_sync.call_args[0][1]

            assert len(np_array_arg.shape) == 3
            assert np_array_arg.shape[2] == 3


@pytest.mark.anyio
async def test_process_paddle_result_current_line_handling() -> None:
    """Test current line handling in result processing - covers lines 198-201."""
    from PIL import Image

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
    """Test image size fallback for mocked images - covers line 218."""
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
async def test_init_paddle_ocr_gpu_memory_conversion() -> None:
    """Test GPU memory limit conversion - covers line 284."""
    from unittest.mock import Mock, patch

    PaddleBackend._paddle_ocr = None

    mock_paddle_ocr_class = Mock()
    mock_paddle_ocr_instance = Mock()
    mock_paddle_ocr_class.return_value = mock_paddle_ocr_instance

    mock_device_info = Mock()
    mock_device_info.device_type = "cuda"

    async def mock_run_sync_func(func: object, *args: object, **kwargs: object) -> object:
        return func(*args, **kwargs)  # type: ignore[operator]

    with (
        patch("kreuzberg._ocr._paddleocr.find_spec", return_value=True),
        patch.dict("sys.modules", {"paddleocr": Mock(PaddleOCR=mock_paddle_ocr_class)}),
        patch.object(PaddleBackend, "_validate_language_code", return_value="en"),
        patch.object(PaddleBackend, "_resolve_device_config", return_value=mock_device_info),
        patch("kreuzberg._ocr._paddleocr.run_sync", side_effect=mock_run_sync_func),
    ):
        await PaddleBackend._init_paddle_ocr(language="en", gpu_memory_limit=2.5)

        mock_paddle_ocr_class.assert_called_once()
        expected_call_kwargs = mock_paddle_ocr_class.call_args[1]
        assert expected_call_kwargs["gpu_mem"] == 2560


@pytest.mark.anyio
async def test_resolve_device_config_deprecated_use_gpu_warnings() -> None:
    """Test deprecated use_gpu warnings - covers lines 312-319, 321."""
    import warnings
    from unittest.mock import patch

    with warnings.catch_warnings(record=True) as w:
        warnings.simplefilter("always")

        with patch("kreuzberg._utils._device.validate_device_request") as mock_validate:
            mock_validate.return_value = Mock(device_type="cpu", name="CPU")

            PaddleBackend._resolve_device_config(use_gpu=True, device="auto")

            assert len(w) == 1
            assert issubclass(w[0].category, DeprecationWarning)
            assert "'use_gpu' parameter is deprecated" in str(w[0].message)

    with warnings.catch_warnings(record=True) as w:
        warnings.simplefilter("always")

        with patch("kreuzberg._utils._device.validate_device_request") as mock_validate:
            mock_validate.return_value = Mock(device_type="cpu", name="CPU")

            PaddleBackend._resolve_device_config(use_gpu=True, device="cpu")

            assert len(w) == 1
            assert issubclass(w[0].category, DeprecationWarning)
            assert "Both 'use_gpu' and 'device' parameters specified" in str(w[0].message)


@pytest.mark.anyio
async def test_resolve_device_config_mps_warning() -> None:
    """Test MPS not supported warning - covers lines 330-335."""
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
    """Test ValidationError fallback for deprecated use_gpu=False - covers lines 345-348."""
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

    with pytest.raises(ValidationError) as excinfo:
        await backend._init_paddle_ocr(language="invalid_language")

    assert "language_code" in excinfo.value.context
    assert excinfo.value.context["language_code"] == "invalid_language"
    assert "supported_languages" in excinfo.value.context

    assert "not supported by PaddleOCR" in str(excinfo.value)
