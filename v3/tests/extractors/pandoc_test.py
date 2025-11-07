from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from typing import TYPE_CHECKING, Any
from unittest.mock import AsyncMock, Mock, patch

from kreuzberg.extraction import DEFAULT_CONFIG

if TYPE_CHECKING:
    from collections.abc import Coroutine

    from kreuzberg._types import ExtractionConfig

import pytest

if sys.version_info >= (3, 11):
    from builtins import ExceptionGroup
else:
    ExceptionGroup = None

from kreuzberg import ExtractionResult, ValidationError
from kreuzberg._extractors._pandoc import (
    BibliographyExtractor,
    EbookExtractor,
    LaTeXExtractor,
    MarkdownExtractor,
    MiscFormatExtractor,
    OfficeDocumentExtractor,
    PandocExtractor,
    StructuredTextExtractor,
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
    async def mock_create(
        extension: str, content: bytes | None = None
    ) -> tuple[Path, Callable[[], Coroutine[None, None, None]]]:
        async def mock_unlink() -> None:
            pass

        return Path("/tmp/test"), mock_unlink

    mocker.patch("kreuzberg._utils._tmp.create_temp_file", side_effect=mock_create)


@pytest.fixture
def mock_async_path(mocker: MockerFixture) -> None:
    # Mock AsyncPath operations for pandoc tests - legitimately needed due to complex temp file workflows ~keep
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

    with pytest.raises(RuntimeError):
        await extractor._handle_extract_metadata(Path("/tmp/test"))

    assert mock_run_process.called


@pytest.mark.anyio
async def test_handle_extract_file_runtime_error(
    mock_run_process: AsyncMock, mock_temp_file: None, mock_async_path: None, test_config: ExtractionConfig
) -> None:
    extractor = MarkdownExtractor(mime_type="text/x-markdown", config=test_config)
    mock_run_process.side_effect = RuntimeError("Test error")

    with pytest.raises(RuntimeError):
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


def test_pandoc_core_supported_mime_types_mapping(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)

    assert (
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        in extractor.MIMETYPE_TO_PANDOC_TYPE_MAPPING
    )
    assert "text/x-markdown" in extractor.MIMETYPE_TO_PANDOC_TYPE_MAPPING
    assert "application/epub+zip" in extractor.MIMETYPE_TO_PANDOC_TYPE_MAPPING

    assert (
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        in extractor.MIMETYPE_TO_FILE_EXTENSION_MAPPING
    )
    assert (
        extractor.MIMETYPE_TO_FILE_EXTENSION_MAPPING[
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        ]
        == "docx"
    )


def test_pandoc_core_get_pandoc_type_from_mime_type_valid(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)

    assert extractor._get_pandoc_type_from_mime_type("text/x-markdown") == "markdown"
    assert (
        extractor._get_pandoc_type_from_mime_type(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        )
        == "docx"
    )
    assert extractor._get_pandoc_type_from_mime_type("text/markdown") == "markdown"


def test_pandoc_core_get_pandoc_type_from_mime_type_invalid(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)

    with pytest.raises(ValidationError, match="Unsupported mime type"):
        extractor._get_pandoc_type_from_mime_type("application/unknown")


def test_pandoc_core_get_pandoc_key_mappings() -> None:
    assert PandocExtractor._get_pandoc_key("abstract") == "summary"
    assert PandocExtractor._get_pandoc_key("date") == "created_at"
    assert PandocExtractor._get_pandoc_key("author") == "authors"
    assert PandocExtractor._get_pandoc_key("contributors") == "authors"
    assert PandocExtractor._get_pandoc_key("institute") == "organization"
    assert PandocExtractor._get_pandoc_key("title") == "title"
    assert PandocExtractor._get_pandoc_key("unknown_key") is None


@pytest.mark.anyio
async def test_pandoc_core_extract_bytes_async_complete(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    content = b"# Test Markdown\n\nThis is a test."

    with (
        patch.object(extractor, "_get_pandoc_type_from_mime_type", return_value="markdown"),
        patch("kreuzberg._utils._tmp.create_temp_file") as mock_temp_file,
        patch.object(extractor, "extract_path_async") as mock_extract_path,
    ):
        mock_unlink = AsyncMock()
        temp_path = "/tmp/test.md"
        mock_temp_file.return_value = (temp_path, mock_unlink)

        mock_result = ExtractionResult(content="Test", mime_type="text/x-markdown", metadata={})
        mock_extract_path.return_value = mock_result

        result = await extractor.extract_bytes_async(content)

        assert result == mock_result
        mock_temp_file.assert_called_once_with(".markdown", content)
        mock_extract_path.assert_called_once_with(temp_path)
        mock_unlink.assert_called_once()


def test_pandoc_core_extract_bytes_sync_complete(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    content = b"# Test Markdown\n\nThis is a test."

    with (
        patch.object(extractor, "_get_pandoc_type_from_mime_type", return_value="markdown"),
        patch("tempfile.mkstemp") as mock_mkstemp,
        patch("os.fdopen") as mock_fdopen,
        patch.object(extractor, "extract_path_sync") as mock_extract_path,
        patch("pathlib.Path.unlink") as mock_unlink,
    ):
        mock_fd = 3
        temp_path = "/tmp/test.md"
        mock_mkstemp.return_value = (mock_fd, temp_path)

        mock_file = Mock()
        mock_fdopen.return_value.__enter__.return_value = mock_file

        mock_result = ExtractionResult(content="Test", mime_type="text/x-markdown", metadata={})
        mock_extract_path.return_value = mock_result

        result = extractor.extract_bytes_sync(content)

        assert result == mock_result
        mock_mkstemp.assert_called_once_with(suffix=".markdown")
        mock_fdopen.assert_called_once_with(mock_fd, "wb")
        mock_file.write.assert_called_once_with(content)
        mock_extract_path.assert_called_once_with(Path(temp_path))
        mock_unlink.assert_called_once()


@pytest.mark.anyio
async def test_pandoc_core_extract_path_async_with_exception_group(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    test_path = Path("/test/file.md")

    with (
        patch.object(extractor, "_validate_pandoc_version", return_value=None),
        patch.object(extractor, "_get_pandoc_type_from_mime_type", return_value="markdown"),
        patch("kreuzberg._extractors._pandoc.run_taskgroup") as mock_taskgroup,
    ):
        mock_error = Exception("Test error")

        if ExceptionGroup is not None:
            mock_taskgroup.side_effect = ExceptionGroup("Multiple errors", [mock_error])
        else:
            pytest.skip("ExceptionGroup not available")

        with pytest.raises(ParsingError, match="Failed to process file"):
            await extractor.extract_path_async(test_path)


def test_pandoc_core_extract_path_sync_complete(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    test_path = Path("/test/file.md")

    with (
        patch.object(extractor, "_validate_pandoc_version_sync", return_value=None) as mock_validate,
        patch.object(extractor, "_get_pandoc_type_from_mime_type", return_value="markdown"),
        patch.object(extractor, "_extract_metadata_sync", return_value={"title": "Test"}) as mock_metadata,
        patch.object(extractor, "_extract_file_sync", return_value="# Test Content") as mock_content,
    ):
        result = extractor.extract_path_sync(test_path)

        assert isinstance(result, ExtractionResult)
        assert result.content == "# Test Content"
        assert result.metadata == {"title": "Test"}
        assert result.mime_type == "text/markdown"
        mock_validate.assert_called_once()
        mock_metadata.assert_called_once_with(test_path)
        mock_content.assert_called_once_with(test_path)


def test_pandoc_core_extract_path_sync_with_exception(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    test_path = Path("/test/file.md")

    with (
        patch.object(extractor, "_validate_pandoc_version_sync", return_value=None),
        patch.object(extractor, "_get_pandoc_type_from_mime_type", return_value="markdown"),
        patch.object(extractor, "_extract_metadata_sync", side_effect=Exception("Test error")),
    ):
        with pytest.raises(ParsingError, match="Failed to process file"):
            extractor.extract_path_sync(test_path)


@pytest.mark.anyio
async def test_pandoc_validation_extended_already_checked(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    extractor._checked_version = True

    with patch("kreuzberg._extractors._pandoc.run_process") as mock_run:
        await extractor._validate_pandoc_version()
        mock_run.assert_not_called()


@pytest.mark.anyio
async def test_pandoc_validation_extended_alternative_formats(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)

    version_strings = [
        b"pandoc version 3.1.2",
        b"pandoc (version 3.1.2)",
        b"pandoc-3.1.2",
        b"3.1.2\nSome other text",
        b"Some text 3.1.2 more text",
    ]

    for version_string in version_strings:
        extractor._checked_version = False
        mock_result = Mock()
        mock_result.stdout = version_string

        with patch("kreuzberg._extractors._pandoc.run_process", return_value=mock_result):
            await extractor._validate_pandoc_version()
            assert extractor._checked_version is True


@pytest.mark.anyio
async def test_pandoc_validation_extended_token_parsing(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    extractor._checked_version = False

    mock_result = Mock()
    mock_result.stdout = b"Some line\nAnother line with 3.1.2 token\nMore lines"

    with patch("kreuzberg._extractors._pandoc.run_process", return_value=mock_result):
        await extractor._validate_pandoc_version()
        assert extractor._checked_version is True


@pytest.mark.anyio
async def test_pandoc_validation_extended_old_version(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    extractor._checked_version = False

    mock_result = Mock()
    mock_result.stdout = b"pandoc 1.19.2"

    with patch("kreuzberg._extractors._pandoc.run_process", return_value=mock_result):
        with pytest.raises(MissingDependencyError, match="Pandoc version 2 or above"):
            await extractor._validate_pandoc_version()


@pytest.mark.anyio
async def test_pandoc_validation_extended_no_version_found(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    extractor._checked_version = False

    mock_result = Mock()
    mock_result.stdout = b"Some output without version numbers"

    with patch("kreuzberg._extractors._pandoc.run_process", return_value=mock_result):
        with pytest.raises(MissingDependencyError, match="Pandoc version 2 or above"):
            await extractor._validate_pandoc_version()


def test_pandoc_validation_extended_sync_success(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    extractor._checked_version = False

    mock_result = Mock()
    mock_result.returncode = 0
    mock_result.stdout = "pandoc 3.1.2\nCompiled with pandoc-types..."

    with patch("subprocess.run", return_value=mock_result):
        extractor._validate_pandoc_version_sync()
        assert extractor._checked_version is True


def test_pandoc_validation_extended_sync_failure(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    extractor._checked_version = False

    mock_result = Mock()
    mock_result.returncode = 1
    mock_result.stdout = ""

    with patch("subprocess.run", return_value=mock_result):
        with pytest.raises(MissingDependencyError, match="Pandoc version 2 or above"):
            extractor._validate_pandoc_version_sync()


def test_pandoc_validation_extended_sync_subprocess_error(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    extractor._checked_version = False

    with patch("subprocess.run", side_effect=subprocess.SubprocessError):
        with pytest.raises(MissingDependencyError, match="Pandoc version 2 or above"):
            extractor._validate_pandoc_version_sync()


def test_pandoc_metadata_extended_extract_metadata_empty(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    result = extractor._extract_metadata({})
    assert result == {}


def test_pandoc_metadata_extended_extract_metadata_with_citations(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    raw_meta = {
        "citations": [
            {"citationId": "cite1"},
            {"citationId": "cite2"},
            {"invalid": "entry"},
            "string_entry",
        ]
    }

    result = extractor._extract_metadata(raw_meta)
    assert result["citations"] == ["cite1", "cite2"]


def test_pandoc_metadata_extended_extract_metadata_with_standard_fields(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    raw_meta = {
        "title": {"t": "MetaString", "c": "Test Title"},
        "abstract": {"t": "MetaString", "c": "Test Abstract"},
        "date": {"t": "MetaString", "c": "2023-01-01"},
        "author": {"t": "MetaString", "c": "Test Author"},
        "institute": {"t": "MetaString", "c": "Test Organization"},
        "unknown_field": {"t": "MetaString", "c": "Should be ignored"},
    }

    result = extractor._extract_metadata(raw_meta)
    assert result["title"] == "Test Title"
    assert result["summary"] == "Test Abstract"
    assert result["created_at"] == "2023-01-01"
    assert result["authors"] == ["Test Author"]
    assert result["organization"] == "Test Organization"
    assert "unknown_field" not in result


def test_pandoc_metadata_extended_extract_metadata_with_valid_field(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    raw_meta = {"valid": {"t": "MetaString", "c": "true"}}

    result = extractor._extract_metadata(raw_meta)
    assert "valid" not in result


def test_pandoc_metadata_extended_extract_metadata_with_blocks_citations(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    raw_meta = {
        "blocks": [
            {"t": "Cite", "c": [[{"citationId": "block_cite1"}, {"citationId": "block_cite2"}], []]},
            {"t": "Para", "c": []},
        ]
    }

    result = extractor._extract_metadata(raw_meta)
    assert result["citations"] == ["block_cite1", "block_cite2"]


def test_pandoc_metadata_extended_extract_metadata_merge_citations(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    raw_meta = {
        "citations": [{"citationId": "cite1"}],
        "blocks": [{"t": "Cite", "c": [[{"citationId": "block_cite1"}], []]}],
    }

    result = extractor._extract_metadata(raw_meta)
    assert result["citations"] == ["cite1", "block_cite1"]


def test_pandoc_inline_text_extended_extract_inline_text_str(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    node = {"t": "Str", "c": "Hello"}

    result = extractor._extract_inline_text(node)
    assert result == "Hello"


def test_pandoc_inline_text_extended_extract_inline_text_space(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    node = {"t": "Space", "c": None}

    result = extractor._extract_inline_text(node)
    assert result == " "


def test_pandoc_inline_text_extended_extract_inline_text_emph(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    node = {"t": "Emph", "c": [{"t": "Str", "c": "emphasized"}]}

    result = extractor._extract_inline_text(node)
    assert result == "emphasized"


def test_pandoc_inline_text_extended_extract_inline_text_strong(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    node = {"t": "Strong", "c": [{"t": "Str", "c": "strong"}]}

    result = extractor._extract_inline_text(node)
    assert result == "strong"


def test_pandoc_inline_text_extended_extract_inline_text_unknown(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    node = {"t": "Unknown", "c": "content"}

    result = extractor._extract_inline_text(node)
    assert result is None


def test_pandoc_inline_text_extended_extract_inlines_multiple(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    nodes: list[dict[str, Any]] = [
        {"t": "Str", "c": "Hello"},
        {"t": "Space", "c": None},
        {"t": "Str", "c": "world"},
    ]

    result = extractor._extract_inlines(nodes)
    assert result == "Hello world"


def test_pandoc_inline_text_extended_extract_inlines_empty(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    nodes: list[dict[str, Any]] = []

    result = extractor._extract_inlines(nodes)
    assert result is None


def test_pandoc_meta_value_extended_extract_meta_value_meta_string(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    node = {"t": "MetaString", "c": "test value"}

    result = extractor._extract_meta_value(node)
    assert result == "test value"


def test_pandoc_meta_value_extended_extract_meta_value_meta_inlines(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    node = {
        "t": "MetaInlines",
        "c": [{"t": "Str", "c": "inline"}, {"t": "Space", "c": None}, {"t": "Str", "c": "text"}],
    }

    result = extractor._extract_meta_value(node)
    assert result == "inline text"


def test_pandoc_meta_value_extended_extract_meta_value_meta_list(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    node = {"t": "MetaList", "c": [{"t": "MetaString", "c": "item1"}, {"t": "MetaString", "c": "item2"}]}

    result = extractor._extract_meta_value(node)
    assert result == ["item1", "item2"]


def test_pandoc_meta_value_extended_extract_meta_value_meta_list_nested(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    node = {
        "t": "MetaList",
        "c": [{"t": "MetaList", "c": [{"t": "MetaString", "c": "nested1"}]}, {"t": "MetaString", "c": "item2"}],
    }

    result = extractor._extract_meta_value(node)
    assert result == ["nested1", "item2"]


def test_pandoc_meta_value_extended_extract_meta_value_meta_blocks(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    node = {
        "t": "MetaBlocks",
        "c": [
            {
                "t": "Para",
                "c": [{"t": "Str", "c": "First"}, {"t": "Space", "c": None}, {"t": "Str", "c": "paragraph"}],
            },
            {
                "t": "Para",
                "c": [{"t": "Str", "c": "Second"}, {"t": "Space", "c": None}, {"t": "Str", "c": "paragraph"}],
            },
        ],
    }

    result = extractor._extract_meta_value(node)
    assert result == "First paragraph Second paragraph"


def test_pandoc_meta_value_extended_extract_meta_value_invalid_node(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)

    assert extractor._extract_meta_value("string") is None

    assert extractor._extract_meta_value({"c": "content"}) is None

    assert extractor._extract_meta_value({"t": "MetaString"}) is None

    assert extractor._extract_meta_value({"t": "MetaString", "c": ""}) == ""


@pytest.mark.anyio
async def test_pandoc_file_extended_handle_extract_metadata_success(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    test_file = Path("/test/file.md")

    mock_json_data = {
        "pandoc-api-version": [1, 23],
        "meta": {"title": {"t": "MetaString", "c": "Test Title"}},
        "blocks": [],
    }

    with (
        patch.object(extractor, "_get_pandoc_type_from_mime_type", return_value="markdown"),
        patch("kreuzberg._utils._tmp.create_temp_file") as mock_temp_file,
        patch("kreuzberg._extractors._pandoc.run_process") as mock_run_process,
        patch("json.loads", return_value=mock_json_data),
        patch.object(extractor, "_extract_metadata", return_value={"title": "Test Title"}) as mock_extract,
    ):
        mock_unlink = AsyncMock()
        temp_path = "/tmp/metadata.json"
        mock_temp_file.return_value = (temp_path, mock_unlink)

        mock_result = Mock()
        mock_result.returncode = 0
        mock_run_process.return_value = mock_result

        mock_path = AsyncMock()
        mock_path.read_text.return_value = json.dumps(mock_json_data)

        with patch("kreuzberg._extractors._pandoc.AsyncPath", return_value=mock_path):
            result = await extractor._handle_extract_metadata(test_file)

            assert result == {"title": "Test Title"}
            mock_run_process.assert_called_once()
            mock_extract.assert_called_once_with(mock_json_data)
            mock_unlink.assert_called_once()


@pytest.mark.anyio
async def test_pandoc_file_extended_handle_extract_metadata_pandoc_error(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    test_file = Path("/test/file.md")

    with (
        patch.object(extractor, "_get_pandoc_type_from_mime_type", return_value="markdown"),
        patch("kreuzberg._utils._tmp.create_temp_file") as mock_temp_file,
        patch("kreuzberg._extractors._pandoc.run_process") as mock_run_process,
    ):
        mock_unlink = AsyncMock()
        temp_path = "/tmp/metadata.json"
        mock_temp_file.return_value = (temp_path, mock_unlink)

        mock_result = Mock()
        mock_result.returncode = 1
        mock_result.stderr = b"Error message"
        mock_run_process.return_value = mock_result

        with pytest.raises(ParsingError, match="Failed to extract file data"):
            await extractor._handle_extract_metadata(test_file)

        mock_unlink.assert_called_once()


@pytest.mark.anyio
async def test_pandoc_file_extended_handle_extract_file_success(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    test_file = Path("/test/file.md")

    with (
        patch.object(extractor, "_get_pandoc_type_from_mime_type", return_value="markdown"),
        patch("kreuzberg._utils._tmp.create_temp_file") as mock_temp_file,
        patch("kreuzberg._extractors._pandoc.run_process") as mock_run_process,
    ):
        mock_unlink = AsyncMock()
        temp_path = "/tmp/output.md"
        mock_temp_file.return_value = (temp_path, mock_unlink)

        mock_result = Mock()
        mock_result.returncode = 0
        mock_run_process.return_value = mock_result

        mock_path = AsyncMock()
        mock_path.read_text.return_value = "# Test Content\n\nThis is test content."

        with patch("kreuzberg._extractors._pandoc.AsyncPath", return_value=mock_path):
            result = await extractor._handle_extract_file(test_file)

            assert "Test Content" in result
            mock_run_process.assert_called_once()
            mock_unlink.assert_called_once()


def test_pandoc_file_extended_extract_metadata_sync_success(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    test_path = Path("/test/file.md")

    mock_json_data = {
        "pandoc-api-version": [1, 23],
        "meta": {"title": {"t": "MetaString", "c": "Test Title"}},
        "blocks": [],
    }

    with (
        patch.object(extractor, "_get_pandoc_type_from_mime_type", return_value="markdown"),
        patch("tempfile.mkstemp") as mock_mkstemp,
        patch("os.close") as mock_close,
        patch("subprocess.run") as mock_run,
        patch("json.loads", return_value=mock_json_data),
        patch.object(extractor, "_extract_metadata", return_value={"title": "Test Title"}) as mock_extract,
        patch("pathlib.Path.open") as mock_open,
        patch("pathlib.Path.unlink") as mock_unlink,
    ):
        mock_fd = 3
        temp_path = "/tmp/metadata.json"
        mock_mkstemp.return_value = (mock_fd, temp_path)

        mock_result = Mock()
        mock_result.returncode = 0
        mock_run.return_value = mock_result

        mock_file = Mock()
        mock_file.read.return_value = json.dumps(mock_json_data)
        mock_open.return_value.__enter__.return_value = mock_file

        result = extractor._extract_metadata_sync(test_path)

        assert result == {"title": "Test Title"}
        mock_close.assert_called_once_with(mock_fd)
        mock_run.assert_called_once()
        mock_extract.assert_called_once_with(mock_json_data)
        mock_unlink.assert_called_once()


def test_pandoc_file_extended_extract_file_sync_success(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    test_path = Path("/test/file.md")

    with (
        patch.object(extractor, "_get_pandoc_type_from_mime_type", return_value="markdown"),
        patch("tempfile.mkstemp") as mock_mkstemp,
        patch("os.close") as mock_close,
        patch("subprocess.run") as mock_run,
        patch("pathlib.Path.open") as mock_open,
        patch("pathlib.Path.unlink") as mock_unlink,
    ):
        mock_fd = 3
        temp_path = "/tmp/output.md"
        mock_mkstemp.return_value = (mock_fd, temp_path)

        mock_result = Mock()
        mock_result.returncode = 0
        mock_run.return_value = mock_result

        mock_file = Mock()
        mock_file.read.return_value = "# Test Content\n\nThis is test content."
        mock_open.return_value.__enter__.return_value = mock_file

        result = extractor._extract_file_sync(test_path)

        assert "Test Content" in result
        mock_close.assert_called_once_with(mock_fd)
        mock_run.assert_called_once()
        mock_unlink.assert_called_once()


def test_pandoc_subclasses_extended_markdown_extractor_mime_types() -> None:
    expected_types = {
        "text/x-markdown",
        "text/x-commonmark",
        "text/x-gfm",
        "text/x-markdown-extra",
        "text/x-multimarkdown",
        "text/x-mdoc",
    }
    assert expected_types == MarkdownExtractor.SUPPORTED_MIME_TYPES


def test_pandoc_subclasses_extended_office_document_extractor_mime_types() -> None:
    expected_types = {
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "application/vnd.oasis.opendocument.text",
    }
    assert expected_types == OfficeDocumentExtractor.SUPPORTED_MIME_TYPES


def test_pandoc_subclasses_extended_ebook_extractor_mime_types() -> None:
    expected_types = {
        "application/epub+zip",
        "application/x-fictionbook+xml",
    }
    assert expected_types == EbookExtractor.SUPPORTED_MIME_TYPES


def test_pandoc_subclasses_extended_structured_text_extractor_mime_types() -> None:
    expected_types = {
        "text/x-rst",
        "text/x-org",
        "text/x-dokuwiki",
        "text/x-pod",
    }
    assert expected_types == StructuredTextExtractor.SUPPORTED_MIME_TYPES


def test_pandoc_subclasses_extended_latex_extractor_mime_types() -> None:
    expected_types = {
        "application/x-latex",
        "application/x-typst",
    }
    assert expected_types == LaTeXExtractor.SUPPORTED_MIME_TYPES


def test_pandoc_subclasses_extended_bibliography_extractor_mime_types() -> None:
    expected_types = {
        "application/x-bibtex",
        "application/x-biblatex",
        "application/csl+json",
        "application/x-research-info-systems",
        "application/x-endnote+xml",
    }
    assert expected_types == BibliographyExtractor.SUPPORTED_MIME_TYPES


def test_pandoc_subclasses_extended_xml_based_extractor_mime_types() -> None:
    expected_types = {
        "application/docbook+xml",
        "application/x-jats+xml",
        "application/x-opml+xml",
    }
    assert expected_types == XMLBasedExtractor.SUPPORTED_MIME_TYPES


def test_pandoc_subclasses_extended_tabular_data_extractor_mime_types() -> None:
    expected_types = {
        "text/csv",
        "text/tab-separated-values",
    }
    assert expected_types == TabularDataExtractor.SUPPORTED_MIME_TYPES


def test_pandoc_subclasses_extended_misc_format_extractor_mime_types() -> None:
    expected_types = {
        "application/rtf",
        "text/troff",
        "application/x-ipynb+json",
    }
    assert expected_types == MiscFormatExtractor.SUPPORTED_MIME_TYPES


def test_pandoc_subclasses_extended_subclass_inheritance(test_config: ExtractionConfig) -> None:
    subclasses = [
        MarkdownExtractor,
        OfficeDocumentExtractor,
        EbookExtractor,
        StructuredTextExtractor,
        LaTeXExtractor,
        BibliographyExtractor,
        XMLBasedExtractor,
        TabularDataExtractor,
        MiscFormatExtractor,
    ]

    for subclass in subclasses:
        assert issubclass(subclass, PandocExtractor)
        instance = subclass("text/x-markdown", test_config)
        assert isinstance(instance, PandocExtractor)


def test_pandoc_base_supported_mime_types_mapping(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)

    assert (
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        in extractor.MIMETYPE_TO_PANDOC_TYPE_MAPPING
    )
    assert "text/x-markdown" in extractor.MIMETYPE_TO_PANDOC_TYPE_MAPPING
    assert "application/epub+zip" in extractor.MIMETYPE_TO_PANDOC_TYPE_MAPPING

    assert (
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        in extractor.MIMETYPE_TO_FILE_EXTENSION_MAPPING
    )
    assert (
        extractor.MIMETYPE_TO_FILE_EXTENSION_MAPPING[
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        ]
        == "docx"
    )


def test_pandoc_base_get_pandoc_key_mappings() -> None:
    assert PandocExtractor._get_pandoc_key("abstract") == "summary"
    assert PandocExtractor._get_pandoc_key("date") == "created_at"
    assert PandocExtractor._get_pandoc_key("author") == "authors"
    assert PandocExtractor._get_pandoc_key("contributors") == "authors"
    assert PandocExtractor._get_pandoc_key("institute") == "organization"
    assert PandocExtractor._get_pandoc_key("title") == "title"
    assert PandocExtractor._get_pandoc_key("unknown_key") is None


def test_pandoc_base_extract_path_sync_success(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    test_path = Path("/test/file.md")

    with (
        patch.object(extractor, "_validate_pandoc_version_sync", return_value=None) as mock_validate,
        patch.object(extractor, "_get_pandoc_type_from_mime_type", return_value="markdown"),
        patch.object(extractor, "_extract_metadata_sync", return_value={"title": "Test"}) as mock_metadata,
        patch.object(extractor, "_extract_file_sync", return_value="# Test Content") as mock_content,
    ):
        result = extractor.extract_path_sync(test_path)

        assert isinstance(result, ExtractionResult)
        assert result.content == "# Test Content"
        assert result.metadata == {"title": "Test"}
        assert result.mime_type == "text/markdown"
        mock_validate.assert_called_once()
        mock_metadata.assert_called_once_with(test_path)
        mock_content.assert_called_once_with(test_path)


def test_pandoc_base_extract_path_sync_failure(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    test_path = Path("/test/file.md")

    with (
        patch.object(extractor, "_validate_pandoc_version_sync", return_value=None),
        patch.object(extractor, "_get_pandoc_type_from_mime_type", return_value="markdown"),
        patch.object(extractor, "_extract_metadata_sync", side_effect=Exception("Test error")),
    ):
        with pytest.raises(ParsingError, match="Failed to process file"):
            extractor.extract_path_sync(test_path)


@pytest.mark.anyio
async def test_pandoc_base_extract_path_async_failure_with_exception_group(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    test_path = Path("/test/file.md")

    with (
        patch.object(extractor, "_validate_pandoc_version", return_value=None),
        patch.object(extractor, "_get_pandoc_type_from_mime_type", return_value="markdown"),
        patch("kreuzberg._extractors._pandoc.run_taskgroup") as mock_taskgroup,
    ):
        mock_error = Exception("Test error")

        if ExceptionGroup is not None:
            mock_taskgroup.side_effect = ExceptionGroup("Multiple errors", [mock_error])
        else:
            pytest.skip("ExceptionGroup not available")

        with pytest.raises(ParsingError, match="Failed to process file"):
            await extractor.extract_path_async(test_path)


def test_pandoc_metadata_extraction_with_citations(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    raw_meta = {
        "citations": [
            {"citationId": "cite1"},
            {"citationId": "cite2"},
            {"invalid": "entry"},
            "string_entry",
        ]
    }

    result = extractor._extract_metadata(raw_meta)
    assert result["citations"] == ["cite1", "cite2"]


def test_pandoc_metadata_extraction_with_standard_fields(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    raw_meta = {
        "title": {"t": "MetaString", "c": "Test Title"},
        "abstract": {"t": "MetaString", "c": "Test Abstract"},
        "date": {"t": "MetaString", "c": "2023-01-01"},
        "author": {"t": "MetaString", "c": "Test Author"},
        "institute": {"t": "MetaString", "c": "Test Organization"},
        "unknown_field": {"t": "MetaString", "c": "Should be ignored"},
    }

    result = extractor._extract_metadata(raw_meta)
    assert result["title"] == "Test Title"
    assert result["summary"] == "Test Abstract"
    assert result["created_at"] == "2023-01-01"
    assert result["authors"] == ["Test Author"]
    assert result["organization"] == "Test Organization"
    assert "unknown_field" not in result


def test_pandoc_metadata_extraction_with_blocks_citations(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    raw_meta = {
        "blocks": [
            {"t": "Cite", "c": [[{"citationId": "block_cite1"}, {"citationId": "block_cite2"}], []]},
            {"t": "Para", "c": []},
        ]
    }

    result = extractor._extract_metadata(raw_meta)
    assert result["citations"] == ["block_cite1", "block_cite2"]


def test_pandoc_metadata_extraction_merge_citations(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    raw_meta = {
        "citations": [{"citationId": "cite1"}],
        "blocks": [{"t": "Cite", "c": [[{"citationId": "block_cite1"}], []]}],
    }

    result = extractor._extract_metadata(raw_meta)
    assert result["citations"] == ["cite1", "block_cite1"]


def test_pandoc_subclasses_markdown_extractor_mime_types() -> None:
    expected_types = {
        "text/x-markdown",
        "text/x-commonmark",
        "text/x-gfm",
        "text/x-markdown-extra",
        "text/x-multimarkdown",
        "text/x-mdoc",
    }
    assert expected_types == MarkdownExtractor.SUPPORTED_MIME_TYPES


def test_pandoc_subclasses_office_document_extractor_mime_types() -> None:
    expected_types = {
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "application/vnd.oasis.opendocument.text",
    }
    assert expected_types == OfficeDocumentExtractor.SUPPORTED_MIME_TYPES


def test_pandoc_subclasses_ebook_extractor_mime_types() -> None:
    expected_types = {
        "application/epub+zip",
        "application/x-fictionbook+xml",
    }
    assert expected_types == EbookExtractor.SUPPORTED_MIME_TYPES


def test_pandoc_subclasses_structured_text_extractor_mime_types() -> None:
    expected_types = {
        "text/x-rst",
        "text/x-org",
        "text/x-dokuwiki",
        "text/x-pod",
    }
    assert expected_types == StructuredTextExtractor.SUPPORTED_MIME_TYPES


def test_pandoc_subclasses_latex_extractor_mime_types() -> None:
    expected_types = {
        "application/x-latex",
        "application/x-typst",
    }
    assert expected_types == LaTeXExtractor.SUPPORTED_MIME_TYPES


def test_pandoc_subclasses_bibliography_extractor_mime_types() -> None:
    expected_types = {
        "application/x-bibtex",
        "application/x-biblatex",
        "application/csl+json",
        "application/x-research-info-systems",
        "application/x-endnote+xml",
    }
    assert expected_types == BibliographyExtractor.SUPPORTED_MIME_TYPES


def test_pandoc_subclasses_xml_based_extractor_mime_types() -> None:
    expected_types = {
        "application/docbook+xml",
        "application/x-jats+xml",
        "application/x-opml+xml",
    }
    assert expected_types == XMLBasedExtractor.SUPPORTED_MIME_TYPES


def test_pandoc_subclasses_tabular_data_extractor_mime_types() -> None:
    expected_types = {
        "text/csv",
        "text/tab-separated-values",
    }
    assert expected_types == TabularDataExtractor.SUPPORTED_MIME_TYPES


def test_pandoc_subclasses_misc_format_extractor_mime_types() -> None:
    expected_types = {
        "application/rtf",
        "text/troff",
        "application/x-ipynb+json",
    }
    assert expected_types == MiscFormatExtractor.SUPPORTED_MIME_TYPES


def test_pandoc_subclasses_subclass_inheritance(test_config: ExtractionConfig) -> None:
    subclasses = [
        MarkdownExtractor,
        OfficeDocumentExtractor,
        EbookExtractor,
        StructuredTextExtractor,
        LaTeXExtractor,
        BibliographyExtractor,
        XMLBasedExtractor,
        TabularDataExtractor,
        MiscFormatExtractor,
    ]

    for subclass in subclasses:
        assert issubclass(subclass, PandocExtractor)
        instance = subclass("text/x-markdown", test_config)
        assert isinstance(instance, PandocExtractor)


def test_pandoc_sync_methods_extract_metadata_sync_json_decode_error(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    test_path = Path("/test/file.md")

    with (
        patch.object(extractor, "_get_pandoc_type_from_mime_type", return_value="markdown"),
        patch("tempfile.mkstemp") as mock_mkstemp,
        patch("os.close") as mock_close,
        patch("subprocess.run") as mock_run,
        patch("pathlib.Path.open") as mock_open,
        patch("pathlib.Path.unlink") as mock_unlink,
    ):
        mock_fd = 3
        temp_path = "/tmp/metadata.json"
        mock_mkstemp.return_value = (mock_fd, temp_path)

        mock_result = Mock()
        mock_result.returncode = 0
        mock_run.return_value = mock_result

        mock_file = Mock()
        mock_file.read.return_value = "invalid json"
        mock_open.return_value.__enter__.return_value = mock_file

        with pytest.raises(json.JSONDecodeError):
            extractor._extract_metadata_sync(test_path)

        mock_close.assert_called_once_with(mock_fd)
        mock_unlink.assert_called_once()


def test_pandoc_sync_methods_extract_metadata_sync_os_error(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    test_path = Path("/test/file.md")

    with (
        patch.object(extractor, "_get_pandoc_type_from_mime_type", return_value="markdown"),
        patch("tempfile.mkstemp") as mock_mkstemp,
        patch("os.close") as mock_close,
        patch("subprocess.run", side_effect=OSError("Subprocess error")),
        patch("pathlib.Path.unlink") as mock_unlink,
    ):
        mock_fd = 3
        temp_path = "/tmp/metadata.json"
        mock_mkstemp.return_value = (mock_fd, temp_path)

        with pytest.raises(OSError, match="Subprocess error"):
            extractor._extract_metadata_sync(test_path)

        mock_close.assert_called_once_with(mock_fd)
        mock_unlink.assert_called_once()


def test_pandoc_sync_methods_extract_file_sync_os_error(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    test_path = Path("/test/file.md")

    with (
        patch.object(extractor, "_get_pandoc_type_from_mime_type", return_value="markdown"),
        patch("tempfile.mkstemp") as mock_mkstemp,
        patch("os.close") as mock_close,
        patch("subprocess.run", side_effect=OSError("File error")),
        patch("pathlib.Path.unlink") as mock_unlink,
    ):
        mock_fd = 3
        temp_path = "/tmp/output.md"
        mock_mkstemp.return_value = (mock_fd, temp_path)

        with pytest.raises(OSError, match="File error"):
            extractor._extract_file_sync(test_path)

        mock_close.assert_called_once_with(mock_fd)
        mock_unlink.assert_called_once()


def test_pandoc_sync_methods_extract_file_sync_subprocess_error(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    test_path = Path("/test/file.md")

    with (
        patch.object(extractor, "_get_pandoc_type_from_mime_type", return_value="markdown"),
        patch("tempfile.mkstemp") as mock_mkstemp,
        patch("os.close") as mock_close,
        patch("subprocess.run") as mock_run,
        patch("pathlib.Path.unlink") as mock_unlink,
    ):
        mock_fd = 3
        temp_path = "/tmp/output.md"
        mock_mkstemp.return_value = (mock_fd, temp_path)

        mock_result = Mock()
        mock_result.returncode = 1
        mock_result.stderr = "Pandoc error"
        mock_run.return_value = mock_result

        with pytest.raises(ParsingError, match="Failed to extract file data"):
            extractor._extract_file_sync(test_path)

        mock_close.assert_called_once_with(mock_fd)
        mock_unlink.assert_called_once()


@pytest.mark.anyio
async def test_pandoc_async_errors_handle_extract_metadata_json_decode_error(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    test_file = Path("/test/file.md")

    with (
        patch.object(extractor, "_get_pandoc_type_from_mime_type", return_value="markdown"),
        patch("kreuzberg._utils._tmp.create_temp_file") as mock_temp_file,
        patch("kreuzberg._extractors._pandoc.run_process") as mock_run_process,
    ):
        mock_unlink = AsyncMock()
        temp_path = "/tmp/metadata.json"
        mock_temp_file.return_value = (temp_path, mock_unlink)

        mock_result = Mock()
        mock_result.returncode = 0
        mock_run_process.return_value = mock_result

        mock_path = AsyncMock()
        mock_path.read_text.return_value = "invalid json"

        with patch("kreuzberg._extractors._pandoc.AsyncPath", return_value=mock_path):
            with pytest.raises(json.JSONDecodeError):
                await extractor._handle_extract_metadata(test_file)

        mock_unlink.assert_called_once()


@pytest.mark.anyio
async def test_pandoc_async_errors_handle_extract_metadata_os_error(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    test_file = Path("/test/file.md")

    with (
        patch.object(extractor, "_get_pandoc_type_from_mime_type", return_value="markdown"),
        patch("kreuzberg._utils._tmp.create_temp_file") as mock_temp_file,
        patch("kreuzberg._extractors._pandoc.run_process", side_effect=OSError("Async OS error")),
    ):
        mock_unlink = AsyncMock()
        temp_path = "/tmp/metadata.json"
        mock_temp_file.return_value = (temp_path, mock_unlink)

        with pytest.raises(OSError, match="Async OS error"):
            await extractor._handle_extract_metadata(test_file)

        mock_unlink.assert_called_once()


@pytest.mark.anyio
async def test_pandoc_async_errors_handle_extract_file_os_error(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    test_file = Path("/test/file.md")

    with (
        patch.object(extractor, "_get_pandoc_type_from_mime_type", return_value="markdown"),
        patch("kreuzberg._utils._tmp.create_temp_file") as mock_temp_file,
        patch("kreuzberg._extractors._pandoc.run_process", side_effect=OSError("File OS error")),
    ):
        mock_unlink = AsyncMock()
        temp_path = "/tmp/output.md"
        mock_temp_file.return_value = (temp_path, mock_unlink)

        with pytest.raises(OSError, match="File OS error"):
            await extractor._handle_extract_file(test_file)

        mock_unlink.assert_called_once()


def test_pandoc_metadata_edge_cases_extract_meta_value_meta_blocks_no_para(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    node = {
        "t": "MetaBlocks",
        "c": [
            {"t": "Header", "c": [1, [], []]},
            {"t": "CodeBlock", "c": ["", "code"]},
        ],
    }

    result = extractor._extract_meta_value(node)
    assert result is None


def test_pandoc_metadata_edge_cases_extract_meta_value_meta_blocks_empty_para(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    node = {
        "t": "MetaBlocks",
        "c": [
            {"t": "Para", "c": []},
            {"t": "Para", "c": None},
        ],
    }

    result = extractor._extract_meta_value(node)
    assert result is None


def test_pandoc_metadata_edge_cases_extract_meta_value_empty_content(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)

    node = {"t": "MetaList", "c": []}
    result = extractor._extract_meta_value(node)
    assert result is None

    node_none: dict[str, Any] = {"t": "MetaString", "c": None}
    result = extractor._extract_meta_value(node_none)
    assert result is None


def test_pandoc_metadata_edge_cases_extract_meta_value_non_dict_content(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    node = {
        "t": "MetaList",
        "c": [
            "string_item",
            {"t": "MetaString", "c": "valid_item"},
            123,
        ],
    }

    result = extractor._extract_meta_value(node)
    assert result == ["valid_item"]


def test_pandoc_metadata_edge_cases_extract_inline_text_empty_content(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)

    node = {"t": "Emph", "c": []}
    result = extractor._extract_inline_text(node)
    assert result is None

    node_strong: dict[str, Any] = {"t": "Strong"}
    result = extractor._extract_inline_text(node_strong)
    assert result is None


def test_pandoc_metadata_edge_cases_extract_inlines_with_empty_results(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    nodes: list[dict[str, Any]] = [
        {"t": "Unknown", "c": "content"},
        {"t": "Str", "c": ""},
        {"t": "Space"},
    ]

    result = extractor._extract_inlines(nodes)
    assert result is None


def test_pandoc_metadata_edge_cases_extract_metadata_contributors_field(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    raw_meta = {
        "contributors": {"t": "MetaString", "c": "Contributor Name"},
    }

    result = extractor._extract_metadata(raw_meta)
    assert result["authors"] == ["Contributor Name"]


def test_pandoc_metadata_edge_cases_extract_metadata_languages_field(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    raw_meta = {
        "languages": {"t": "MetaString", "c": "en"},
    }

    result = extractor._extract_metadata(raw_meta)
    assert result["languages"] == ["en"]


def test_pandoc_metadata_edge_cases_extract_metadata_invalid_citations_structure(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    raw_meta = {
        "citations": "not_a_list",
    }

    result = extractor._extract_metadata(raw_meta)
    assert "citations" not in result


def test_pandoc_metadata_edge_cases_extract_metadata_blocks_invalid_citation_structure(
    test_config: ExtractionConfig,
) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    raw_meta = {
        "blocks": [
            {"t": "Para", "c": []},
            {"t": "Cite", "c": [["not_a_dict"], []]},
        ]
    }

    result = extractor._extract_metadata(raw_meta)
    assert "citations" not in result


def test_pandoc_version_validation_edge_cases_validate_pandoc_version_sync_file_not_found(
    test_config: ExtractionConfig,
) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    extractor._checked_version = False

    with patch("subprocess.run", side_effect=FileNotFoundError):
        with pytest.raises(MissingDependencyError, match="Pandoc version 2 or above"):
            extractor._validate_pandoc_version_sync()


def test_pandoc_version_validation_edge_cases_validate_pandoc_version_sync_already_checked(
    test_config: ExtractionConfig,
) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    extractor._checked_version = True

    with patch("subprocess.run") as mock_run:
        extractor._validate_pandoc_version_sync()
        mock_run.assert_not_called()


def test_pandoc_version_validation_edge_cases_validate_pandoc_version_sync_complex_patterns(
    test_config: ExtractionConfig,
) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)

    version_strings = [
        "pandoc version 3.1.2",
        "pandoc (version 3.1.2)",
        "pandoc-3.1.2",
        "3.1.2\nSome other text",
    ]

    for version_string in version_strings:
        extractor._checked_version = False
        mock_result = Mock()
        mock_result.returncode = 0
        mock_result.stdout = version_string

        with patch("subprocess.run", return_value=mock_result):
            extractor._validate_pandoc_version_sync()
            assert extractor._checked_version is True


def test_pandoc_version_validation_edge_cases_validate_pandoc_version_sync_no_match_found(
    test_config: ExtractionConfig,
) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    extractor._checked_version = False

    mock_result = Mock()
    mock_result.returncode = 0
    mock_result.stdout = "Some output without recognizable version"

    with patch("subprocess.run", return_value=mock_result):
        with pytest.raises(MissingDependencyError, match="Pandoc version 2 or above"):
            extractor._validate_pandoc_version_sync()


def test_pandoc_mime_type_handling_get_pandoc_type_fallback_to_markdown(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/markdown", test_config)
    result = extractor._get_pandoc_type_from_mime_type("text/markdown")
    assert result == "markdown"


def test_pandoc_mime_type_handling_get_pandoc_type_prefix_matching(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)

    result = extractor._get_pandoc_type_from_mime_type("text/x-markdown; charset=utf-8")
    assert result == "markdown"


def test_pandoc_mime_type_handling_get_pandoc_key_all_mappings() -> None:
    assert PandocExtractor._get_pandoc_key("abstract") == "summary"
    assert PandocExtractor._get_pandoc_key("date") == "created_at"
    assert PandocExtractor._get_pandoc_key("author") == "authors"
    assert PandocExtractor._get_pandoc_key("contributors") == "authors"
    assert PandocExtractor._get_pandoc_key("institute") == "organization"

    assert PandocExtractor._get_pandoc_key("title") == "title"
    assert PandocExtractor._get_pandoc_key("subject") == "subject"
    assert PandocExtractor._get_pandoc_key("keywords") == "keywords"

    assert PandocExtractor._get_pandoc_key("unknown_field") is None
    assert PandocExtractor._get_pandoc_key("random_key") is None


def test_pandoc_complex_metadata_extract_metadata_complex_nested_structure(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    raw_meta = {
        "title": {
            "t": "MetaInlines",
            "c": [
                {"t": "Str", "c": "Complex"},
                {"t": "Space", "c": None},
                {"t": "Emph", "c": [{"t": "Str", "c": "Title"}]},
            ],
        },
        "authors": {
            "t": "MetaList",
            "c": [
                {
                    "t": "MetaInlines",
                    "c": [
                        {"t": "Str", "c": "Author"},
                        {"t": "Space", "c": None},
                        {"t": "Str", "c": "One"},
                    ],
                },
                {
                    "t": "MetaInlines",
                    "c": [{"t": "Str", "c": "Author"}, {"t": "Space", "c": None}, {"t": "Str", "c": "Two"}],
                },
            ],
        },
        "abstract": {
            "t": "MetaBlocks",
            "c": [
                {
                    "t": "Para",
                    "c": [
                        {"t": "Str", "c": "This"},
                        {"t": "Space", "c": None},
                        {"t": "Str", "c": "is"},
                        {"t": "Space", "c": None},
                        {"t": "Str", "c": "abstract."},
                    ],
                },
                {
                    "t": "Para",
                    "c": [
                        {"t": "Str", "c": "Second"},
                        {"t": "Space", "c": None},
                        {"t": "Str", "c": "paragraph."},
                    ],
                },
            ],
        },
    }

    result = extractor._extract_metadata(raw_meta)
    assert result["title"] == "Complex Title"
    assert result["authors"] == ["Author One", "Author Two"]
    assert result["summary"] == "This is abstract. Second paragraph."


def test_pandoc_constants_and_types_block_constants() -> None:
    from kreuzberg._extractors._pandoc import (
        BLOCK_CODE,
        BLOCK_HEADER,
        BLOCK_LIST,
        BLOCK_ORDERED,
        BLOCK_PARA,
        BLOCK_QUOTE,
    )

    assert BLOCK_HEADER == "Header"
    assert BLOCK_PARA == "Para"
    assert BLOCK_CODE == "CodeBlock"
    assert BLOCK_QUOTE == "BlockQuote"
    assert BLOCK_LIST == "BulletList"
    assert BLOCK_ORDERED == "OrderedList"


def test_pandoc_constants_and_types_inline_constants() -> None:
    from kreuzberg._extractors._pandoc import (
        INLINE_CODE,
        INLINE_EMPH,
        INLINE_IMAGE,
        INLINE_LINK,
        INLINE_MATH,
        INLINE_SPACE,
        INLINE_STR,
        INLINE_STRONG,
    )

    assert INLINE_STR == "Str"
    assert INLINE_SPACE == "Space"
    assert INLINE_EMPH == "Emph"
    assert INLINE_STRONG == "Strong"
    assert INLINE_LINK == "Link"
    assert INLINE_IMAGE == "Image"
    assert INLINE_CODE == "Code"
    assert INLINE_MATH == "Math"


def test_pandoc_constants_and_types_meta_constants() -> None:
    from kreuzberg._extractors._pandoc import (
        META_BLOCKS,
        META_INLINES,
        META_LIST,
        META_MAP,
        META_STRING,
    )

    assert META_MAP == "MetaMap"
    assert META_LIST == "MetaList"
    assert META_INLINES == "MetaInlines"
    assert META_STRING == "MetaString"
    assert META_BLOCKS == "MetaBlocks"


def test_pandoc_constants_and_types_field_constants() -> None:
    from kreuzberg._extractors._pandoc import CONTENT_FIELD, TYPE_FIELD

    assert CONTENT_FIELD == "c"
    assert TYPE_FIELD == "t"


def test_pandoc_constants_and_types_mime_type_mappings_complete(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)

    pandoc_types = extractor.MIMETYPE_TO_PANDOC_TYPE_MAPPING
    extensions = extractor.MIMETYPE_TO_FILE_EXTENSION_MAPPING

    for mime_type in pandoc_types:
        assert mime_type in extensions, f"Missing file extension for MIME type: {mime_type}"


def test_pandoc_constants_and_types_file_cleanup_on_exception(test_config: ExtractionConfig) -> None:
    extractor = PandocExtractor("text/x-markdown", test_config)
    content = b"# Test Content"

    with (
        patch.object(extractor, "_get_pandoc_type_from_mime_type", return_value="markdown"),
        patch("tempfile.mkstemp") as mock_mkstemp,
        patch("os.fdopen") as mock_fdopen,
        patch.object(extractor, "extract_path_sync", side_effect=Exception("Test error")),
        patch("pathlib.Path.unlink") as mock_unlink,
    ):
        mock_fd = 3
        temp_path = "/tmp/test.md"
        mock_mkstemp.return_value = (mock_fd, temp_path)

        mock_file = Mock()
        mock_fdopen.return_value.__enter__.return_value = mock_file

        with pytest.raises(Exception, match="Test error"):
            extractor.extract_bytes_sync(content)

        mock_unlink.assert_called_once()
