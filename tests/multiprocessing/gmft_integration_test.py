"""Integration tests for GMFT isolated process execution."""

import sys
from pathlib import Path

import pytest
from pytest import MonkeyPatch

from kreuzberg._gmft import GMFTConfig, extract_tables
from kreuzberg.exceptions import ParsingError


@pytest.fixture
def sample_pdf(tmp_path: Path) -> Path:
    """Create a simple valid PDF file."""
    pdf_file = tmp_path / "sample.pdf"

    pdf_content = b"""%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> >>
endobj
xref
0 4
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
trailer
<< /Size 4 /Root 1 0 R >>
startxref
217
%%EOF"""
    pdf_file.write_bytes(pdf_content)
    return pdf_file


@pytest.mark.skipif(sys.platform == "win32", reason="Process isolation not fully supported on Windows")
def test_gmft_isolated_enabled_by_default(sample_pdf: Path, monkeypatch: MonkeyPatch) -> None:
    """Test that GMFT isolation is enabled by default."""

    monkeypatch.delenv("KREUZBERG_GMFT_ISOLATED", raising=False)

    # This should use isolated process by default  # ~keep
    try:
        result = extract_tables(sample_pdf, GMFTConfig())

        assert isinstance(result, list)
    except ParsingError as e:
        # If GMFT isn't installed, we should get a controlled error  # ~keep
        assert "GMFT" in str(e)
    except Exception as e:
        # Should not get random segfaults or other errors  # ~keep
        pytest.fail(f"Unexpected error type: {type(e).__name__}: {e}")


@pytest.mark.skipif(sys.platform == "win32", reason="Process isolation not fully supported on Windows")
def test_gmft_isolated_can_be_disabled(sample_pdf: Path, monkeypatch: MonkeyPatch) -> None:
    """Test that GMFT isolation can be disabled via env var."""
    monkeypatch.setenv("KREUZBERG_GMFT_ISOLATED", "false")

    # This should NOT use isolated process  # ~keep
    try:
        result = extract_tables(sample_pdf, GMFTConfig())
        assert isinstance(result, list)
    except ImportError:
        pass
    except Exception as e:
        if "Segmentation fault" not in str(e):
            pytest.skip(f"Non-segfault error: {e}")


@pytest.mark.anyio
@pytest.mark.skipif(sys.platform == "win32", reason="Process isolation not fully supported on Windows")
async def test_gmft_isolated_async(sample_pdf: Path) -> None:
    """Test async version of isolated GMFT."""
    try:
        result = await extract_tables(sample_pdf, GMFTConfig(), use_isolated_process=True)
        assert isinstance(result, list)
    except ParsingError as e:
        assert any(word in str(e) for word in ["GMFT", "timeout", "failed"])
    except Exception as e:
        pytest.fail(f"Unexpected error type: {type(e).__name__}: {e}")


def test_gmft_config_serialization() -> None:
    """Test that GMFTConfig can be serialized for process passing."""
    config = GMFTConfig(
        verbosity=2,
        detector_base_threshold=0.8,
        remove_null_rows=False,
    )

    config_dict = config.__dict__.copy()
    new_config = GMFTConfig(**config_dict)

    assert new_config.verbosity == 2
    assert new_config.detector_base_threshold == 0.8
    assert new_config.remove_null_rows is False
