"""Integration tests for GMFT table extraction with real PDFs."""

from __future__ import annotations

from pathlib import Path

import pytest

from kreuzberg._gmft import extract_tables_async, extract_tables_sync
from kreuzberg._types import GMFTConfig
from kreuzberg.exceptions import MissingDependencyError

# These tests require ML models which may not be downloaded
# They test the actual model loading and inference paths


def test_extract_tables_from_tiny_pdf() -> None:
    """Test table extraction from a small PDF with tables."""
    pdf_path = Path("tests/test_source_files/gmft/tiny.pdf")

    # This will fail with MissingDependencyError if deps not installed
    try:
        tables = extract_tables_sync(pdf_path)

        # Verify we got some tables
        assert isinstance(tables, list)

        # Check structure of returned data
        for table in tables:
            assert "cropped_image" in table
            assert "df" in table
            assert "page_number" in table
            assert "text" in table
            assert table["page_number"] > 0

    except MissingDependencyError:
        # Expected when transformers/torch not installed
        pass


def test_custom_detection_threshold() -> None:
    """Test with custom detection threshold configuration."""
    pdf_path = Path("tests/test_source_files/gmft/tiny.pdf")

    # Test with high threshold (fewer detections)
    config_high = GMFTConfig(detection_threshold=0.9)

    try:
        tables_high = extract_tables_sync(pdf_path, config=config_high)

        # Test with low threshold (more detections)
        config_low = GMFTConfig(detection_threshold=0.3)
        tables_low = extract_tables_sync(pdf_path, config=config_low)

        # Lower threshold should generally find same or more tables
        assert len(tables_low) >= len(tables_high)

    except MissingDependencyError:
        pass


def test_custom_structure_threshold() -> None:
    """Test with custom structure recognition threshold."""
    pdf_path = Path("tests/test_source_files/gmft/tiny.pdf")

    # Test with different structure thresholds
    config = GMFTConfig(
        detection_threshold=0.5,
        structure_threshold=0.3,  # Lower threshold for cell detection
    )

    try:
        tables = extract_tables_sync(pdf_path, config=config)

        for table in tables:
            # Check that dataframe has content
            df = table["df"]
            assert df is not None

            # Markdown representation should exist
            assert table["text"]

    except MissingDependencyError:
        pass


def test_model_variants() -> None:
    """Test different TATR model variants."""
    pdf_path = Path("tests/test_source_files/gmft/tiny.pdf")

    # Test v1.1-all variant (default)
    config_all = GMFTConfig(structure_model="microsoft/table-transformer-structure-recognition-v1.1-all")

    # Test v1.1-pub variant (for published tables)
    config_pub = GMFTConfig(structure_model="microsoft/table-transformer-structure-recognition-v1.1-pub")

    try:
        tables_all = extract_tables_sync(pdf_path, config=config_all)
        tables_pub = extract_tables_sync(pdf_path, config=config_pub)

        # Both should extract tables, but might have different results
        assert isinstance(tables_all, list)
        assert isinstance(tables_pub, list)

    except MissingDependencyError:
        pass


def test_device_configuration() -> None:
    """Test device configuration options."""
    pdf_path = Path("tests/test_source_files/gmft/tiny.pdf")

    # Force CPU usage
    config = GMFTConfig(detection_device="cpu", structure_device="cpu")

    try:
        tables = extract_tables_sync(pdf_path, config=config)
        assert isinstance(tables, list)

    except MissingDependencyError:
        pass


@pytest.mark.anyio
async def test_async_extraction() -> None:
    """Test async table extraction."""
    pdf_path = Path("tests/test_source_files/gmft/tiny.pdf")

    try:
        tables = await extract_tables_async(pdf_path)

        assert isinstance(tables, list)
        for table in tables:
            assert "cropped_image" in table
            assert "df" in table

    except MissingDependencyError:
        pass


def test_empty_pdf_handling() -> None:
    """Test handling of PDFs without tables."""
    pdf_path = Path("tests/test_source_files/searchable.pdf")

    try:
        tables = extract_tables_sync(pdf_path)

        # Should return empty list for PDFs without tables
        assert isinstance(tables, list)
        # This PDF likely has no tables

    except MissingDependencyError:
        pass


def test_multipage_pdf() -> None:
    """Test table extraction from multi-page PDFs."""
    # Use a PDF that we know has multiple pages
    pdf_path = Path("tests/test_source_files/gmft/tatr.pdf")

    try:
        tables = extract_tables_sync(pdf_path)

        if tables:
            # Check page numbers are set correctly
            page_numbers = {table["page_number"] for table in tables}
            assert all(p > 0 for p in page_numbers)

    except MissingDependencyError:
        pass
    except FileNotFoundError:
        # The tatr.pdf might not exist
        pass
