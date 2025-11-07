from __future__ import annotations

import json
from pathlib import Path
from unittest.mock import Mock, patch

import click
import pytest
from click.testing import CliRunner

from kreuzberg import ExtractionResult
from kreuzberg.cli import (
    OcrBackendParamType,
    _build_cli_args,
    _load_config,
    _perform_extraction,
    _write_output,
    cli,
    config,
    extract,
    format_extraction_result,
    handle_error,
)
from kreuzberg.exceptions import KreuzbergError, MissingDependencyError


def test_ocr_backend_param_type_none() -> None:
    param_type = OcrBackendParamType()
    result = param_type.convert(None, None, None)
    assert result is None


def test_ocr_backend_param_type_none_string() -> None:
    param_type = OcrBackendParamType()
    result = param_type.convert("none", None, None)
    assert result == "none"


def test_ocr_backend_param_type_valid() -> None:
    param_type = OcrBackendParamType()
    result = param_type.convert("tesseract", None, None)
    assert result == "tesseract"

    result = param_type.convert("EasyOCR", None, None)
    assert result == "easyocr"

    result = param_type.convert("PaddleOCR", None, None)
    assert result == "paddleocr"


def test_ocr_backend_param_type_invalid() -> None:
    param_type = OcrBackendParamType()
    mock_param = Mock(spec=click.Parameter)
    mock_ctx = Mock(spec=click.Context)
    mock_ctx.command = Mock()

    with pytest.raises(click.exceptions.BadParameter):
        param_type.convert("invalid", mock_param, mock_ctx)


def test_format_extraction_result_text() -> None:
    result = ExtractionResult(
        content="Test content",
        mime_type="text/plain",
        metadata={"title": "Test Document"},
        tables=[{"page_number": 1, "text": "table text", "cropped_image": None, "df": None}],  # type: ignore[typeddict-item]
        chunks=["chunk1", "chunk2"],
    )

    output = format_extraction_result(result, show_metadata=False, output_format="text")
    assert "Test content" in output
    assert "METADATA" not in output
    assert "TABLES" in output

    output = format_extraction_result(result, show_metadata=True, output_format="text")
    assert "Test content" in output
    assert "METADATA" in output
    assert "TABLES" in output
    assert '"title": "Test Document"' in output


def test_format_extraction_result_json() -> None:
    result = ExtractionResult(
        content="Test content",
        mime_type="text/plain",
        metadata={"title": "Test Document"},
        tables=[{"page_number": 1, "text": "table text", "cropped_image": None, "df": None}],  # type: ignore[typeddict-item]
        chunks=["chunk1", "chunk2"],
    )

    output = format_extraction_result(result, show_metadata=False, output_format="json")
    data = json.loads(output)
    assert data["content"] == "Test content"
    assert data["mime_type"] == "text/plain"
    assert "metadata" not in data
    assert data["tables"][0]["page_number"] == 1
    assert data["chunks"] == ["chunk1", "chunk2"]

    output = format_extraction_result(result, show_metadata=True, output_format="json")
    data = json.loads(output)
    assert data["metadata"]["title"] == "Test Document"


def test_format_extraction_result_json_with_dataframe() -> None:
    mock_df = Mock()
    mock_df.write_csv.return_value = "col1,col2\nval1,val2"

    result = ExtractionResult(
        content="Test",
        mime_type="text/plain",
        tables=[{"page_number": 1, "text": "table", "cropped_image": None, "df": mock_df}],  # type: ignore[typeddict-item]
    )

    output = format_extraction_result(result, show_metadata=False, output_format="json")
    data = json.loads(output)
    assert data["tables"][0]["data_csv"] == "col1,col2\nval1,val2"


def test_format_extraction_result_json_with_pandas_dataframe() -> None:
    mock_df = Mock()
    del mock_df.write_csv
    mock_df.to_csv.return_value = "col1,col2\nval1,val2"

    result = ExtractionResult(
        content="Test",
        mime_type="text/plain",
        tables=[{"page_number": 1, "text": "table", "cropped_image": None, "df": mock_df}],  # type: ignore[typeddict-item]
    )

    output = format_extraction_result(result, show_metadata=False, output_format="json")
    data = json.loads(output)
    assert data["tables"][0]["data_csv"] == "col1,col2\nval1,val2"
    mock_df.to_csv.assert_called_once_with(index=False)


def test_format_extraction_result_markdown() -> None:
    result = ExtractionResult(
        content="# Heading\n\nBody text",
        mime_type="text/markdown",
        metadata={"title": "Test Document"},
        tables=[{"text": "Fallback table content"}],  # type: ignore[typeddict-item]
    )

    output = format_extraction_result(result, show_metadata=True, output_format="markdown")
    assert "# Heading" in output
    assert "## Tables" in output
    assert "Fallback table content" in output
    assert "## Metadata" in output
    assert '"title": "Test Document"' in output


def test_load_config_with_path(tmp_path: Path) -> None:
    config_file = tmp_path / "config.toml"
    config_file.write_text("force_ocr = true")

    with patch("kreuzberg.cli.load_config_from_file") as mock_load:
        mock_load.return_value = {"force_ocr": True}
        result = _load_config(config_file, verbose=False)

    assert result == {"force_ocr": True}
    mock_load.assert_called_once_with(config_file)


def test_load_config_default(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"

    with (
        patch("kreuzberg.cli.find_config_file") as mock_find,
        patch("kreuzberg.cli.load_config_from_file") as mock_load,
        patch("kreuzberg.cli.console") as mock_console,
    ):
        mock_find.return_value = config_file
        mock_load.return_value = {"chunk_content": True}

        result = _load_config(None, verbose=True)

        assert result == {"chunk_content": True}
        mock_console.print.assert_called_once()


def test_load_config_default_error() -> None:
    with (
        patch("kreuzberg.cli.find_config_file") as mock_find,
        patch("kreuzberg.cli.load_config_from_file") as mock_load,
    ):
        mock_find.return_value = Path("config.toml")
        mock_load.side_effect = Exception("Load error")

        result = _load_config(None, verbose=False)

        assert result == {}


def test_build_cli_args_basic() -> None:
    params = {
        "force_ocr": True,
        "chunk_content": False,
        "extract_tables": True,
        "extract_entities": False,
        "extract_keywords": False,
        "auto_detect_language": True,
        "keyword_count": 10,
        "max_chars": 1000,
        "max_overlap": 200,
        "ocr_backend": "tesseract",
        "tesseract_lang": None,
        "tesseract_psm": None,
        "tesseract_output_format": None,
        "enable_table_detection": False,
        "easyocr_languages": None,
        "paddleocr_languages": None,
    }

    result = _build_cli_args(params)

    assert result["force_ocr"] is True
    assert result["chunk_content"] is None
    assert result["extract_tables"] is True
    assert result["auto_detect_language"] is True
    assert result["keyword_count"] is None
    assert result["ocr_backend"] == "tesseract"


def test_build_cli_args_tesseract() -> None:
    params = {
        "force_ocr": False,
        "chunk_content": False,
        "extract_tables": False,
        "extract_entities": False,
        "extract_keywords": False,
        "auto_detect_language": False,
        "keyword_count": 10,
        "max_chars": 1000,
        "max_overlap": 200,
        "ocr_backend": "tesseract",
        "tesseract_lang": "eng+deu",
        "tesseract_psm": 3,
        "tesseract_output_format": "tsv",
        "enable_table_detection": True,
        "easyocr_languages": None,
        "paddleocr_languages": None,
    }

    result = _build_cli_args(params)

    assert "tesseract_config" in result
    assert result["tesseract_config"]["language"] == "eng+deu"
    assert result["tesseract_config"]["psm"] == 3
    assert result["tesseract_config"]["output_format"] == "tsv"
    assert result["tesseract_config"]["enable_table_detection"] is True


def test_build_cli_args_easyocr() -> None:
    params = {
        "force_ocr": False,
        "chunk_content": False,
        "extract_tables": False,
        "extract_entities": False,
        "extract_keywords": False,
        "auto_detect_language": False,
        "keyword_count": 10,
        "max_chars": 1000,
        "max_overlap": 200,
        "ocr_backend": "easyocr",
        "tesseract_lang": None,
        "tesseract_psm": None,
        "tesseract_output_format": None,
        "enable_table_detection": False,
        "easyocr_languages": "en,de",
        "paddleocr_languages": None,
    }

    result = _build_cli_args(params)

    assert "easyocr_config" in result
    assert result["easyocr_config"]["languages"] == ["en", "de"]


def test_build_cli_args_paddleocr() -> None:
    params = {
        "force_ocr": False,
        "chunk_content": False,
        "extract_tables": False,
        "extract_entities": False,
        "extract_keywords": False,
        "auto_detect_language": False,
        "keyword_count": 10,
        "max_chars": 1000,
        "max_overlap": 200,
        "ocr_backend": "paddleocr",
        "tesseract_lang": None,
        "tesseract_psm": None,
        "tesseract_output_format": None,
        "enable_table_detection": False,
        "easyocr_languages": None,
        "paddleocr_languages": "en,ch_sim",
    }

    result = _build_cli_args(params)

    assert "paddleocr_config" in result
    assert result["paddleocr_config"]["languages"] == ["en", "ch_sim"]


def test_perform_extraction_from_file(tmp_path: Path) -> None:
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")

    mock_config = Mock()
    mock_result = ExtractionResult(content="Extracted", mime_type="text/plain")

    with patch("kreuzberg.cli.extract_file_sync") as mock_extract:
        mock_extract.return_value = mock_result

        result = _perform_extraction(test_file, mock_config, verbose=False)

        assert result == mock_result
        mock_extract.assert_called_once_with(str(test_file), config=mock_config)


def test_perform_extraction_from_stdin() -> None:
    mock_config = Mock()
    mock_result = ExtractionResult(content="Extracted", mime_type="text/plain")

    with (
        patch("sys.stdin.buffer.read") as mock_stdin,
        patch("kreuzberg.cli.extract_bytes_sync") as mock_extract,
    ):
        mock_stdin.return_value = b"Test input"
        mock_extract.return_value = mock_result

        result = _perform_extraction(None, mock_config, verbose=True)
        assert result == mock_result
        # MIME type detection now uses simple content inspection
        mock_extract.assert_called_once_with(b"Test input", "text/plain", config=mock_config)


def test_perform_extraction_from_stdin_text_fallback() -> None:
    mock_config = Mock()
    mock_result = ExtractionResult(content="Extracted", mime_type="text/plain")

    with (
        patch("sys.stdin") as mock_stdin,
        patch("kreuzberg.cli.extract_bytes_sync") as mock_extract,
    ):
        mock_stdin.buffer.read.side_effect = Exception("No buffer")
        mock_stdin.read.return_value = "Test input"
        mock_extract.return_value = mock_result

        result = _perform_extraction(None, mock_config, verbose=False)
        assert result == mock_result
        mock_extract.assert_called_once_with(b"Test input", "text/plain", config=mock_config)


def test_perform_extraction_stdin_detect_html() -> None:
    mock_config = Mock()
    mock_result = ExtractionResult(content="Extracted", mime_type="text/html")

    with (
        patch("sys.stdin.buffer.read") as mock_stdin,
        patch("kreuzberg.cli.extract_bytes_sync") as mock_extract,
    ):
        mock_stdin.return_value = b"<html><body>Test</body></html>"
        mock_extract.return_value = mock_result

        result = _perform_extraction(Path("-"), mock_config, verbose=False)
        assert result == mock_result
        # HTML detection now works through content inspection
        mock_extract.assert_called_once_with(b"<html><body>Test</body></html>", "text/html", config=mock_config)


def test_perform_extraction_stdin_detect_json() -> None:
    mock_config = Mock()
    mock_result = ExtractionResult(content="Extracted", mime_type="application/json")

    with (
        patch("sys.stdin.buffer.read") as mock_stdin,
        patch("kreuzberg.cli.extract_bytes_sync") as mock_extract,
    ):
        mock_stdin.return_value = b'{"test": "data"}'
        mock_extract.return_value = mock_result

        result = _perform_extraction(Path("-"), mock_config, verbose=False)
        assert result == mock_result
        # JSON detection now works through content inspection
        mock_extract.assert_called_once_with(b'{"test": "data"}', "application/json", config=mock_config)


def test_write_output_to_file(tmp_path: Path) -> None:
    output_file = tmp_path / "output.txt"
    result = ExtractionResult(content="Test output", mime_type="text/plain")

    _write_output(result, output_file, show_metadata=False, output_format="text", verbose=True)

    assert output_file.read_text() == "Test output"


def test_write_output_to_stdout() -> None:
    result = ExtractionResult(content="Test output", mime_type="text/plain")

    with patch("click.echo") as mock_echo:
        _write_output(result, None, show_metadata=False, output_format="text", verbose=False)
        mock_echo.assert_called_once_with("Test output")


def test_write_output_unicode_error() -> None:
    result = ExtractionResult(content="Test 🔥 output", mime_type="text/plain")

    with (
        patch("click.echo") as mock_echo,
        patch("sys.stdout.buffer.write") as mock_buffer,
    ):
        mock_echo.side_effect = UnicodeEncodeError("ascii", "test", 0, 1, "ordinal not in range")
        _write_output(result, None, show_metadata=False, output_format="text", verbose=False)
        mock_buffer.assert_called_once()


def test_handle_error_missing_dependency() -> None:
    error = MissingDependencyError("Missing package", context={"dependency_group": "test"})

    with (
        patch("kreuzberg.cli.console") as mock_console,
        patch("sys.exit") as mock_exit,
    ):
        handle_error(error, verbose=False)
        mock_console.print.assert_called_once()
        mock_exit.assert_called_once_with(2)


def test_handle_error_kreuzberg_error() -> None:
    error = KreuzbergError("Test error", context={"key": "value"})

    with (
        patch("kreuzberg.cli.console") as mock_console,
        patch("sys.exit") as mock_exit,
    ):
        handle_error(error, verbose=True)
        assert mock_console.print.call_count >= 2
        mock_exit.assert_called_once_with(1)


def test_handle_error_generic() -> None:
    error = ValueError("Generic error")

    with (
        patch("kreuzberg.cli.console") as mock_console,
        patch("sys.exit") as mock_exit,
        patch("traceback.print_exc") as mock_traceback,
    ):
        handle_error(error, verbose=True)
        mock_console.print.assert_called()
        mock_traceback.assert_called_once()
        mock_exit.assert_called_once_with(1)


def test_cli_no_command() -> None:
    runner = CliRunner()
    result = runner.invoke(cli, [])
    assert result.exit_code == 0
    assert "Kreuzberg - Text extraction" in result.output


def test_cli_version() -> None:
    runner = CliRunner()
    result = runner.invoke(cli, ["--version"])
    assert result.exit_code == 0
    assert "version" in result.output.lower()


def test_extract_command_basic(tmp_path: Path) -> None:
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")

    mock_result = ExtractionResult(content="Extracted text", mime_type="text/plain")

    runner = CliRunner()
    with patch("kreuzberg.cli.extract_file_sync") as mock_extract:
        mock_extract.return_value = mock_result

        result = runner.invoke(extract, [str(test_file)])

        assert result.exit_code == 0
        assert "Extracted text" in result.output


def test_extract_command_with_options(tmp_path: Path) -> None:
    test_file = tmp_path / "test.pdf"
    test_file.touch()
    output_file = tmp_path / "output.txt"

    mock_result = ExtractionResult(content="Extracted", mime_type="application/pdf")

    runner = CliRunner()
    with patch("kreuzberg.cli.extract_file_sync") as mock_extract:
        mock_extract.return_value = mock_result

        result = runner.invoke(
            extract,
            [
                str(test_file),
                "-o",
                str(output_file),
                "--force-ocr",
                "--chunk-content",
                "--extract-tables",
                "--output-format",
                "json",
                "--verbose",
            ],
        )

        assert result.exit_code == 0
        assert output_file.exists()


def test_extract_command_markdown_output(tmp_path: Path) -> None:
    test_file = tmp_path / "test.md"
    test_file.touch()

    mock_result = ExtractionResult(
        content="Document intro",
        mime_type="text/markdown",
        metadata={"title": "Markdown Doc"},
        tables=[{"text": "Some table data"}],  # type: ignore[typeddict-item]
    )

    runner = CliRunner()
    with patch("kreuzberg.cli.extract_file_sync") as mock_extract:
        mock_extract.return_value = mock_result

        result = runner.invoke(
            extract,
            [
                str(test_file),
                "--output-format",
                "markdown",
                "--show-metadata",
            ],
        )

    assert result.exit_code == 0
    assert "Document intro" in result.output
    assert "## Tables" in result.output
    assert "## Metadata" in result.output


def test_extract_command_error() -> None:
    runner = CliRunner()
    with patch("kreuzberg.cli._perform_extraction") as mock_extract:
        mock_extract.side_effect = KreuzbergError("Test error")

        result = runner.invoke(extract, ["nonexistent.pdf"])

        assert result.exit_code == 2


def test_config_command_with_file(tmp_path: Path) -> None:
    config_file = tmp_path / "config.toml"
    config_file.write_text("force_ocr = true")

    runner = CliRunner()
    with patch("kreuzberg.cli.load_config_from_file") as mock_load:
        mock_load.return_value = {"force_ocr": True}

        result = runner.invoke(config, ["--config", str(config_file)])

        assert result.exit_code == 0
        assert "force_ocr" in result.output


def test_config_command_no_file() -> None:
    runner = CliRunner()
    with patch("kreuzberg.cli.find_config_file") as mock_find:
        mock_find.return_value = None

        result = runner.invoke(config, [])

        assert result.exit_code == 0
        assert "No configuration file found" in result.output
        assert "Default configuration" in result.output


def test_config_command_error() -> None:
    runner = CliRunner()
    with patch("kreuzberg.cli.find_config_file") as mock_find:
        mock_find.side_effect = Exception("Config error")

        result = runner.invoke(config, [])

        assert result.exit_code == 1
