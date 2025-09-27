"""Tests to validate GMFT configuration defaults and settings."""

from __future__ import annotations

import pytest

from kreuzberg._types import GMFTConfig


def test_default_thresholds_are_optimal() -> None:
    """Verify default thresholds match Microsoft recommendations."""
    config = GMFTConfig()

    # Microsoft recommends 0.7 for detection (balanced precision/recall)
    assert config.detection_threshold == 0.7

    # Structure threshold should be lower to capture more details
    assert config.structure_threshold == 0.5
    assert config.structure_threshold < config.detection_threshold


def test_default_models_are_latest() -> None:
    """Verify default models use the latest TATR v1.1 versions."""
    config = GMFTConfig()

    # Detection model
    assert "microsoft/table-transformer-detection" in config.detection_model

    # Structure model should use v1.1-all (best overall performance)
    assert "v1.1-all" in config.structure_model
    assert "microsoft/table-transformer-structure-recognition" in config.structure_model


def test_device_auto_configuration() -> None:
    """Test that device defaults to 'auto' for flexibility."""
    config = GMFTConfig()

    assert config.detection_device == "auto"
    assert config.structure_device == "auto"


def test_cell_confidence_thresholds() -> None:
    """Test cell confidence thresholds are reasonable."""
    config = GMFTConfig()

    # All cell thresholds should be relatively low for better recall
    assert config.cell_confidence_table == 0.3
    assert config.cell_confidence_column == 0.3
    assert config.cell_confidence_row == 0.3
    assert config.cell_confidence_column_header == 0.3


def test_table_size_constraints() -> None:
    """Test table size constraints are reasonable."""
    config = GMFTConfig()

    # Minimum area to filter out noise
    assert config.min_table_area == 1000

    # No maximum by default (None)
    assert config.max_table_area is None

    # Reasonable padding around tables
    assert config.crop_padding == 20


def test_processing_flags() -> None:
    """Test processing flags have sensible defaults."""
    config = GMFTConfig()

    # Should remove null rows by default for cleaner output
    assert config.remove_null_rows is True

    # Multi-header detection should be off by default (simpler)
    assert config.enable_multi_header is False

    # Semantic spanning cells off by default (simpler)
    assert config.semantic_spanning_cells is False


def test_performance_settings() -> None:
    """Test performance-related settings."""
    config = GMFTConfig()

    # Model caching should be enabled by default
    assert config.enable_model_caching is True

    # Batch size of 1 for simplicity (can be increased for performance)
    assert config.batch_size == 1

    # Mixed precision off by default (compatibility)
    assert config.mixed_precision is False


def test_verbosity_default() -> None:
    """Test verbosity has a reasonable default."""
    config = GMFTConfig()

    # Moderate verbosity by default
    assert config.verbosity == 1


def test_config_immutability() -> None:
    """Test that GMFTConfig is immutable (frozen)."""
    config = GMFTConfig()

    # Should raise error when trying to modify
    with pytest.raises((AttributeError, TypeError)):
        config.detection_threshold = 0.9


def test_config_hashability() -> None:
    """Test that GMFTConfig is hashable for caching."""
    config1 = GMFTConfig(detection_threshold=0.7)
    config2 = GMFTConfig(detection_threshold=0.7)
    config3 = GMFTConfig(detection_threshold=0.8)

    # Same configs should have same hash
    assert hash(config1) == hash(config2)

    # Different configs should have different hash
    assert hash(config1) != hash(config3)

    # Should be usable as dict key
    cache = {config1: "result"}
    assert cache[config2] == "result"  # Same config retrieves same result


def test_model_variant_configurations() -> None:
    """Test different model variant configurations."""
    # Test pub variant for published/academic papers
    config_pub = GMFTConfig(structure_model="microsoft/table-transformer-structure-recognition-v1.1-pub")
    assert "v1.1-pub" in config_pub.structure_model

    # Test fin variant for financial documents
    config_fin = GMFTConfig(structure_model="microsoft/table-transformer-structure-recognition-v1.1-fin")
    assert "v1.1-fin" in config_fin.structure_model


def test_threshold_ranges() -> None:
    """Test that thresholds accept valid ranges."""
    # Should accept any value between 0 and 1
    config = GMFTConfig(
        detection_threshold=0.1,
        structure_threshold=0.95,
    )
    assert config.detection_threshold == 0.1
    assert config.structure_threshold == 0.95


def test_cache_directory_configuration() -> None:
    """Test model cache directory configuration."""
    # Default should be None (use HuggingFace default)
    config = GMFTConfig()
    assert config.model_cache_dir is None

    # Should accept custom paths
    config = GMFTConfig(model_cache_dir="/custom/cache/path")
    assert config.model_cache_dir == "/custom/cache/path"
