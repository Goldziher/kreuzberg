from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING

import pytest

from kreuzberg._types import TableExtractionConfig
from kreuzberg._vision_tables import (
    _get_cached_detector,
    _get_cached_formatter,
    clear_table_extraction_caches,
    extract_tables_async,
    extract_tables_sync,
)
from kreuzberg.exceptions import MissingDependencyError

if TYPE_CHECKING:
    from collections.abc import Generator

    import pytest_mock


@pytest.fixture(autouse=True)
def reset_table_extraction_caches() -> Generator[None, None, None]:
    clear_table_extraction_caches()
    yield
    clear_table_extraction_caches()


def test_extract_tables_sync_with_path_object(mocker: pytest_mock.MockerFixture) -> None:
    mocker.patch("kreuzberg._vision_tables._detector._import_transformers", return_value=(None, None))

    test_pdf = Path("test_documents/gmft/tiny.pdf")

    with pytest.raises(MissingDependencyError) as exc_info:
        extract_tables_sync(test_pdf)

    assert "transformers" in str(exc_info.value) or "torch" in str(exc_info.value)


@pytest.mark.anyio
async def test_extract_tables_async_with_path_object(mocker: pytest_mock.MockerFixture) -> None:
    mocker.patch("kreuzberg._vision_tables._detector._import_transformers", return_value=(None, None))

    test_pdf = Path("test_documents/gmft/tiny.pdf")

    with pytest.raises(MissingDependencyError) as exc_info:
        await extract_tables_async(test_pdf)

    assert "transformers" in str(exc_info.value) or "torch" in str(exc_info.value)


def test_extract_tables_sync_with_default_config(mocker: pytest_mock.MockerFixture) -> None:
    mocker.patch("kreuzberg._vision_tables._detector._import_transformers", return_value=(None, None))

    test_pdf = "test_documents/gmft/tiny.pdf"

    with pytest.raises(MissingDependencyError) as exc_info:
        extract_tables_sync(test_pdf, config=None)

    assert "transformers" in str(exc_info.value) or "torch" in str(exc_info.value)


def test_cached_detector_different_configs() -> None:
    detector1 = _get_cached_detector("microsoft/table-transformer-detection", 0.7)
    detector2 = _get_cached_detector("microsoft/table-transformer-detection", 0.8)
    detector3 = _get_cached_detector("microsoft/table-transformer-detection", 0.7)

    assert detector1 is not detector2
    assert detector1 is detector3


def test_cached_formatter_different_configs() -> None:
    formatter1 = _get_cached_formatter("microsoft/table-transformer-structure-recognition-v1.1-all", 0.5)
    formatter2 = _get_cached_formatter("microsoft/table-transformer-structure-recognition-v1.1-pub", 0.5)
    formatter3 = _get_cached_formatter("microsoft/table-transformer-structure-recognition-v1.1-all", 0.5)

    assert formatter1 is not formatter2
    assert formatter1 is formatter3


def test_gmft_config_custom_thresholds() -> None:
    config = TableExtractionConfig(detection_threshold=0.85, structure_threshold=0.25, verbosity=3)

    assert config.detection_threshold == 0.85
    assert config.structure_threshold == 0.25
    assert config.verbosity == 3


def test_config_hash_generation() -> None:
    config1 = TableExtractionConfig(detection_threshold=0.9, structure_threshold=0.3)
    config2 = TableExtractionConfig(detection_threshold=0.8, structure_threshold=0.4)

    hash1 = f"{config1.detection_threshold}_{config1.structure_threshold}"
    hash2 = f"{config2.detection_threshold}_{config2.structure_threshold}"

    assert hash1 != hash2
    assert hash1 == "0.9_0.3"
    assert hash2 == "0.8_0.4"


def test_detector_model_configuration() -> None:
    config = TableExtractionConfig(
        detection_model="custom/detector",
        detection_device="cuda",
        detection_threshold=0.95,
        model_cache_dir="/custom/cache",
    )

    assert config.detection_model == "custom/detector"
    assert config.detection_device == "cuda"
    assert config.detection_threshold == 0.95
    assert config.model_cache_dir == "/custom/cache"


def test_structure_model_configuration() -> None:
    config = TableExtractionConfig(
        structure_model="custom/formatter",
        structure_device="cuda",
        structure_threshold=0.2,
        verbosity=0,
        cell_confidence_table=0.1,
        remove_null_rows=False,
        enable_multi_header=True,
        semantic_spanning_cells=True,
    )

    assert config.structure_model == "custom/formatter"
    assert config.structure_device == "cuda"
    assert config.structure_threshold == 0.2
    assert config.verbosity == 0
    assert config.cell_confidence_table == 0.1
    assert config.remove_null_rows is False
    assert config.enable_multi_header is True
    assert config.semantic_spanning_cells is True


def test_extract_tables_sync_file_not_found() -> None:
    with pytest.raises(FileNotFoundError):
        extract_tables_sync("definitely_does_not_exist.pdf")


@pytest.mark.anyio
async def test_extract_tables_async_file_not_found() -> None:
    with pytest.raises(FileNotFoundError):
        await extract_tables_async("definitely_does_not_exist.pdf")
