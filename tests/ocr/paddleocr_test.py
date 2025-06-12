from __future__ import annotations

from typing import TYPE_CHECKING, Any
from unittest.mock import Mock, patch

import numpy as np
import pytest
from PIL import Image

from kreuzberg._ocr._paddleocr import PaddleOCRBackend, PaddleOCRConfig
from kreuzberg._types import ExtractionResult
from kreuzberg.exceptions import MissingDependencyError, OCRError, ValidationError

if TYPE_CHECKING:
    from pathlib import Path

    from pytest_mock import MockerFixture


@pytest.fixture
def config() -> PaddleOCRConfig:
    return PaddleOCRConfig()


@pytest.fixture
def backend(config: PaddleOCRConfig) -> PaddleOCRBackend:
    return PaddleOCRBackend(config)


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


@pytest.mark.anyio
async def test_is_mkldnn_supported(mocker: MockerFixture) -> None:
    mocker.patch("platform.system", return_value="Linux")
    mocker.patch("platform.processor", return_value="x86_64")
    mocker.patch("platform.machine", return_value="x86_64")
    assert PaddleOCRBackend._is_mkldnn_supported() is True

    mocker.patch("platform.system", return_value="Windows")
    mocker.patch("platform.processor", return_value="Intel64 Family 6")
    assert PaddleOCRBackend._is_mkldnn_supported() is True

    mocker.patch("platform.system", return_value="Darwin")
    mocker.patch("platform.machine", return_value="x86_64")
    assert PaddleOCRBackend._is_mkldnn_supported() is True

    mocker.patch("platform.system", return_value="Darwin")
    mocker.patch("platform.machine", return_value="arm64")
    assert PaddleOCRBackend._is_mkldnn_supported() is False

    mocker.patch("platform.system", return_value="FreeBSD")
    assert PaddleOCRBackend._is_mkldnn_supported() is False

    mocker.patch("platform.system", return_value="Windows")
    mocker.patch("platform.processor", return_value="AMD64")
    mocker.patch("platform.machine", return_value="AMD64")
    assert PaddleOCRBackend._is_mkldnn_supported() is True

    mocker.patch("platform.system", return_value="Linux")
    mocker.patch("platform.processor", return_value="aarch64")
    mocker.patch("platform.machine", return_value="aarch64")
    assert PaddleOCRBackend._is_mkldnn_supported() is False


@pytest.mark.anyio
async def test_init_paddle_ocr(
    backend: PaddleOCRBackend, mock_paddleocr: Mock, mock_run_sync: Mock, mock_find_spec: Mock
) -> None:
    backend._ocr = None

    await backend._init_paddle_ocr()

    assert backend._ocr is not None
    mock_paddleocr.assert_called_once_with(
        use_angle_cls=False,
        lang="en",
        use_gpu=False,
        show_log=False,
        enable_mkldnn=True,
    )


@pytest.mark.anyio
async def test_init_paddle_ocr_with_gpu_package(
    backend: PaddleOCRBackend, mock_paddleocr: Mock, mock_run_sync: Mock, mock_find_spec: Mock, mocker: MockerFixture
) -> None:
    backend._ocr = None
    mocker.patch("kreuzberg._ocr._paddleocr.find_spec", return_value=Mock())

    await backend._init_paddle_ocr()

    mock_paddleocr.assert_called_once_with(
        use_angle_cls=False,
        lang="en",
        use_gpu=True,
        show_log=False,
        enable_mkldnn=True,
    )


@pytest.mark.anyio
async def test_init_paddle_ocr_with_language(
    backend: PaddleOCRBackend, mock_paddleocr: Mock, mock_run_sync: Mock, mock_find_spec: Mock
) -> None:
    backend._ocr = None

    await backend._init_paddle_ocr(language="french")

    mock_paddleocr.assert_called_once_with(
        use_angle_cls=False,
        lang="french",
        use_gpu=False,
        show_log=False,
        enable_mkldnn=True,
    )


@pytest.mark.anyio
async def test_init_paddle_ocr_with_custom_options(
    backend: PaddleOCRBackend, mock_paddleocr: Mock, mock_run_sync: Mock, mock_find_spec: Mock
) -> None:
    backend._ocr = None

    await backend._init_paddle_ocr(
        language="french",
        use_angle_cls=True,
        det_db_thresh=0.4,
        det_db_box_thresh=0.5,
        det_db_unclip_ratio=1.6,
        max_batch_size=10,
        use_dilation=True,
        det_limit_side_len=960,
        det_limit_type="min",
    )

    mock_paddleocr.assert_called_once_with(
        use_angle_cls=True,
        lang="french",
        use_gpu=False,
        show_log=False,
        enable_mkldnn=True,
        det_db_thresh=0.4,
        det_db_box_thresh=0.5,
        det_db_unclip_ratio=1.6,
        max_batch_size=10,
        use_dilation=True,
        det_limit_side_len=960,
        det_limit_type="min",
    )


@pytest.mark.anyio
async def test_init_paddle_ocr_with_model_dirs(
    backend: PaddleOCRBackend, mock_paddleocr: Mock, mock_run_sync: Mock, mock_find_spec: Mock
) -> None:
    backend._ocr = None

    await backend._init_paddle_ocr(
        language="french",
        det_model_dir="custom/det",
        cls_model_dir="custom/cls",
        rec_model_dir="custom/rec",
    )

    mock_paddleocr.assert_called_once_with(
        use_angle_cls=False,
        lang="french",
        use_gpu=False,
        show_log=False,
        enable_mkldnn=True,
        det_model_dir="custom/det",
        cls_model_dir="custom/cls",
        rec_model_dir="custom/rec",
    )


@pytest.mark.anyio
async def test_init_paddle_ocr_missing_dependency(backend: PaddleOCRBackend, mock_find_spec_missing: Mock) -> None:
    backend._ocr = None

    def mock_import(name: str, *args: Any, **kwargs: Any) -> Any:
        if name == "paddleocr":
            raise ImportError("No module named 'paddleocr'")
        return Mock()

    with patch("builtins.__import__", side_effect=mock_import):
        with pytest.raises(OCRError) as excinfo:
            await backend._init_paddle_ocr()

        assert "PaddleOCR is not installed" in str(excinfo.value)


@pytest.mark.anyio
async def test_init_paddle_ocr_initialization_error(backend: PaddleOCRBackend, mock_find_spec: Mock) -> None:
    backend._ocr = None

    async def mock_run_sync_error(*args: Any, **_: Any) -> None:
        raise Exception("Initialization error")

    with patch("kreuzberg._ocr._paddleocr.run_sync", side_effect=mock_run_sync_error):
        with pytest.raises(OCRError) as excinfo:
            await backend._init_paddle_ocr()

        assert "Failed to initialize PaddleOCR" in str(excinfo.value)


@pytest.mark.anyio
async def test_process_image(
    backend: PaddleOCRBackend, mock_image: Mock, mock_run_sync: Mock, mock_paddleocr: Mock
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
    backend._ocr = paddle_mock

    result = await backend.process_image(mock_image)

    assert isinstance(result, ExtractionResult)
    assert "Sample text 1 Sample text 2" in result.content

    paddle_mock.ocr.assert_called_once_with(mock_image, cls=True)


@pytest.mark.anyio
async def test_process_image_with_options(backend: PaddleOCRBackend, mock_image: Mock, mock_run_sync: Mock) -> None:
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
    backend._ocr = paddle_mock

    result = await backend.process_image(
        mock_image,
        language="french",
        use_angle_cls=True,
        det_db_thresh=0.4,
    )

    assert isinstance(result, ExtractionResult)
    assert "Sample text 1 Sample text 2" in result.content


@pytest.mark.anyio
async def test_process_image_error(backend: PaddleOCRBackend, mock_image: Mock) -> None:
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
    backend._ocr = paddle_mock

    with patch("kreuzberg._ocr._paddleocr.run_sync", side_effect=Exception("OCR processing error")):
        with pytest.raises(OCRError) as excinfo:
            await backend.process_image(mock_image)

        assert "Failed to OCR using PaddleOCR" in str(excinfo.value)


@pytest.mark.anyio
async def test_process_file(backend: PaddleOCRBackend, mock_run_sync: Mock, ocr_image: Path) -> None:
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
    backend._ocr = paddle_mock

    result = await backend.process_file(ocr_image)

    assert isinstance(result, ExtractionResult)
    assert "Sample text 1 Sample text 2" in result.content


@pytest.mark.anyio
async def test_process_file_with_options(backend: PaddleOCRBackend, mock_run_sync: Mock, ocr_image: Path) -> None:
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
    backend._ocr = paddle_mock

    result = await backend.process_file(
        ocr_image,
        language="french",
        use_angle_cls=True,
        det_db_thresh=0.4,
    )

    assert isinstance(result, ExtractionResult)
    assert "Sample text 1 Sample text 2" in result.content


@pytest.mark.anyio
async def test_process_file_error(backend: PaddleOCRBackend, ocr_image: Path) -> None:
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
    backend._ocr = paddle_mock

    with patch("kreuzberg._ocr._paddleocr.run_sync", side_effect=Exception("File processing error")):
        with pytest.raises(OCRError) as excinfo:
            await backend.process_file(ocr_image)

        assert "Failed to load or process image using PaddleOCR" in str(excinfo.value)


@pytest.mark.anyio
async def test_process_paddle_result_empty() -> None:
    image = Mock(spec=Image.Image)
    image.size = (100, 100)

    result = PaddleOCRBackend._process_paddle_result([], image)

    assert isinstance(result, ExtractionResult)
    assert result.content == ""

    assert isinstance(result.metadata, dict)
    assert result.metadata.get("width") == 100
    assert result.metadata.get("height") == 100


@pytest.mark.anyio
async def test_process_paddle_result_empty_page() -> None:
    image = Mock(spec=Image.Image)
    image.size = (100, 100)

    result = PaddleOCRBackend._process_paddle_result([[]], image)

    assert isinstance(result, ExtractionResult)
    assert result.content == ""
    assert result.metadata.get("width") == 100
    assert result.metadata.get("height") == 100


@pytest.mark.anyio
async def test_process_paddle_result_complex() -> None:
    image = Mock(spec=Image.Image)
    image.size = (200, 200)

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

    result = PaddleOCRBackend._process_paddle_result(paddle_result, image)

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
    image.size = (100, 100)

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

    result = PaddleOCRBackend._process_paddle_result(paddle_result, image)

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

    result = PaddleOCRBackend._process_paddle_result(paddle_result, image)

    assert isinstance(result, ExtractionResult)
    assert "Same line 1 Same line 2" in result.content
    assert "Different line" in result.content


@pytest.mark.anyio
async def test_integration_process_file(backend: PaddleOCRBackend, ocr_image: Path) -> None:
    try:
        from paddleocr import PaddleOCR  # noqa: F401
    except ImportError:
        pytest.skip("PaddleOCR not installed")

    import platform

    if platform.system() == "Darwin" and platform.machine() == "arm64":
        pytest.skip("Test not applicable on Mac M1/ARM architecture")

    try:
        result = await backend.process_file(ocr_image)
        assert isinstance(result, ExtractionResult)
        assert result.content.strip()
    except (MissingDependencyError, OCRError):
        pytest.skip("PaddleOCR not properly installed or configured")


@pytest.mark.anyio
async def test_integration_process_image(backend: PaddleOCRBackend, ocr_image: Path) -> None:
    try:
        from paddleocr import PaddleOCR  # noqa: F401
    except ImportError:
        pytest.skip("PaddleOCR not installed")

    import platform

    if platform.system() == "Darwin" and platform.machine() == "arm64":
        pytest.skip("Test not applicable on Mac M1/ARM architecture")

    try:
        image = Image.open(ocr_image)
        with image:
            result = await backend.process_image(image)
            assert isinstance(result, ExtractionResult)
            assert result.content.strip()
    except (MissingDependencyError, OCRError):
        pytest.skip("PaddleOCR not properly installed or configured")


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
    result = PaddleOCRBackend._validate_language_code(language_code)
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
        PaddleOCRBackend._validate_language_code(invalid_language_code)

    assert "language_code" in excinfo.value.context
    assert excinfo.value.context["language_code"] == invalid_language_code
    assert "supported_languages" in excinfo.value.context

    assert "not supported by PaddleOCR" in str(excinfo.value)


@pytest.mark.anyio
async def test_init_paddle_ocr_with_invalid_language(
    backend: PaddleOCRBackend, mock_find_spec: Mock, mocker: MockerFixture
) -> None:
    backend._ocr = None

    with pytest.raises(OCRError) as excinfo:
        await backend._init_paddle_ocr(language="invalid")

    assert "Invalid language code" in str(excinfo.value)
