from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING

import pytest

from kreuzberg import ExtractionConfig, ImageExtractionConfig
from kreuzberg._extractors._legacy_office import LegacyPresentationExtractor, LegacyWordExtractor
from kreuzberg._mime_types import LEGACY_POWERPOINT_MIME_TYPE, LEGACY_WORD_MIME_TYPE
from kreuzberg.exceptions import MissingDependencyError, ParsingError
from kreuzberg.extraction import DEFAULT_CONFIG

if TYPE_CHECKING:
    from pytest_mock import MockerFixture


@pytest.fixture(scope="session")
def word_extractor() -> LegacyWordExtractor:
    return LegacyWordExtractor(mime_type=LEGACY_WORD_MIME_TYPE, config=DEFAULT_CONFIG)


@pytest.fixture(scope="session")
def presentation_extractor() -> LegacyPresentationExtractor:
    return LegacyPresentationExtractor(mime_type=LEGACY_POWERPOINT_MIME_TYPE, config=DEFAULT_CONFIG)


@pytest.fixture(scope="session")
def test_doc_file() -> Path:
    test_file = Path("test_documents/legacy_office/unit_test_lists.doc")
    if not test_file.exists():
        pytest.skip(f"Test file not found: {test_file}")
    return test_file


@pytest.fixture(scope="session")
def test_ppt_file() -> Path:
    test_file = Path("test_documents/legacy_office/simple.ppt")
    if not test_file.exists():
        pytest.skip(f"Test file not found: {test_file}")
    return test_file


def test_word_extract_bytes_sync_basic(word_extractor: LegacyWordExtractor, test_doc_file: Path) -> None:
    content = test_doc_file.read_bytes()
    result = word_extractor.extract_bytes_sync(content)

    assert result.mime_type == "text/markdown"
    assert len(result.content) > 0
    assert result.metadata["source_format"] == "doc"
    assert result.metadata["converted_via"] == "libreoffice"


def test_word_extract_path_sync_basic(word_extractor: LegacyWordExtractor, test_doc_file: Path) -> None:
    result = word_extractor.extract_path_sync(test_doc_file)

    assert result.mime_type == "text/markdown"
    assert len(result.content) > 0
    assert result.metadata["source_format"] == "doc"
    assert result.metadata["converted_via"] == "libreoffice"


@pytest.mark.anyio
async def test_word_extract_bytes_async_basic(word_extractor: LegacyWordExtractor, test_doc_file: Path) -> None:
    content = test_doc_file.read_bytes()
    result = await word_extractor.extract_bytes_async(content)

    assert result.mime_type == "text/markdown"
    assert len(result.content) > 0
    assert result.metadata["source_format"] == "doc"
    assert result.metadata["converted_via"] == "libreoffice"


@pytest.mark.anyio
async def test_word_extract_path_async_basic(word_extractor: LegacyWordExtractor, test_doc_file: Path) -> None:
    result = await word_extractor.extract_path_async(test_doc_file)

    assert result.mime_type == "text/markdown"
    assert len(result.content) > 0
    assert result.metadata["source_format"] == "doc"
    assert result.metadata["converted_via"] == "libreoffice"


def test_presentation_extract_bytes_sync_basic(
    presentation_extractor: LegacyPresentationExtractor, test_ppt_file: Path
) -> None:
    content = test_ppt_file.read_bytes()
    result = presentation_extractor.extract_bytes_sync(content)

    assert result.mime_type == "text/markdown"
    assert len(result.content) > 0
    assert result.metadata["source_format"] == "ppt"
    assert result.metadata["converted_via"] == "libreoffice"


def test_presentation_extract_path_sync_basic(
    presentation_extractor: LegacyPresentationExtractor, test_ppt_file: Path
) -> None:
    result = presentation_extractor.extract_path_sync(test_ppt_file)

    assert result.mime_type == "text/markdown"
    assert len(result.content) > 0
    assert result.metadata["source_format"] == "ppt"
    assert result.metadata["converted_via"] == "libreoffice"


@pytest.mark.anyio
async def test_presentation_extract_bytes_async_basic(
    presentation_extractor: LegacyPresentationExtractor, test_ppt_file: Path
) -> None:
    content = test_ppt_file.read_bytes()
    result = await presentation_extractor.extract_bytes_async(content)

    assert result.mime_type == "text/markdown"
    assert len(result.content) > 0
    assert result.metadata["source_format"] == "ppt"
    assert result.metadata["converted_via"] == "libreoffice"


@pytest.mark.anyio
async def test_presentation_extract_path_async_basic(
    presentation_extractor: LegacyPresentationExtractor, test_ppt_file: Path
) -> None:
    result = await presentation_extractor.extract_path_async(test_ppt_file)

    assert result.mime_type == "text/markdown"
    assert len(result.content) > 0
    assert result.metadata["source_format"] == "ppt"
    assert result.metadata["converted_via"] == "libreoffice"


def test_word_content_extraction(word_extractor: LegacyWordExtractor, test_doc_file: Path) -> None:
    result = word_extractor.extract_path_sync(test_doc_file)

    assert "list" in result.content.lower() or "test" in result.content.lower()
    assert len(result.content) > 50


def test_presentation_content_extraction(
    presentation_extractor: LegacyPresentationExtractor, test_ppt_file: Path
) -> None:
    result = presentation_extractor.extract_path_sync(test_ppt_file)

    assert len(result.content) > 10
    assert "slide" in result.content.lower() or len(result.content) > 20


def test_word_mime_type_support() -> None:
    assert LegacyWordExtractor.supports_mimetype(LEGACY_WORD_MIME_TYPE)
    assert not LegacyWordExtractor.supports_mimetype("application/pdf")


def test_presentation_mime_type_support() -> None:
    assert LegacyPresentationExtractor.supports_mimetype(LEGACY_POWERPOINT_MIME_TYPE)
    assert not LegacyPresentationExtractor.supports_mimetype("application/pdf")


def test_word_with_config() -> None:
    config = ExtractionConfig(images=ImageExtractionConfig())
    extractor = LegacyWordExtractor(mime_type=LEGACY_WORD_MIME_TYPE, config=config)
    assert extractor.config.images is not None


def test_presentation_with_config() -> None:
    config = ExtractionConfig(images=ImageExtractionConfig())
    extractor = LegacyPresentationExtractor(mime_type=LEGACY_POWERPOINT_MIME_TYPE, config=config)
    assert extractor.config.images is not None


@pytest.mark.anyio
async def test_libreoffice_not_installed(mocker: MockerFixture, test_doc_file: Path) -> None:
    mocker.patch("shutil.which", return_value=None)

    word_extractor = LegacyWordExtractor(mime_type=LEGACY_WORD_MIME_TYPE, config=DEFAULT_CONFIG)

    with pytest.raises(MissingDependencyError, match="soffice"):
        await word_extractor.extract_path_async(test_doc_file)


@pytest.mark.anyio
async def test_conversion_failure_unsupported_format(mocker: MockerFixture, test_doc_file: Path) -> None:
    mock_result = mocker.Mock()
    mock_result.returncode = 1
    mock_result.stderr = b"Error: unsupported format"
    mock_result.stdout = b""

    mocker.patch("kreuzberg._utils._libreoffice.run_process", return_value=mock_result)

    word_extractor = LegacyWordExtractor(mime_type=LEGACY_WORD_MIME_TYPE, config=DEFAULT_CONFIG)

    with pytest.raises(ParsingError, match="unsupported"):
        await word_extractor.extract_path_async(test_doc_file)


@pytest.mark.anyio
async def test_conversion_failure_general_error(mocker: MockerFixture, test_doc_file: Path) -> None:
    mock_result = mocker.Mock()
    mock_result.returncode = 127
    mock_result.stderr = b"Command not found"
    mock_result.stdout = b""

    mocker.patch("kreuzberg._utils._libreoffice.run_process", return_value=mock_result)

    word_extractor = LegacyWordExtractor(mime_type=LEGACY_WORD_MIME_TYPE, config=DEFAULT_CONFIG)

    with pytest.raises(OSError, match="return code 127"):
        await word_extractor.extract_path_async(test_doc_file)


@pytest.mark.anyio
async def test_output_file_not_created(mocker: MockerFixture, test_doc_file: Path) -> None:
    mock_result = mocker.Mock()
    mock_result.returncode = 0
    mock_result.stderr = b""
    mock_result.stdout = b"Conversion completed"

    mocker.patch("kreuzberg._utils._libreoffice.run_process", return_value=mock_result)

    mock_exists = mocker.patch("anyio.Path.exists")
    mock_exists.return_value = False

    word_extractor = LegacyWordExtractor(mime_type=LEGACY_WORD_MIME_TYPE, config=DEFAULT_CONFIG)

    with pytest.raises(ParsingError, match="output file not found"):
        await word_extractor.extract_path_async(test_doc_file)


@pytest.mark.anyio
async def test_empty_output_file(mocker: MockerFixture, test_doc_file: Path) -> None:
    mock_result = mocker.Mock()
    mock_result.returncode = 0
    mock_result.stderr = b""
    mock_result.stdout = b"Conversion completed"

    mocker.patch("kreuzberg._utils._libreoffice.run_process", return_value=mock_result)

    mock_exists = mocker.patch("anyio.Path.exists")
    mock_exists.return_value = True

    mock_stat = mocker.Mock()
    mock_stat.st_size = 0
    mocker.patch("anyio.Path.stat", return_value=mock_stat)

    word_extractor = LegacyWordExtractor(mime_type=LEGACY_WORD_MIME_TYPE, config=DEFAULT_CONFIG)

    with pytest.raises(ParsingError, match="empty file"):
        await word_extractor.extract_path_async(test_doc_file)


@pytest.mark.anyio
@pytest.mark.timeout(15)
async def test_conversion_timeout(mocker: MockerFixture, test_doc_file: Path) -> None:
    import tempfile

    import anyio

    from kreuzberg._utils._libreoffice import convert_office_doc

    async def slow_process(*args: object, **kwargs: object) -> object:
        await anyio.sleep(10)
        mock_result = mocker.Mock()
        mock_result.returncode = 0
        return mock_result

    mocker.patch("kreuzberg._utils._libreoffice.run_process", side_effect=slow_process)

    temp_dir = Path(tempfile.mkdtemp(prefix="test_timeout_"))
    with pytest.raises(ParsingError, match="timed out"):
        await convert_office_doc(test_doc_file, temp_dir, "docx", timeout=0.1)
