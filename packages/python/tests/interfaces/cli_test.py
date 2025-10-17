"""Tests for the Python CLI wrapper.

The Python CLI is a thin wrapper around Rust core functionality via bindings.
Tests focus on CLI argument parsing and integration with Rust functions.
"""

from __future__ import annotations

from typing import TYPE_CHECKING
from unittest.mock import patch

from click.testing import CliRunner

from kreuzberg import ExtractionConfig, ExtractionResult, OcrConfig
from kreuzberg.cli import extract, main, mcp, serve

if TYPE_CHECKING:
    from pathlib import Path


def test_main_no_command() -> None:
    """Test main CLI without command shows help."""
    runner = CliRunner()
    result = runner.invoke(main, [])
    assert result.exit_code == 0
    assert "Kreuzberg" in result.output


def test_main_version() -> None:
    """Test version flag."""
    runner = CliRunner()
    result = runner.invoke(main, ["--version"])
    assert result.exit_code == 0


def test_extract_basic(tmp_path: Path) -> None:
    """Test basic file extraction."""
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content", encoding="utf-8")

    mock_result = ExtractionResult(
        content="Extracted text",
        mime_type="text/plain",
        metadata={},
        tables=[],
    )

    runner = CliRunner()
    with patch("kreuzberg.cli.extract_file_sync") as mock_extract:
        mock_extract.return_value = mock_result
        result = runner.invoke(extract, [str(test_file)])

        assert result.exit_code == 0
        assert "Extracted text" in result.output
        mock_extract.assert_called_once()


def test_extract_with_output_file(tmp_path: Path) -> None:
    """Test extraction with output file."""
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content", encoding="utf-8")
    output_file = tmp_path / "output.txt"

    mock_result = ExtractionResult(
        content="Extracted text",
        mime_type="text/plain",
        metadata={},
        tables=[],
    )

    runner = CliRunner()
    with patch("kreuzberg.cli.extract_file_sync") as mock_extract:
        mock_extract.return_value = mock_result
        result = runner.invoke(extract, [str(test_file), "-o", str(output_file)])

        assert result.exit_code == 0
        assert output_file.exists()
        assert output_file.read_text(encoding="utf-8") == "Extracted text"


def test_extract_with_ocr() -> None:
    """Test extraction with OCR enabled."""
    mock_result = ExtractionResult(
        content="OCR extracted text",
        mime_type="application/pdf",
        metadata={},
        tables=[],
    )

    runner = CliRunner()
    with patch("kreuzberg.cli.extract_file_sync") as mock_extract:
        mock_extract.return_value = mock_result
        result = runner.invoke(extract, ["test.pdf", "--ocr"])

        assert result.exit_code == 0
        call_args = mock_extract.call_args
        config: ExtractionConfig = call_args[0][2]
        assert config.ocr is not None
        assert isinstance(config.ocr, OcrConfig)


def test_extract_with_force_ocr() -> None:
    """Test extraction with force OCR."""
    mock_result = ExtractionResult(
        content="OCR text",
        mime_type="application/pdf",
        metadata={},
        tables=[],
    )

    runner = CliRunner()
    with patch("kreuzberg.cli.extract_file_sync") as mock_extract:
        mock_extract.return_value = mock_result
        result = runner.invoke(extract, ["test.pdf", "--force-ocr"])

        assert result.exit_code == 0
        call_args = mock_extract.call_args
        config: ExtractionConfig = call_args[0][2]
        assert config.force_ocr is True


def test_extract_with_mime_type() -> None:
    """Test extraction with explicit MIME type."""
    mock_result = ExtractionResult(
        content="Text",
        mime_type="application/pdf",
        metadata={},
        tables=[],
    )

    runner = CliRunner()
    with patch("kreuzberg.cli.extract_file_sync") as mock_extract:
        mock_extract.return_value = mock_result
        result = runner.invoke(extract, ["test.pdf", "--mime-type", "application/pdf"])

        assert result.exit_code == 0
        call_args = mock_extract.call_args
        assert call_args[0][1] == "application/pdf"


def test_extract_error_handling() -> None:
    """Test error handling in extract command."""
    runner = CliRunner()
    with patch("kreuzberg.cli.extract_file_sync") as mock_extract:
        mock_extract.side_effect = Exception("Extraction failed")
        result = runner.invoke(extract, ["test.pdf"])

        assert result.exit_code == 1
        assert "Error" in result.output


def test_serve_command() -> None:
    """Test serve command starts API server."""
    runner = CliRunner()
    with patch("kreuzberg.cli.start_api_server") as mock_serve:
        # Simulate KeyboardInterrupt to stop server
        mock_serve.side_effect = KeyboardInterrupt()
        result = runner.invoke(serve, [])

        assert result.exit_code == 0
        mock_serve.assert_called_once_with("0.0.0.0", 8000)


def test_serve_with_custom_host_port() -> None:
    """Test serve command with custom host and port."""
    runner = CliRunner()
    with patch("kreuzberg.cli.start_api_server") as mock_serve:
        mock_serve.side_effect = KeyboardInterrupt()
        result = runner.invoke(serve, ["--host", "127.0.0.1", "--port", "9000"])

        assert result.exit_code == 0
        mock_serve.assert_called_once_with("127.0.0.1", 9000)


def test_serve_error_handling() -> None:
    """Test error handling in serve command."""
    runner = CliRunner()
    with patch("kreuzberg.cli.start_api_server") as mock_serve:
        mock_serve.side_effect = Exception("Server error")
        result = runner.invoke(serve, [])

        assert result.exit_code == 1
        assert "Error" in result.output


def test_mcp_command() -> None:
    """Test MCP command starts MCP server."""
    runner = CliRunner()
    with patch("kreuzberg.cli.start_mcp_server") as mock_mcp:
        mock_mcp.side_effect = KeyboardInterrupt()
        result = runner.invoke(mcp, [])

        assert result.exit_code == 0
        mock_mcp.assert_called_once()


def test_mcp_with_transport() -> None:
    """Test MCP command with transport option."""
    runner = CliRunner()
    with patch("kreuzberg.cli.start_mcp_server") as mock_mcp:
        mock_mcp.side_effect = KeyboardInterrupt()
        result = runner.invoke(mcp, ["--transport", "stdio"])

        assert result.exit_code == 0
        assert "stdio" in result.output


def test_mcp_error_handling() -> None:
    """Test error handling in MCP command."""
    runner = CliRunner()
    with patch("kreuzberg.cli.start_mcp_server") as mock_mcp:
        mock_mcp.side_effect = Exception("MCP error")
        result = runner.invoke(mcp, [])

        assert result.exit_code == 1
        assert "Error" in result.output
