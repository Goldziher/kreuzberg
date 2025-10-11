from __future__ import annotations

import pytest

from kreuzberg._types import TableExtractionConfig


def test_default_thresholds_are_optimal() -> None:
    config = TableExtractionConfig()

    assert config.detection_threshold == 0.7

    assert config.structure_threshold == 0.5
    assert config.structure_threshold < config.detection_threshold


def test_default_models_are_latest() -> None:
    config = TableExtractionConfig()

    assert "microsoft/table-transformer-detection" in config.detection_model

    assert "v1.1-all" in config.structure_model
    assert "microsoft/table-transformer-structure-recognition" in config.structure_model


def test_device_auto_configuration() -> None:
    config = TableExtractionConfig()

    assert config.detection_device == "auto"
    assert config.structure_device == "auto"


def test_cell_confidence_thresholds() -> None:
    config = TableExtractionConfig()

    assert config.cell_confidence_table == 0.3
    assert config.cell_confidence_column == 0.3
    assert config.cell_confidence_row == 0.3
    assert config.cell_confidence_column_header == 0.3


def test_table_size_constraints() -> None:
    config = TableExtractionConfig()

    assert config.min_table_area == 1000

    assert config.max_table_area is None

    assert config.crop_padding == 20


def test_processing_flags() -> None:
    config = TableExtractionConfig()

    assert config.remove_null_rows is True

    assert config.enable_multi_header is False

    assert config.semantic_spanning_cells is False


def test_performance_settings() -> None:
    config = TableExtractionConfig()

    assert config.enable_model_caching is True

    assert config.batch_size == 1

    assert config.mixed_precision is False


def test_verbosity_default() -> None:
    config = TableExtractionConfig()

    assert config.verbosity == 1


def test_config_immutability() -> None:
    config = TableExtractionConfig()

    with pytest.raises((AttributeError, TypeError)):
        config.detection_threshold = 0.9  # type: ignore[misc]


def test_config_hashability() -> None:
    config1 = TableExtractionConfig(detection_threshold=0.7)
    config2 = TableExtractionConfig(detection_threshold=0.7)
    config3 = TableExtractionConfig(detection_threshold=0.8)

    assert hash(config1) == hash(config2)

    assert hash(config1) != hash(config3)

    cache = {config1: "result"}
    assert cache[config2] == "result"


def test_model_variant_configurations() -> None:
    config_pub = TableExtractionConfig(structure_model="microsoft/table-transformer-structure-recognition-v1.1-pub")
    assert "v1.1-pub" in config_pub.structure_model

    config_fin = TableExtractionConfig(structure_model="microsoft/table-transformer-structure-recognition-v1.1-fin")
    assert "v1.1-fin" in config_fin.structure_model


def test_threshold_ranges() -> None:
    config = TableExtractionConfig(
        detection_threshold=0.1,
        structure_threshold=0.95,
    )
    assert config.detection_threshold == 0.1
    assert config.structure_threshold == 0.95


def test_cache_directory_configuration() -> None:
    config = TableExtractionConfig()
    assert config.model_cache_dir is None

    config = TableExtractionConfig(model_cache_dir="/custom/cache/path")
    assert config.model_cache_dir == "/custom/cache/path"
