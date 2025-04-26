from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING, Any
from unittest.mock import AsyncMock, Mock

from kreuzberg.extraction import DEFAULT_CONFIG

if TYPE_CHECKING:
    from collections.abc import Coroutine

    from kreuzberg._types import ExtractionConfig

import pytest

from kreuzberg import ExtractionResult, ValidationError
from kreuzberg._extractors._pandoc import (
    BibliographyExtractor,
    EbookExtractor,
    LaTeXExtractor,
    MarkdownExtractor,
    MiscFormatExtractor,
    OfficeDocumentExtractor,
    PandocExtractor,
    TabularDataExtractor,
    XMLBasedExtractor,
)
from kreuzberg.exceptions import MissingDependencyError, ParsingError

if TYPE_CHECKING:
    from collections.abc import Callable

    from pytest_mock import MockerFixture

SAMPLE_PANDOC_JSON = {
    "pandoc-api-version": [1, 23, 1],
    "meta": {"title": {"t": "MetaString", "c": "Test Document"}, "author": {"t": "MetaString", "c": "Test Author"}},
    "blocks": [],
}


@pytest.fixture
def mock_run_process(mocker: MockerFixture) -> AsyncMock:
    return mocker.patch("kreuzberg._extractors._pandoc.run_process", new_callable=AsyncMock)


@pytest.fixture
def mock_version_check(mocker: MockerFixture) -> None:
    mocker.patch("kreuzberg._extractors._pandoc.PandocExtractor._checked_version", True)


@pytest.fixture
def mock_run_taskgroup(mocker: MockerFixture) -> AsyncMock:
    return mocker.patch("kreuzberg._extractors._pandoc.run_taskgroup", new_callable=AsyncMock)


@pytest.fixture
def test_config() -> ExtractionConfig:
    return DEFAULT_CONFIG


@pytest.mark.anyio
@pytest.mark.parametrize(
    "major_version, should_raise",
    [
        (1, True),
        (2, False),
        (3, False),
    ],
)
async def test_validate_pandoc_version(
    mocker: MockerFixture, mock_run_process: Mock, major_version: int, should_raise: bool, test_config: ExtractionConfig
) -> None:
    extractor = MarkdownExtractor(mime_type="text/x-markdown", config=test_config)
    extractor._checked_version = False

    mock_run_process.return_value.returncode = 0
    mock_run_process.return_value.stderr = b""
    mock_run_process.return_value.stdout = f"pandoc {major_version}.1.0".encode()

    if should_raise:
        with pytest.raises(MissingDependencyError):
            await extractor._validate_pandoc_version()
    else:
        await extractor._validate_pandoc_version()

    mock_run_process.assert_called_with(["pandoc", "--version"])


@pytest.mark.anyio
@pytest.mark.parametrize(
    "major_version, should_raise",
    [
        (1, True),
        (2, False),
        (3, False),
    ],
)
async def test_validate_pandoc_version_short(
    mocker: MockerFixture, mock_run_process: Mock, major_version: int, should_raise: bool, test_config: ExtractionConfig
) -> None:
    extractor = MarkdownExtractor(mime_type="text/x-markdown", config=test_config)
    extractor._checked_version = False

    mock_run_process.return_value.returncode = 0
    mock_run_process.return_value.stderr = b""
    mock_run_process.return_value.stdout = f"pandoc {major_version}.1".encode()

    if should_raise:
        with pytest.raises(MissingDependencyError):
            await extractor._validate_pandoc_version()
    else:
        await extractor._validate_pandoc_version()

    mock_run_process.assert_called_with(["pandoc", "--version"])


@pytest.mark.anyio
@pytest.mark.parametrize(
    "version_output, should_raise",
    [
        ("pandoc.exe 2.11.4\nCompiled with pandoc-types 1.22", False),
        ("pandoc-2.14.1 @ /usr/bin/pandoc", False),
        ("pandoc version 2.5 (revision abc123d)", False),
        ("2.9.2.1\npandoc-types 1.20", False),
        ("pandoc v2.11.4\nCompiled with pandoc-types 1.22", False),
        ("This is the pandoc 2.14 package", False),
        (
            "pandoc 2.11.4\nCompiled with pandoc-types 1.22\nUser data directory: /Users/user/.pandoc",
            False,
        ),
        ("pandoc (version 2.8.1)", False),
        ("2.11.4 [pandoc-dependencies]", False),
    ],
)
async def test_validate_pandoc_version_flexible_formats(
    mocker: MockerFixture,
    mock_run_process: Mock,
    version_output: str,
    should_raise: bool,
    test_config: ExtractionConfig,
) -> None:
    extractor = MarkdownExtractor(mime_type="text/x-markdown", config=test_config)
    extractor._checked_version = False

    mock_run_process.return_value.returncode = 0
    mock_run_process.return_value.stderr = b""
    mock_run_process.return_value.stdout = version_output.encode()

    if should_raise:
        with pytest.raises(MissingDependencyError):
            await extractor._validate_pandoc_version()
    else:
        await extractor._validate_pandoc_version()

    mock_run_process.assert_called_with(["pandoc", "--version"])


@pytest.mark.parametrize(
    "node, expected_output",
    [
        ({"t": "Str", "c": "Hello"}, "Hello"),
        ({"t": "Space", "c": " "}, " "),
        ({"t": "Emph", "c": [{"t": "Str", "c": "Emphasized"}]}, "Emphasized"),
    ],
)
def test_extract_inline_text(node: dict[str, Any], expected_output: str, test_config: ExtractionConfig) -> None:
    extractor = MarkdownExtractor(mime_type="text/x-markdown", config=test_config)
    assert extractor._extract_inline_text(node) == expected_output


@pytest.mark.parametrize(
    "nodes, expected_output",
    [
        ([{"t": "Str", "c": "Hello"}, {"t": "Space", "c": " "}, {"t": "Str", "c": "World"}], "Hello World"),
        ([{"t": "Emph", "c": [{"t": "Str", "c": "Emphasized"}]}], "Emphasized"),
    ],
)
def test_extract_inlines(nodes: list[dict[str, Any]], expected_output: str, test_config: ExtractionConfig) -> None:
    extractor = MarkdownExtractor(mime_type="text/x-markdown", config=test_config)
    assert extractor._extract_inlines(nodes) == expected_output


@pytest.mark.parametrize(
    "node, expected_output",
    [
        ({"t": "MetaString", "c": "Test String"}, "Test String"),
        ({"t": "MetaInlines", "c": [{"t": "Str", "c": "Inline String"}]}, "Inline String"),
        ({"t": "MetaList", "c": [{"t": "MetaString", "c": "List Item"}]}, ["List Item"]),
    ],
)
def test_extract_meta_value(node: Any, expected_output: Any, test_config: ExtractionConfig) -> None:
    extractor = MarkdownExtractor(mime_type="text/x-markdown", config=test_config)
    assert extractor._extract_meta_value(node) == expected_output


@pytest.mark.parametrize(
    "extractor_class, mime_type, expected_type",
    [
        (MarkdownExtractor, "text/x-markdown", "markdown"),
        (OfficeDocumentExtractor, "application/vnd.openxmlformats-officedocument.wordprocessingml.document", "docx"),
        (EbookExtractor, "application/epub+zip", "epub"),
        (LaTeXExtractor, "application/x-latex", "latex"),
        (BibliographyExtractor, "application/x-bibtex", "bibtex"),
        (XMLBasedExtractor, "application/docbook+xml", "docbook"),
        (TabularDataExtractor, "text/csv", "csv"),
        (MiscFormatExtractor, "application/rtf", "rtf"),
    ],
)
def test_get_pandoc_type_from_mime_type(
    extractor_class: type[PandocExtractor], mime_type: str, expected_type: str, test_config: ExtractionConfig
) -> None:
    extractor = extractor_class(mime_type=mime_type, config=test_config)
    assert extractor._get_pandoc_type_from_mime_type(mime_type) == expected_type


@pytest.fixture(autouse=True)
def mock_pandoc_version(mocker: MockerFixture) -> None:
    mocker.patch("kreuzberg._extractors._pandoc.PandocExtractor._checked_version", True)


@pytest.fixture
def mock_temp_file(mocker: MockerFixture) -> None:
    async def mock_create(_: Any) -> tuple[str, Callable[[], Coroutine[None, None, None]]]:
        async def mock_unlink() -> None:
            pass

        return "/tmp/test", mock_unlink

    mocker.patch("kreuzberg._extractors._pandoc.create_temp_file", side_effect=mock_create)


@pytest.fixture
def mock_async_path(mocker: MockerFixture) -> None:
    mock_path = mocker.patch("kreuzberg._extractors._pandoc.AsyncPath")
    mock_path.return_value.read_text = mocker.AsyncMock(return_value="Test content")
    mock_path.return_value.write_bytes = mocker.AsyncMock()


@pytest.mark.anyio
async def test_handle_extract_file(
    mock_run_process: Mock, mock_temp_file: None, mock_async_path: None, test_config: ExtractionConfig
) -> None:
    extractor = MarkdownExtractor(mime_type="text/x-markdown", config=test_config)
    mock_run_process.return_value.returncode = 0
    mock_run_process.return_value.stdout = b"Test content"

    result = await extractor._handle_extract_file(Path("/tmp/test"))
    assert isinstance(result, str)


@pytest.mark.anyio
async def test_extract_path_async(
    mock_version_check: None,
    mock_run_taskgroup: AsyncMock,
    mock_temp_file: None,
    mock_async_path: None,
    test_config: ExtractionConfig,
) -> None:
    extractor = MarkdownExtractor(mime_type="text/x-markdown", config=test_config)

    mock_run_taskgroup.return_value = ({"title": "Test Document"}, "Test Content")

    result = await extractor.extract_path_async(Path("/tmp/test"))
    assert isinstance(result, ExtractionResult)
    assert result.metadata["title"] == "Test Document"
    assert result.content == "Test Content"

    assert mock_run_taskgroup.called


@pytest.mark.anyio
async def test_extract_bytes_async(
    mock_version_check: None,
    mock_run_taskgroup: AsyncMock,
    mock_temp_file: None,
    mock_async_path: None,
    test_config: ExtractionConfig,
) -> None:
    extractor = MarkdownExtractor(mime_type="text/x-markdown", config=test_config)

    mock_run_taskgroup.return_value = ({"title": "Test Document"}, "Test Content")

    result = await extractor.extract_bytes_async(b"Test Content")
    assert isinstance(result, ExtractionResult)
    assert result.metadata["title"] == "Test Document"
    assert result.content == "Test Content"

    assert mock_run_taskgroup.called


@pytest.mark.anyio
async def test_validate_pandoc_version_file_not_found(mocker: MockerFixture, test_config: ExtractionConfig) -> None:
    extractor = MarkdownExtractor(mime_type="text/x-markdown", config=test_config)
    extractor._checked_version = False

    mock_run = mocker.patch("kreuzberg._extractors._pandoc.run_process", new_callable=AsyncMock)
    mock_run.side_effect = FileNotFoundError()

    with pytest.raises(MissingDependencyError) as excinfo:
        await extractor._validate_pandoc_version()

    error_message = str(excinfo.value)
    assert "Pandoc version 2" in error_message
    assert "required" in error_message

    assert mock_run.called


@pytest.mark.anyio
async def test_validate_pandoc_version_invalid_output(mocker: MockerFixture, test_config: ExtractionConfig) -> None:
    extractor = MarkdownExtractor(mime_type="text/x-markdown", config=test_config)
    extractor._checked_version = False

    mock_run = mocker.patch("kreuzberg._extractors._pandoc.run_process", new_callable=AsyncMock)

    mock_return = Mock()
    mock_return.stdout = b"invalid version output"
    mock_run.return_value = mock_return

    with pytest.raises(MissingDependencyError) as excinfo:
        await extractor._validate_pandoc_version()

    error_message = str(excinfo.value)
    assert "Pandoc version 2" in error_message
    assert "required" in error_message

    assert mock_run.called


@pytest.mark.anyio
async def test_validate_pandoc_version_parse_error(mocker: MockerFixture, test_config: ExtractionConfig) -> None:
    extractor = MarkdownExtractor(mime_type="text/x-markdown", config=test_config)
    extractor._checked_version = False

    mock_run = mocker.patch("kreuzberg._extractors._pandoc.run_process", new_callable=AsyncMock)

    mock_return = Mock()
    mock_return.stdout = b"pandoc abc"
    mock_run.return_value = mock_return

    with pytest.raises(MissingDependencyError) as excinfo:
        await extractor._validate_pandoc_version()

    error_message = str(excinfo.value)
    assert "Pandoc version 2" in error_message
    assert "required" in error_message

    assert mock_run.called


@pytest.mark.anyio
async def test_handle_extract_metadata_runtime_error(
    mock_run_process: AsyncMock, mock_temp_file: None, mock_async_path: None, test_config: ExtractionConfig
) -> None:
    extractor = MarkdownExtractor(mime_type="text/x-markdown", config=test_config)
    mock_run_process.side_effect = RuntimeError("Test error")

    with pytest.raises(ParsingError):
        await extractor._handle_extract_metadata(Path("/tmp/test"))

    assert mock_run_process.called


@pytest.mark.anyio
async def test_handle_extract_file_runtime_error(
    mock_run_process: AsyncMock, mock_temp_file: None, mock_async_path: None, test_config: ExtractionConfig
) -> None:
    extractor = MarkdownExtractor(mime_type="text/x-markdown", config=test_config)
    mock_run_process.side_effect = RuntimeError("Test error")

    with pytest.raises(ParsingError):
        await extractor._handle_extract_file(Path("/tmp/test"))

    assert mock_run_process.called


@pytest.mark.anyio
async def test_extract_bytes_async_runtime_error(
    mock_version_check: None,
    mock_temp_file: None,
    mock_async_path: None,
    mock_run_process: AsyncMock,
    test_config: ExtractionConfig,
) -> None:
    extractor = MarkdownExtractor(mime_type="text/x-markdown", config=test_config)
    mock_run_process.side_effect = RuntimeError("Test error")

    with pytest.raises(ParsingError):
        await extractor.extract_bytes_async(b"Test content")

    assert mock_run_process.called


def test_get_pandoc_type_unsupported_mime(test_config: ExtractionConfig) -> None:
    extractor = MarkdownExtractor(mime_type="text/x-markdown", config=test_config)
    with pytest.raises(ValidationError):
        extractor._get_pandoc_type_from_mime_type("unsupported/mime-type")


def test_get_pandoc_type_prefix_match(test_config: ExtractionConfig) -> None:
    extractor = MarkdownExtractor(mime_type="text/markdown", config=test_config)
    assert extractor._get_pandoc_type_from_mime_type("text/markdown") == "markdown"


@pytest.mark.anyio
async def test_handle_extract_metadata_error(
    mock_run_process: AsyncMock, mock_temp_file: None, mock_async_path: None, test_config: ExtractionConfig
) -> None:
    extractor = MarkdownExtractor(mime_type="text/x-markdown", config=test_config)

    mock_return = Mock()
    mock_return.returncode = 1
    mock_return.stderr = b"Test error"
    mock_run_process.return_value = mock_return

    with pytest.raises(ParsingError):
        await extractor._handle_extract_metadata(Path("/tmp/test"))

    assert mock_run_process.called


@pytest.mark.anyio
async def test_handle_extract_file_error(
    mock_run_process: AsyncMock, mock_temp_file: None, mock_async_path: None, test_config: ExtractionConfig
) -> None:
    extractor = MarkdownExtractor(mime_type="text/x-markdown", config=test_config)

    mock_return = Mock()
    mock_return.returncode = 1
    mock_return.stderr = b"Test error"
    mock_run_process.return_value = mock_return

    with pytest.raises(ParsingError):
        await extractor._handle_extract_file(Path("/tmp/test"))

    assert mock_run_process.called


@pytest.mark.anyio
async def test_extract_bytes_async_error(
    mock_version_check: None,
    mock_temp_file: None,
    mock_async_path: None,
    mock_run_process: AsyncMock,
    test_config: ExtractionConfig,
) -> None:
    extractor = MarkdownExtractor(mime_type="text/x-markdown", config=test_config)

    mock_return = Mock()
    mock_return.returncode = 1
    mock_return.stderr = b"Test error"
    mock_run_process.return_value = mock_return

    with pytest.raises(ParsingError):
        await extractor.extract_bytes_async(b"Test content")

    assert mock_run_process.called
