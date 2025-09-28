from __future__ import annotations

from pathlib import Path

import pytest

from kreuzberg._gmft import extract_tables_async, extract_tables_sync
from kreuzberg._types import GMFTConfig
from kreuzberg.exceptions import MissingDependencyError


def test_extract_tables_from_tiny_pdf() -> None:
    pdf_path = Path("test_documents/gmft/tiny.pdf")

    try:
        tables = extract_tables_sync(pdf_path)

        assert isinstance(tables, list)

        for table in tables:
            assert "cropped_image" in table
            assert "df" in table
            assert "page_number" in table
            assert "text" in table
            assert table["page_number"] > 0

    except MissingDependencyError:
        pass


def test_custom_detection_threshold() -> None:
    pdf_path = Path("test_documents/gmft/tiny.pdf")

    config_high = GMFTConfig(detection_threshold=0.9)

    try:
        tables_high = extract_tables_sync(pdf_path, config=config_high)

        config_low = GMFTConfig(detection_threshold=0.3)
        tables_low = extract_tables_sync(pdf_path, config=config_low)

        assert len(tables_low) >= len(tables_high)

    except MissingDependencyError:
        pass


def test_custom_structure_threshold() -> None:
    pdf_path = Path("test_documents/gmft/tiny.pdf")

    config = GMFTConfig(
        detection_threshold=0.5,
        structure_threshold=0.3,
    )

    try:
        tables = extract_tables_sync(pdf_path, config=config)

        for table in tables:
            df = table["df"]
            assert df is not None

            assert table["text"]

    except MissingDependencyError:
        pass


def test_model_variants() -> None:
    pdf_path = Path("test_documents/gmft/tiny.pdf")

    config_all = GMFTConfig(structure_model="microsoft/table-transformer-structure-recognition-v1.1-all")

    config_pub = GMFTConfig(structure_model="microsoft/table-transformer-structure-recognition-v1.1-pub")

    try:
        tables_all = extract_tables_sync(pdf_path, config=config_all)
        tables_pub = extract_tables_sync(pdf_path, config=config_pub)

        assert isinstance(tables_all, list)
        assert isinstance(tables_pub, list)

    except MissingDependencyError:
        pass


def test_device_configuration() -> None:
    pdf_path = Path("test_documents/gmft/tiny.pdf")

    config = GMFTConfig(detection_device="cpu", structure_device="cpu")

    try:
        tables = extract_tables_sync(pdf_path, config=config)
        assert isinstance(tables, list)

    except MissingDependencyError:
        pass


@pytest.mark.anyio
async def test_async_extraction() -> None:
    pdf_path = Path("test_documents/gmft/tiny.pdf")

    try:
        tables = await extract_tables_async(pdf_path)

        assert isinstance(tables, list)
        for table in tables:
            assert "cropped_image" in table
            assert "df" in table

    except MissingDependencyError:
        pass


def test_empty_pdf_handling() -> None:
    pdf_path = Path("test_documents/searchable.pdf")

    try:
        tables = extract_tables_sync(pdf_path)

        assert isinstance(tables, list)

    except MissingDependencyError:
        pass


def test_multipage_pdf() -> None:
    pdf_path = Path("test_documents/gmft/tatr.pdf")

    try:
        tables = extract_tables_sync(pdf_path)

        if tables:
            page_numbers = {table["page_number"] for table in tables}
            assert all(p > 0 for p in page_numbers)

    except MissingDependencyError:
        pass
    except FileNotFoundError:
        pass
