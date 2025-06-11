from pathlib import Path
from click.testing import CliRunner
import pytest

from kreuzberg.cli import cli


def test_extract_file(tmp_path):
    runner = CliRunner()
    
    # Create a test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World")
    
    # Test basic extraction
    result = runner.invoke(cli, ["extract", str(test_file)])
    assert result.exit_code == 0
    assert "Hello World" in result.output

    # Test with output file
    output_file = tmp_path / "output.txt"
    result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])
    assert result.exit_code == 0
    assert output_file.read_text() == "Hello World\n"

    # Test with metadata
    result = runner.invoke(cli, ["extract", str(test_file), "--show-metadata"])
    assert result.exit_code == 0
    assert "Metadata:" in result.output
    assert "Content:" in result.output


def test_extract_stdin():
    runner = CliRunner()
    result = runner.invoke(cli, ["extract"], input="Hello from stdin")
    assert result.exit_code == 0
    assert "Hello from stdin" in result.output


def test_config_file(tmp_path):
    runner = CliRunner()
    
    # Create config file
    config = tmp_path / "pyproject.toml"
    config.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = true
extract_tables = false
max_chars = 1000
ocr_backend = "tesseract"

[tool.kreuzberg.tesseract]
language = "eng"
""")
    
    # Create test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")
    
    # Test with config file
    result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config)])
    assert result.exit_code == 0
    assert "Test content" in result.output


def test_error_handling():
    runner = CliRunner()
    
    # Test missing file
    result = runner.invoke(cli, ["extract", "nonexistent.pdf"])
    assert result.exit_code != 0
    assert "Error:" in result.output

    # Test no input
    result = runner.invoke(cli, ["extract"])
    assert result.exit_code != 0
    assert "No input file provided" in result.output
from pathlib import Path
from click.testing import CliRunner
import pytest

from kreuzberg.cli import cli


def test_extract_file(tmp_path):
    runner = CliRunner()
    
    # Create a test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World")
    
    # Test basic extraction
    result = runner.invoke(cli, ["extract", str(test_file)])
    assert result.exit_code == 0
    assert "Hello World" in result.output

    # Test with output file
    output_file = tmp_path / "output.txt"
    result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])
    assert result.exit_code == 0
    assert output_file.read_text() == "Hello World\n"

    # Test with metadata
    result = runner.invoke(cli, ["extract", str(test_file), "--show-metadata"])
    assert result.exit_code == 0
    assert "Metadata:" in result.output
    assert "Content:" in result.output


def test_extract_stdin():
    runner = CliRunner()
    result = runner.invoke(cli, ["extract"], input="Hello from stdin")
    assert result.exit_code == 0
    assert "Hello from stdin" in result.output


def test_config_file(tmp_path):
    runner = CliRunner()
    
    # Create config file
    config = tmp_path / "pyproject.toml"
    config.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = true
extract_tables = false
max_chars = 1000
ocr_backend = "tesseract"

[tool.kreuzberg.tesseract]
language = "eng"
""")
    
    # Create test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")
    
    # Test with config file
    result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config)])
    assert result.exit_code == 0
    assert "Test content" in result.output


def test_error_handling():
    runner = CliRunner()
    
    # Test missing file
    result = runner.invoke(cli, ["extract", "nonexistent.pdf"])
    assert result.exit_code != 0
    assert "Error:" in result.output

    # Test no input
    result = runner.invoke(cli, ["extract"])
    assert result.exit_code != 0
    assert "No input file provided" in result.output
from pathlib import Path
from click.testing import CliRunner
import pytest

from kreuzberg.cli import cli


def test_extract_file(tmp_path):
    runner = CliRunner()
    
    # Create a test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World")
    
    # Test basic extraction
    result = runner.invoke(cli, ["extract", str(test_file)])
    assert result.exit_code == 0
    assert "Hello World" in result.output

    # Test with output file
    output_file = tmp_path / "output.txt"
    result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])
    assert result.exit_code == 0
    assert output_file.read_text() == "Hello World\n"

    # Test with metadata
    result = runner.invoke(cli, ["extract", str(test_file), "--show-metadata"])
    assert result.exit_code == 0
    assert "Metadata:" in result.output
    assert "Content:" in result.output


def test_extract_stdin():
    runner = CliRunner()
    result = runner.invoke(cli, ["extract"], input="Hello from stdin")
    assert result.exit_code == 0
    assert "Hello from stdin" in result.output


def test_config_file(tmp_path):
    runner = CliRunner()
    
    # Create config file
    config = tmp_path / "pyproject.toml"
    config.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = true
extract_tables = false
max_chars = 1000
ocr_backend = "tesseract"

[tool.kreuzberg.tesseract]
language = "eng"
""")
    
    # Create test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")
    
    # Test with config file
    result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config)])
    assert result.exit_code == 0
    assert "Test content" in result.output


def test_error_handling():
    runner = CliRunner()
    
    # Test missing file
    result = runner.invoke(cli, ["extract", "nonexistent.pdf"])
    assert result.exit_code != 0
    assert "Error:" in result.output

    # Test no input
    result = runner.invoke(cli, ["extract"])
    assert result.exit_code != 0
    assert "No input file provided" in result.output
from pathlib import Path
from click.testing import CliRunner
import pytest

from kreuzberg.cli import cli


def test_extract_file(tmp_path):
    runner = CliRunner()
    
    # Create a test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World")
    
    # Test basic extraction
    result = runner.invoke(cli, ["extract", str(test_file)])
    assert result.exit_code == 0
    assert "Hello World" in result.output

    # Test with output file
    output_file = tmp_path / "output.txt"
    result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])
    assert result.exit_code == 0
    assert output_file.read_text() == "Hello World\n"

    # Test with metadata
    result = runner.invoke(cli, ["extract", str(test_file), "--show-metadata"])
    assert result.exit_code == 0
    assert "Metadata:" in result.output
    assert "Content:" in result.output


def test_extract_stdin():
    runner = CliRunner()
    result = runner.invoke(cli, ["extract"], input="Hello from stdin")
    assert result.exit_code == 0
    assert "Hello from stdin" in result.output


def test_config_file(tmp_path):
    runner = CliRunner()
    
    # Create config file
    config = tmp_path / "pyproject.toml"
    config.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = true
extract_tables = false
max_chars = 1000
ocr_backend = "tesseract"

[tool.kreuzberg.tesseract]
language = "eng"
""")
    
    # Create test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")
    
    # Test with config file
    result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config)])
    assert result.exit_code == 0
    assert "Test content" in result.output


def test_error_handling():
    runner = CliRunner()
    
    # Test missing file
    result = runner.invoke(cli, ["extract", "nonexistent.pdf"])
    assert result.exit_code != 0
    assert "Error:" in result.output

    # Test no input
    result = runner.invoke(cli, ["extract"])
    assert result.exit_code != 0
    assert "No input file provided" in result.output
from pathlib import Path
from click.testing import CliRunner
import pytest

from kreuzberg.cli import cli


def test_extract_file(tmp_path):
    runner = CliRunner()
    
    # Create a test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World")
    
    # Test basic extraction
    result = runner.invoke(cli, ["extract", str(test_file)])
    assert result.exit_code == 0
    assert "Hello World" in result.output

    # Test with output file
    output_file = tmp_path / "output.txt"
    result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])
    assert result.exit_code == 0
    assert output_file.read_text() == "Hello World\n"

    # Test with metadata
    result = runner.invoke(cli, ["extract", str(test_file), "--show-metadata"])
    assert result.exit_code == 0
    assert "Metadata:" in result.output
    assert "Content:" in result.output


def test_extract_stdin():
    runner = CliRunner()
    result = runner.invoke(cli, ["extract"], input="Hello from stdin")
    assert result.exit_code == 0
    assert "Hello from stdin" in result.output


def test_config_file(tmp_path):
    runner = CliRunner()
    
    # Create config file
    config = tmp_path / "pyproject.toml"
    config.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = true
extract_tables = false
max_chars = 1000
ocr_backend = "tesseract"

[tool.kreuzberg.tesseract]
language = "eng"
""")
    
    # Create test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")
    
    # Test with config file
    result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config)])
    assert result.exit_code == 0
    assert "Test content" in result.output


def test_error_handling():
    runner = CliRunner()
    
    # Test missing file
    result = runner.invoke(cli, ["extract", "nonexistent.pdf"])
    assert result.exit_code != 0
    assert "Error:" in result.output

    # Test no input
    result = runner.invoke(cli, ["extract"])
    assert result.exit_code != 0
    assert "No input file provided" in result.output
from pathlib import Path
from click.testing import CliRunner
import pytest

from kreuzberg.cli import cli


def test_extract_file(tmp_path):
    runner = CliRunner()
    
    # Create a test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World")
    
    # Test basic extraction
    result = runner.invoke(cli, ["extract", str(test_file)])
    assert result.exit_code == 0
    assert "Hello World" in result.output

    # Test with output file
    output_file = tmp_path / "output.txt"
    result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])
    assert result.exit_code == 0
    assert output_file.read_text() == "Hello World\n"

    # Test with metadata
    result = runner.invoke(cli, ["extract", str(test_file), "--show-metadata"])
    assert result.exit_code == 0
    assert "Metadata:" in result.output
    assert "Content:" in result.output


def test_extract_stdin():
    runner = CliRunner()
    result = runner.invoke(cli, ["extract"], input="Hello from stdin")
    assert result.exit_code == 0
    assert "Hello from stdin" in result.output


def test_config_file(tmp_path):
    runner = CliRunner()
    
    # Create config file
    config = tmp_path / "pyproject.toml"
    config.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = true
extract_tables = false
max_chars = 1000
ocr_backend = "tesseract"

[tool.kreuzberg.tesseract]
language = "eng"
""")
    
    # Create test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")
    
    # Test with config file
    result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config)])
    assert result.exit_code == 0
    assert "Test content" in result.output


def test_error_handling():
    runner = CliRunner()
    
    # Test missing file
    result = runner.invoke(cli, ["extract", "nonexistent.pdf"])
    assert result.exit_code != 0
    assert "Error:" in result.output

    # Test no input
    result = runner.invoke(cli, ["extract"])
    assert result.exit_code != 0
    assert "No input file provided" in result.output
from pathlib import Path
from click.testing import CliRunner
import pytest

from kreuzberg.cli import cli


def test_extract_file(tmp_path):
    runner = CliRunner()
    
    # Create a test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World")
    
    # Test basic extraction
    result = runner.invoke(cli, ["extract", str(test_file)])
    assert result.exit_code == 0
    assert "Hello World" in result.output

    # Test with output file
    output_file = tmp_path / "output.txt"
    result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])
    assert result.exit_code == 0
    assert output_file.read_text() == "Hello World\n"

    # Test with metadata
    result = runner.invoke(cli, ["extract", str(test_file), "--show-metadata"])
    assert result.exit_code == 0
    assert "Metadata:" in result.output
    assert "Content:" in result.output


def test_extract_stdin():
    runner = CliRunner()
    result = runner.invoke(cli, ["extract"], input="Hello from stdin")
    assert result.exit_code == 0
    assert "Hello from stdin" in result.output


def test_config_file(tmp_path):
    runner = CliRunner()
    
    # Create config file
    config = tmp_path / "pyproject.toml"
    config.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = true
extract_tables = false
max_chars = 1000
ocr_backend = "tesseract"

[tool.kreuzberg.tesseract]
language = "eng"
""")
    
    # Create test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")
    
    # Test with config file
    result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config)])
    assert result.exit_code == 0
    assert "Test content" in result.output


def test_error_handling():
    runner = CliRunner()
    
    # Test missing file
    result = runner.invoke(cli, ["extract", "nonexistent.pdf"])
    assert result.exit_code != 0
    assert "Error:" in result.output

    # Test no input
    result = runner.invoke(cli, ["extract"])
    assert result.exit_code != 0
    assert "No input file provided" in result.output
from pathlib import Path
from click.testing import CliRunner
import pytest

from kreuzberg.cli import cli


def test_extract_file(tmp_path):
    runner = CliRunner()
    
    # Create a test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World")
    
    # Test basic extraction
    result = runner.invoke(cli, ["extract", str(test_file)])
    assert result.exit_code == 0
    assert "Hello World" in result.output

    # Test with output file
    output_file = tmp_path / "output.txt"
    result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])
    assert result.exit_code == 0
    assert output_file.read_text() == "Hello World\n"

    # Test with metadata
    result = runner.invoke(cli, ["extract", str(test_file), "--show-metadata"])
    assert result.exit_code == 0
    assert "Metadata:" in result.output
    assert "Content:" in result.output


def test_extract_stdin():
    runner = CliRunner()
    result = runner.invoke(cli, ["extract"], input="Hello from stdin")
    assert result.exit_code == 0
    assert "Hello from stdin" in result.output


def test_config_file(tmp_path):
    runner = CliRunner()
    
    # Create config file
    config = tmp_path / "pyproject.toml"
    config.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = true
extract_tables = false
max_chars = 1000
ocr_backend = "tesseract"

[tool.kreuzberg.tesseract]
language = "eng"
""")
    
    # Create test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")
    
    # Test with config file
    result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config)])
    assert result.exit_code == 0
    assert "Test content" in result.output


def test_error_handling():
    runner = CliRunner()
    
    # Test missing file
    result = runner.invoke(cli, ["extract", "nonexistent.pdf"])
    assert result.exit_code != 0
    assert "Error:" in result.output

    # Test no input
    result = runner.invoke(cli, ["extract"])
    assert result.exit_code != 0
    assert "No input file provided" in result.output
from pathlib import Path
from click.testing import CliRunner
import pytest

from kreuzberg.cli import cli


def test_extract_file(tmp_path):
    runner = CliRunner()
    
    # Create a test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World")
    
    # Test basic extraction
    result = runner.invoke(cli, ["extract", str(test_file)])
    assert result.exit_code == 0
    assert "Hello World" in result.output

    # Test with output file
    output_file = tmp_path / "output.txt"
    result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])
    assert result.exit_code == 0
    assert output_file.read_text() == "Hello World\n"

    # Test with metadata
    result = runner.invoke(cli, ["extract", str(test_file), "--show-metadata"])
    assert result.exit_code == 0
    assert "Metadata:" in result.output
    assert "Content:" in result.output


def test_extract_stdin():
    runner = CliRunner()
    result = runner.invoke(cli, ["extract"], input="Hello from stdin")
    assert result.exit_code == 0
    assert "Hello from stdin" in result.output


def test_config_file(tmp_path):
    runner = CliRunner()
    
    # Create config file
    config = tmp_path / "pyproject.toml"
    config.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = true
extract_tables = false
max_chars = 1000
ocr_backend = "tesseract"

[tool.kreuzberg.tesseract]
language = "eng"
""")
    
    # Create test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")
    
    # Test with config file
    result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config)])
    assert result.exit_code == 0
    assert "Test content" in result.output


def test_error_handling():
    runner = CliRunner()
    
    # Test missing file
    result = runner.invoke(cli, ["extract", "nonexistent.pdf"])
    assert result.exit_code != 0
    assert "Error:" in result.output

    # Test no input
    result = runner.invoke(cli, ["extract"])
    assert result.exit_code != 0
    assert "No input file provided" in result.output
from pathlib import Path
from click.testing import CliRunner
import pytest

from kreuzberg.cli import cli


def test_extract_file(tmp_path):
    runner = CliRunner()
    
    # Create a test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World")
    
    # Test basic extraction
    result = runner.invoke(cli, ["extract", str(test_file)])
    assert result.exit_code == 0
    assert "Hello World" in result.output

    # Test with output file
    output_file = tmp_path / "output.txt"
    result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])
    assert result.exit_code == 0
    assert output_file.read_text() == "Hello World\n"

    # Test with metadata
    result = runner.invoke(cli, ["extract", str(test_file), "--show-metadata"])
    assert result.exit_code == 0
    assert "Metadata:" in result.output
    assert "Content:" in result.output


def test_extract_stdin():
    runner = CliRunner()
    result = runner.invoke(cli, ["extract"], input="Hello from stdin")
    assert result.exit_code == 0
    assert "Hello from stdin" in result.output


def test_config_file(tmp_path):
    runner = CliRunner()
    
    # Create config file
    config = tmp_path / "pyproject.toml"
    config.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = true
extract_tables = false
max_chars = 1000
ocr_backend = "tesseract"

[tool.kreuzberg.tesseract]
language = "eng"
""")
    
    # Create test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")
    
    # Test with config file
    result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config)])
    assert result.exit_code == 0
    assert "Test content" in result.output


def test_error_handling():
    runner = CliRunner()
    
    # Test missing file
    result = runner.invoke(cli, ["extract", "nonexistent.pdf"])
    assert result.exit_code != 0
    assert "Error:" in result.output

    # Test no input
    result = runner.invoke(cli, ["extract"])
    assert result.exit_code != 0
    assert "No input file provided" in result.output
from pathlib import Path
from click.testing import CliRunner
import pytest

from kreuzberg.cli import cli


def test_extract_file(tmp_path):
    runner = CliRunner()
    
    # Create a test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World")
    
    # Test basic extraction
    result = runner.invoke(cli, ["extract", str(test_file)])
    assert result.exit_code == 0
    assert "Hello World" in result.output

    # Test with output file
    output_file = tmp_path / "output.txt"
    result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])
    assert result.exit_code == 0
    assert output_file.read_text() == "Hello World\n"

    # Test with metadata
    result = runner.invoke(cli, ["extract", str(test_file), "--show-metadata"])
    assert result.exit_code == 0
    assert "Metadata:" in result.output
    assert "Content:" in result.output


def test_extract_stdin():
    runner = CliRunner()
    result = runner.invoke(cli, ["extract"], input="Hello from stdin")
    assert result.exit_code == 0
    assert "Hello from stdin" in result.output


def test_config_file(tmp_path):
    runner = CliRunner()
    
    # Create config file
    config = tmp_path / "pyproject.toml"
    config.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = true
extract_tables = false
max_chars = 1000
ocr_backend = "tesseract"

[tool.kreuzberg.tesseract]
language = "eng"
""")
    
    # Create test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")
    
    # Test with config file
    result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config)])
    assert result.exit_code == 0
    assert "Test content" in result.output


def test_error_handling():
    runner = CliRunner()
    
    # Test missing file
    result = runner.invoke(cli, ["extract", "nonexistent.pdf"])
    assert result.exit_code != 0
    assert "Error:" in result.output

    # Test no input
    result = runner.invoke(cli, ["extract"])
    assert result.exit_code != 0
    assert "No input file provided" in result.output
from pathlib import Path
from click.testing import CliRunner
import pytest

from kreuzberg.cli import cli


def test_extract_file(tmp_path):
    runner = CliRunner()
    
    # Create a test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World")
    
    # Test basic extraction
    result = runner.invoke(cli, ["extract", str(test_file)])
    assert result.exit_code == 0
    assert "Hello World" in result.output

    # Test with output file
    output_file = tmp_path / "output.txt"
    result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])
    assert result.exit_code == 0
    assert output_file.read_text() == "Hello World\n"

    # Test with metadata
    result = runner.invoke(cli, ["extract", str(test_file), "--show-metadata"])
    assert result.exit_code == 0
    assert "Metadata:" in result.output
    assert "Content:" in result.output


def test_extract_stdin():
    runner = CliRunner()
    result = runner.invoke(cli, ["extract"], input="Hello from stdin")
    assert result.exit_code == 0
    assert "Hello from stdin" in result.output


def test_config_file(tmp_path):
    runner = CliRunner()
    
    # Create config file
    config = tmp_path / "pyproject.toml"
    config.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = true
extract_tables = false
max_chars = 1000
ocr_backend = "tesseract"

[tool.kreuzberg.tesseract]
language = "eng"
""")
    
    # Create test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")
    
    # Test with config file
    result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config)])
    assert result.exit_code == 0
    assert "Test content" in result.output


def test_error_handling():
    runner = CliRunner()
    
    # Test missing file
    result = runner.invoke(cli, ["extract", "nonexistent.pdf"])
    assert result.exit_code != 0
    assert "Error:" in result.output

    # Test no input
    result = runner.invoke(cli, ["extract"])
    assert result.exit_code != 0
    assert "No input file provided" in result.output
from pathlib import Path
from click.testing import CliRunner
import pytest

from kreuzberg.cli import cli


def test_extract_file(tmp_path):
    runner = CliRunner()
    
    # Create a test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World")
    
    # Test basic extraction
    result = runner.invoke(cli, ["extract", str(test_file)])
    assert result.exit_code == 0
    assert "Hello World" in result.output

    # Test with output file
    output_file = tmp_path / "output.txt"
    result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])
    assert result.exit_code == 0
    assert output_file.read_text() == "Hello World\n"

    # Test with metadata
    result = runner.invoke(cli, ["extract", str(test_file), "--show-metadata"])
    assert result.exit_code == 0
    assert "Metadata:" in result.output
    assert "Content:" in result.output


def test_extract_stdin():
    runner = CliRunner()
    result = runner.invoke(cli, ["extract"], input="Hello from stdin")
    assert result.exit_code == 0
    assert "Hello from stdin" in result.output


def test_config_file(tmp_path):
    runner = CliRunner()
    
    # Create config file
    config = tmp_path / "pyproject.toml"
    config.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = true
extract_tables = false
max_chars = 1000
ocr_backend = "tesseract"

[tool.kreuzberg.tesseract]
language = "eng"
""")
    
    # Create test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")
    
    # Test with config file
    result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config)])
    assert result.exit_code == 0
    assert "Test content" in result.output


def test_error_handling():
    runner = CliRunner()
    
    # Test missing file
    result = runner.invoke(cli, ["extract", "nonexistent.pdf"])
    assert result.exit_code != 0
    assert "Error:" in result.output

    # Test no input
    result = runner.invoke(cli, ["extract"])
    assert result.exit_code != 0
    assert "No input file provided" in result.output
from pathlib import Path
from click.testing import CliRunner
import pytest

from kreuzberg.cli import cli


def test_extract_file(tmp_path):
    runner = CliRunner()
    
    # Create a test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World")
    
    # Test basic extraction
    result = runner.invoke(cli, ["extract", str(test_file)])
    assert result.exit_code == 0
    assert "Hello World" in result.output

    # Test with output file
    output_file = tmp_path / "output.txt"
    result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])
    assert result.exit_code == 0
    assert output_file.read_text() == "Hello World\n"

    # Test with metadata
    result = runner.invoke(cli, ["extract", str(test_file), "--show-metadata"])
    assert result.exit_code == 0
    assert "Metadata:" in result.output
    assert "Content:" in result.output


def test_extract_stdin():
    runner = CliRunner()
    result = runner.invoke(cli, ["extract"], input="Hello from stdin")
    assert result.exit_code == 0
    assert "Hello from stdin" in result.output


def test_config_file(tmp_path):
    runner = CliRunner()
    
    # Create config file
    config = tmp_path / "pyproject.toml"
    config.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = true
extract_tables = false
max_chars = 1000
ocr_backend = "tesseract"

[tool.kreuzberg.tesseract]
language = "eng"
""")
    
    # Create test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")
    
    # Test with config file
    result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config)])
    assert result.exit_code == 0
    assert "Test content" in result.output


def test_error_handling():
    runner = CliRunner()
    
    # Test missing file
    result = runner.invoke(cli, ["extract", "nonexistent.pdf"])
    assert result.exit_code != 0
    assert "Error:" in result.output

    # Test no input
    result = runner.invoke(cli, ["extract"])
    assert result.exit_code != 0
    assert "No input file provided" in result.output
from pathlib import Path
from click.testing import CliRunner
import pytest

from kreuzberg.cli import cli


def test_extract_file(tmp_path):
    runner = CliRunner()
    
    # Create a test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World")
    
    # Test basic extraction
    result = runner.invoke(cli, ["extract", str(test_file)])
    assert result.exit_code == 0
    assert "Hello World" in result.output

    # Test with output file
    output_file = tmp_path / "output.txt"
    result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])
    assert result.exit_code == 0
    assert output_file.read_text() == "Hello World\n"

    # Test with metadata
    result = runner.invoke(cli, ["extract", str(test_file), "--show-metadata"])
    assert result.exit_code == 0
    assert "Metadata:" in result.output
    assert "Content:" in result.output


def test_extract_stdin():
    runner = CliRunner()
    result = runner.invoke(cli, ["extract"], input="Hello from stdin")
    assert result.exit_code == 0
    assert "Hello from stdin" in result.output


def test_config_file(tmp_path):
    runner = CliRunner()
    
    # Create config file
    config = tmp_path / "pyproject.toml"
    config.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = true
extract_tables = false
max_chars = 1000
ocr_backend = "tesseract"

[tool.kreuzberg.tesseract]
language = "eng"
""")
    
    # Create test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")
    
    # Test with config file
    result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config)])
    assert result.exit_code == 0
    assert "Test content" in result.output


def test_error_handling():
    runner = CliRunner()
    
    # Test missing file
    result = runner.invoke(cli, ["extract", "nonexistent.pdf"])
    assert result.exit_code != 0
    assert "Error:" in result.output

    # Test no input
    result = runner.invoke(cli, ["extract"])
    assert result.exit_code != 0
    assert "No input file provided" in result.output
from pathlib import Path
from click.testing import CliRunner
import pytest

from kreuzberg.cli import cli


def test_extract_file(tmp_path):
    runner = CliRunner()
    
    # Create a test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World")
    
    # Test basic extraction
    result = runner.invoke(cli, ["extract", str(test_file)])
    assert result.exit_code == 0
    assert "Hello World" in result.output

    # Test with output file
    output_file = tmp_path / "output.txt"
    result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])
    assert result.exit_code == 0
    assert output_file.read_text() == "Hello World\n"

    # Test with metadata
    result = runner.invoke(cli, ["extract", str(test_file), "--show-metadata"])
    assert result.exit_code == 0
    assert "Metadata:" in result.output
    assert "Content:" in result.output


def test_extract_stdin():
    runner = CliRunner()
    result = runner.invoke(cli, ["extract"], input="Hello from stdin")
    assert result.exit_code == 0
    assert "Hello from stdin" in result.output


def test_config_file(tmp_path):
    runner = CliRunner()
    
    # Create config file
    config = tmp_path / "pyproject.toml"
    config.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = true
extract_tables = false
max_chars = 1000
ocr_backend = "tesseract"

[tool.kreuzberg.tesseract]
language = "eng"
""")
    
    # Create test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")
    
    # Test with config file
    result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config)])
    assert result.exit_code == 0
    assert "Test content" in result.output


def test_error_handling():
    runner = CliRunner()
    
    # Test missing file
    result = runner.invoke(cli, ["extract", "nonexistent.pdf"])
    assert result.exit_code != 0
    assert "Error:" in result.output

    # Test no input
    result = runner.invoke(cli, ["extract"])
    assert result.exit_code != 0
    assert "No input file provided" in result.output
from pathlib import Path
from click.testing import CliRunner
import pytest

from kreuzberg.cli import cli


def test_extract_file(tmp_path):
    runner = CliRunner()
    
    # Create a test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World")
    
    # Test basic extraction
    result = runner.invoke(cli, ["extract", str(test_file)])
    assert result.exit_code == 0
    assert "Hello World" in result.output

    # Test with output file
    output_file = tmp_path / "output.txt"
    result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])
    assert result.exit_code == 0
    assert output_file.read_text() == "Hello World\n"

    # Test with metadata
    result = runner.invoke(cli, ["extract", str(test_file), "--show-metadata"])
    assert result.exit_code == 0
    assert "Metadata:" in result.output
    assert "Content:" in result.output


def test_extract_stdin():
    runner = CliRunner()
    result = runner.invoke(cli, ["extract"], input="Hello from stdin")
    assert result.exit_code == 0
    assert "Hello from stdin" in result.output


def test_config_file(tmp_path):
    runner = CliRunner()
    
    # Create config file
    config = tmp_path / "pyproject.toml"
    config.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = true
extract_tables = false
max_chars = 1000
ocr_backend = "tesseract"

[tool.kreuzberg.tesseract]
language = "eng"
""")
    
    # Create test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")
    
    # Test with config file
    result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config)])
    assert result.exit_code == 0
    assert "Test content" in result.output


def test_error_handling():
    runner = CliRunner()
    
    # Test missing file
    result = runner.invoke(cli, ["extract", "nonexistent.pdf"])
    assert result.exit_code != 0
    assert "Error:" in result.output

    # Test no input
    result = runner.invoke(cli, ["extract"])
    assert result.exit_code != 0
    assert "No input file provided" in result.output
from pathlib import Path
from click.testing import CliRunner
import pytest

from kreuzberg.cli import cli


def test_extract_file(tmp_path):
    runner = CliRunner()
    
    # Create a test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World")
    
    # Test basic extraction
    result = runner.invoke(cli, ["extract", str(test_file)])
    assert result.exit_code == 0
    assert "Hello World" in result.output

    # Test with output file
    output_file = tmp_path / "output.txt"
    result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])
    assert result.exit_code == 0
    assert output_file.read_text() == "Hello World\n"

    # Test with metadata
    result = runner.invoke(cli, ["extract", str(test_file), "--show-metadata"])
    assert result.exit_code == 0
    assert "Metadata:" in result.output
    assert "Content:" in result.output


def test_extract_stdin():
    runner = CliRunner()
    result = runner.invoke(cli, ["extract"], input="Hello from stdin")
    assert result.exit_code == 0
    assert "Hello from stdin" in result.output


def test_config_file(tmp_path):
    runner = CliRunner()
    
    # Create config file
    config = tmp_path / "pyproject.toml"
    config.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = true
extract_tables = false
max_chars = 1000
ocr_backend = "tesseract"

[tool.kreuzberg.tesseract]
language = "eng"
""")
    
    # Create test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")
    
    # Test with config file
    result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config)])
    assert result.exit_code == 0
    assert "Test content" in result.output


def test_error_handling():
    runner = CliRunner()
    
    # Test missing file
    result = runner.invoke(cli, ["extract", "nonexistent.pdf"])
    assert result.exit_code != 0
    assert "Error:" in result.output

    # Test no input
    result = runner.invoke(cli, ["extract"])
    assert result.exit_code != 0
    assert "No input file provided" in result.output
from pathlib import Path
from click.testing import CliRunner
import pytest

from kreuzberg.cli import cli


def test_extract_file(tmp_path):
    runner = CliRunner()
    
    # Create a test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World")
    
    # Test basic extraction
    result = runner.invoke(cli, ["extract", str(test_file)])
    assert result.exit_code == 0
    assert "Hello World" in result.output

    # Test with output file
    output_file = tmp_path / "output.txt"
    result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])
    assert result.exit_code == 0
    assert output_file.read_text() == "Hello World\n"

    # Test with metadata
    result = runner.invoke(cli, ["extract", str(test_file), "--show-metadata"])
    assert result.exit_code == 0
    assert "Metadata:" in result.output
    assert "Content:" in result.output


def test_extract_stdin():
    runner = CliRunner()
    result = runner.invoke(cli, ["extract"], input="Hello from stdin")
    assert result.exit_code == 0
    assert "Hello from stdin" in result.output


def test_config_file(tmp_path):
    runner = CliRunner()
    
    # Create config file
    config = tmp_path / "pyproject.toml"
    config.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = true
extract_tables = false
max_chars = 1000
ocr_backend = "tesseract"

[tool.kreuzberg.tesseract]
language = "eng"
""")
    
    # Create test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")
    
    # Test with config file
    result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config)])
    assert result.exit_code == 0
    assert "Test content" in result.output


def test_error_handling():
    runner = CliRunner()
    
    # Test missing file
    result = runner.invoke(cli, ["extract", "nonexistent.pdf"])
    assert result.exit_code != 0
    assert "Error:" in result.output

    # Test no input
    result = runner.invoke(cli, ["extract"])
    assert result.exit_code != 0
    assert "No input file provided" in result.output
from pathlib import Path
from click.testing import CliRunner
import pytest

from kreuzberg.cli import cli


def test_extract_file(tmp_path):
    runner = CliRunner()
    
    # Create a test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World")
    
    # Test basic extraction
    result = runner.invoke(cli, ["extract", str(test_file)])
    assert result.exit_code == 0
    assert "Hello World" in result.output

    # Test with output file
    output_file = tmp_path / "output.txt"
    result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])
    assert result.exit_code == 0
    assert output_file.read_text() == "Hello World\n"

    # Test with metadata
    result = runner.invoke(cli, ["extract", str(test_file), "--show-metadata"])
    assert result.exit_code == 0
    assert "Metadata:" in result.output
    assert "Content:" in result.output


def test_extract_stdin():
    runner = CliRunner()
    result = runner.invoke(cli, ["extract"], input="Hello from stdin")
    assert result.exit_code == 0
    assert "Hello from stdin" in result.output


def test_config_file(tmp_path):
    runner = CliRunner()
    
    # Create config file
    config = tmp_path / "pyproject.toml"
    config.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = true
extract_tables = false
max_chars = 1000
ocr_backend = "tesseract"

[tool.kreuzberg.tesseract]
language = "eng"
""")
    
    # Create test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")
    
    # Test with config file
    result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config)])
    assert result.exit_code == 0
    assert "Test content" in result.output


def test_error_handling():
    runner = CliRunner()
    
    # Test missing file
    result = runner.invoke(cli, ["extract", "nonexistent.pdf"])
    assert result.exit_code != 0
    assert "Error:" in result.output

    # Test no input
    result = runner.invoke(cli, ["extract"])
    assert result.exit_code != 0
    assert "No input file provided" in result.output
from pathlib import Path
from click.testing import CliRunner
import pytest

from kreuzberg.cli import cli


def test_extract_file(tmp_path):
    runner = CliRunner()
    
    # Create a test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World")
    
    # Test basic extraction
    result = runner.invoke(cli, ["extract", str(test_file)])
    assert result.exit_code == 0
    assert "Hello World" in result.output

    # Test with output file
    output_file = tmp_path / "output.txt"
    result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])
    assert result.exit_code == 0
    assert output_file.read_text() == "Hello World\n"

    # Test with metadata
    result = runner.invoke(cli, ["extract", str(test_file), "--show-metadata"])
    assert result.exit_code == 0
    assert "Metadata:" in result.output
    assert "Content:" in result.output


def test_extract_stdin():
    runner = CliRunner()
    result = runner.invoke(cli, ["extract"], input="Hello from stdin")
    assert result.exit_code == 0
    assert "Hello from stdin" in result.output


def test_config_file(tmp_path):
    runner = CliRunner()
    
    # Create config file
    config = tmp_path / "pyproject.toml"
    config.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = true
extract_tables = false
max_chars = 1000
ocr_backend = "tesseract"

[tool.kreuzberg.tesseract]
language = "eng"
""")
    
    # Create test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")
    
    # Test with config file
    result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config)])
    assert result.exit_code == 0
    assert "Test content" in result.output


def test_error_handling():
    runner = CliRunner()
    
    # Test missing file
    result = runner.invoke(cli, ["extract", "nonexistent.pdf"])
    assert result.exit_code != 0
    assert "Error:" in result.output

    # Test no input
    result = runner.invoke(cli, ["extract"])
    assert result.exit_code != 0
    assert "No input file provided" in result.output
from pathlib import Path
from click.testing import CliRunner
import pytest

from kreuzberg.cli import cli


def test_extract_file(tmp_path):
    runner = CliRunner()
    
    # Create a test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World")
    
    # Test basic extraction
    result = runner.invoke(cli, ["extract", str(test_file)])
    assert result.exit_code == 0
    assert "Hello World" in result.output

    # Test with output file
    output_file = tmp_path / "output.txt"
    result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])
    assert result.exit_code == 0
    assert output_file.read_text() == "Hello World\n"

    # Test with metadata
    result = runner.invoke(cli, ["extract", str(test_file), "--show-metadata"])
    assert result.exit_code == 0
    assert "Metadata:" in result.output
    assert "Content:" in result.output


def test_extract_stdin():
    runner = CliRunner()
    result = runner.invoke(cli, ["extract"], input="Hello from stdin")
    assert result.exit_code == 0
    assert "Hello from stdin" in result.output


def test_config_file(tmp_path):
    runner = CliRunner()
    
    # Create config file
    config = tmp_path / "pyproject.toml"
    config.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = true
extract_tables = false
max_chars = 1000
ocr_backend = "tesseract"

[tool.kreuzberg.tesseract]
language = "eng"
""")
    
    # Create test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")
    
    # Test with config file
    result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config)])
    assert result.exit_code == 0
    assert "Test content" in result.output


def test_error_handling():
    runner = CliRunner()
    
    # Test missing file
    result = runner.invoke(cli, ["extract", "nonexistent.pdf"])
    assert result.exit_code != 0
    assert "Error:" in result.output

    # Test no input
    result = runner.invoke(cli, ["extract"])
    assert result.exit_code != 0
    assert "No input file provided" in result.output
from pathlib import Path
from click.testing import CliRunner
import pytest

from kreuzberg.cli import cli


def test_extract_file(tmp_path):
    runner = CliRunner()
    
    # Create a test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World")
    
    # Test basic extraction
    result = runner.invoke(cli, ["extract", str(test_file)])
    assert result.exit_code == 0
    assert "Hello World" in result.output

    # Test with output file
    output_file = tmp_path / "output.txt"
    result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])
    assert result.exit_code == 0
    assert output_file.read_text() == "Hello World\n"

    # Test with metadata
    result = runner.invoke(cli, ["extract", str(test_file), "--show-metadata"])
    assert result.exit_code == 0
    assert "Metadata:" in result.output
    assert "Content:" in result.output


def test_extract_stdin():
    runner = CliRunner()
    result = runner.invoke(cli, ["extract"], input="Hello from stdin")
    assert result.exit_code == 0
    assert "Hello from stdin" in result.output


def test_config_file(tmp_path):
    runner = CliRunner()
    
    # Create config file
    config = tmp_path / "pyproject.toml"
    config.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = true
extract_tables = false
max_chars = 1000
ocr_backend = "tesseract"

[tool.kreuzberg.tesseract]
language = "eng"
""")
    
    # Create test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")
    
    # Test with config file
    result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config)])
    assert result.exit_code == 0
    assert "Test content" in result.output


def test_error_handling():
    runner = CliRunner()
    
    # Test missing file
    result = runner.invoke(cli, ["extract", "nonexistent.pdf"])
    assert result.exit_code != 0
    assert "Error:" in result.output

    # Test no input
    result = runner.invoke(cli, ["extract"])
    assert result.exit_code != 0
    assert "No input file provided" in result.output
from pathlib import Path
from click.testing import CliRunner
import pytest

from kreuzberg.cli import cli


def test_extract_file(tmp_path):
    runner = CliRunner()
    
    # Create a test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World")
    
    # Test basic extraction
    result = runner.invoke(cli, ["extract", str(test_file)])
    assert result.exit_code == 0
    assert "Hello World" in result.output

    # Test with output file
    output_file = tmp_path / "output.txt"
    result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])
    assert result.exit_code == 0
    assert output_file.read_text() == "Hello World\n"

    # Test with metadata
    result = runner.invoke(cli, ["extract", str(test_file), "--show-metadata"])
    assert result.exit_code == 0
    assert "Metadata:" in result.output
    assert "Content:" in result.output


def test_extract_stdin():
    runner = CliRunner()
    result = runner.invoke(cli, ["extract"], input="Hello from stdin")
    assert result.exit_code == 0
    assert "Hello from stdin" in result.output


def test_config_file(tmp_path):
    runner = CliRunner()
    
    # Create config file
    config = tmp_path / "pyproject.toml"
    config.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = true
extract_tables = false
max_chars = 1000
ocr_backend = "tesseract"

[tool.kreuzberg.tesseract]
language = "eng"
""")
    
    # Create test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")
    
    # Test with config file
    result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config)])
    assert result.exit_code == 0
    assert "Test content" in result.output


def test_error_handling():
    runner = CliRunner()
    
    # Test missing file
    result = runner.invoke(cli, ["extract", "nonexistent.pdf"])
    assert result.exit_code != 0
    assert "Error:" in result.output

    # Test no input
    result = runner.invoke(cli, ["extract"])
    assert result.exit_code != 0
    assert "No input file provided" in result.output
from pathlib import Path
from click.testing import CliRunner
import pytest

from kreuzberg.cli import cli


def test_extract_file(tmp_path):
    runner = CliRunner()
    
    # Create a test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World")
    
    # Test basic extraction
    result = runner.invoke(cli, ["extract", str(test_file)])
    assert result.exit_code == 0
    assert "Hello World" in result.output

    # Test with output file
    output_file = tmp_path / "output.txt"
    result = runner.invoke(cli, ["extract", str(test_file), "-o", str(output_file)])
    assert result.exit_code == 0
    assert output_file.read_text() == "Hello World\n"

    # Test with metadata
    result = runner.invoke(cli, ["extract", str(test_file), "--show-metadata"])
    assert result.exit_code == 0
    assert "Metadata:" in result.output
    assert "Content:" in result.output


def test_extract_stdin():
    runner = CliRunner()
    result = runner.invoke(cli, ["extract"], input="Hello from stdin")
    assert result.exit_code == 0
    assert "Hello from stdin" in result.output


def test_config_file(tmp_path):
    runner = CliRunner()
    
    # Create config file
    config = tmp_path / "pyproject.toml"
    config.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = true
extract_tables = false
max_chars = 1000
ocr_backend = "tesseract"

[tool.kreuzberg.tesseract]
language = "eng"
""")
    
    # Create test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")
    
    # Test with config file
    result = runner.invoke(cli, ["extract", str(test_file), "--config", str(config)])
    assert result.exit_code == 0
    assert "Test content" in result.output


def test_error_handling():
    runner = CliRunner()
    
    # Test missing file
    result = runner.invoke(cli, ["extract", "nonexistent.pdf"])
    assert result.exit_code != 0
    assert "Error:" in result.output

    # Test no input
    result = runner.invoke(cli, ["extract"])
    assert result.exit_code != 0
    assert "No input file provided" in result.output
from pathlib import Path
from click.testing import CliRunner
import pytest

from kreuzberg.cli import cli


def test_extract_file(tmp_path):
    runner = CliRunner()
    
    # Create a test file
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello World")
    
    # Test basic extraction
    result = runner.invoke(cli, ["extract", str(test_file)])
    assert result.exit_code ==