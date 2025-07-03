"""Extended tests for GMFT functionality."""

from __future__ import annotations

from dataclasses import replace
from typing import TYPE_CHECKING
from unittest.mock import patch

import pytest
import anyio

from kreuzberg._gmft import (
    GMFTConfig,
    extract_tables,
)
from kreuzberg.exceptions import MissingDependencyError

if TYPE_CHECKING:
    from pathlib import Path


def test_gmft_config_defaults() -> None:
    """Test GMFTConfig default values."""
    config = GMFTConfig()

    assert config.verbosity == 0
    assert config.formatter_base_threshold == 0.3
    assert config.detector_base_threshold == 0.9
    assert config.remove_null_rows is True
    # Note: apply_formatter_for_format_only and visualize are not config options
    # They were incorrectly included in the test

    assert config.cell_required_confidence[0] == 0.3
    assert config.cell_required_confidence[4] == 0.5
    assert config.cell_required_confidence[6] == 99


def test_gmft_config_custom() -> None:
    """Test GMFTConfig with custom values."""
    config = GMFTConfig(
        verbosity=2,
        formatter_base_threshold=0.5,
        remove_null_rows=False,
    )

    assert config.verbosity == 2
    assert config.formatter_base_threshold == 0.5
    assert config.remove_null_rows is False
    # visualize is not a config option


def test_gmft_config_replace() -> None:
    """Test replacing GMFTConfig values."""
    config = GMFTConfig()
    new_config = replace(config, verbosity=3)

    assert config.verbosity == 0
    assert new_config.verbosity == 3
    # visualize is not a config option


def test_gmft_config_hash() -> None:
    """Test GMFTConfig is hashable."""
    config1 = GMFTConfig(verbosity=1)
    config2 = GMFTConfig(verbosity=1)
    config3 = GMFTConfig(verbosity=2)

    assert hash(config1) == hash(config2)

    assert hash(config1) != hash(config3)

    config_set = {config1, config2, config3}
    assert len(config_set) == 2


def test_extract_tables_sync_missing_dependency(tiny_pdf_with_tables: Path) -> None:
    """Test extract_tables_sync when gmft is not installed."""
    import os

    if os.getenv("KREUZBERG_GMFT_ISOLATED", "true").lower() == "true":
        pytest.skip("Cannot test missing dependency with isolated process")

    # Since extract_tables_sync now uses isolated process by default,
    # we need to disable it to test the import error path

    with (
        patch.dict("sys.modules", {"gmft": None, "gmft.auto": None}),
        patch.dict(os.environ, {"KREUZBERG_GMFT_ISOLATED": "false"}),
    ):
        with pytest.raises(MissingDependencyError) as exc_info:
            extract_tables_sync(tiny_pdf_with_tables)

        assert "gmft" in str(exc_info.value)
        assert "table extraction" in str(exc_info.value)


def test_extract_tables_sync_success(tiny_pdf_with_tables: Path) -> None:
    """Test successful table extraction."""
    try:
        results = anyio.run(extract_tables, tiny_pdf_with_tables)
        assert isinstance(results, list)
        if results:
            assert all(isinstance(table, dict) for table in results)
            assert all("page_number" in table for table in results)
    except MissingDependencyError:
        pytest.skip("GMFT dependency not installed")


def test_extract_tables_sync_custom_config(tiny_pdf_with_tables: Path) -> None:
    """Test table extraction with custom config."""
    config = GMFTConfig(
        verbosity=2,
        detector_base_threshold=0.8,
        remove_null_rows=False,
    )
    try:
        results = anyio.run(extract_tables, tiny_pdf_with_tables, config)
        assert isinstance(results, list)
    except MissingDependencyError:
        pytest.skip("GMFT dependency not installed")


def test_extract_tables_sync_multiple_tables(tiny_pdf_with_tables: Path) -> None:
    """Test extraction of multiple tables."""
    try:
        results = anyio.run(extract_tables, tiny_pdf_with_tables)
        assert isinstance(results, list)
    except MissingDependencyError:
        pytest.skip("GMFT dependency not installed")


def test_extract_tables_sync_no_tables(tmp_path: Path) -> None:
    """Test extraction when no tables are found."""
    no_tables_pdf = tmp_path / "no_tables.pdf"
    import pypdfium2 as pdfium
    pdf = pdfium.PdfDocument.new()
    pdf.new_page(200, 200)
    pdf.save(no_tables_pdf)
    pdf.close()
    try:
        results = anyio.run(extract_tables, no_tables_pdf)
        assert results == []
    except MissingDependencyError:
        pytest.skip("GMFT dependency not installed")
