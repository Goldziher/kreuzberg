"""Tests for GMFT table extraction functionality."""

from __future__ import annotations

import pytest
from PIL import Image

from kreuzberg._gmft import (
    CroppedTable,
    TableDetector,
    TableFormatter,
    _get_cached_detector,
    _get_cached_formatter,
    extract_tables_async,
    extract_tables_sync,
)
from kreuzberg._gmft._algorithm import (
    _apply_non_maximum_suppression,
    _calculate_cell_intersection,
    _extract_cell_text,
    _filter_predictions_cached,
    _merge_close_predictions,
    extract_table_dataframe,
)
from kreuzberg._gmft._base import Rect
from kreuzberg._gmft._types import (
    BboxPredictions,
    FormattedTable,
    TablePredictions,
)
from kreuzberg._types import GMFTConfig
from kreuzberg.exceptions import MissingDependencyError

# Configuration Tests


def test_gmft_config_defaults() -> None:
    config = GMFTConfig()
    assert config.detection_threshold == 0.7
    assert config.structure_threshold == 0.5
    assert config.detection_device == "auto"
    assert config.structure_device == "auto"
    assert config.detection_model == "microsoft/table-transformer-detection"
    assert config.structure_model == "microsoft/table-transformer-structure-recognition-v1.1-all"


def test_gmft_config_custom() -> None:
    config = GMFTConfig(
        detection_threshold=0.8,
        structure_threshold=0.6,
        model_cache_dir="/tmp/models",
        detection_device="cpu",
        structure_device="cuda",
    )
    assert config.detection_threshold == 0.8
    assert config.structure_threshold == 0.6
    assert config.model_cache_dir == "/tmp/models"
    assert config.detection_device == "cpu"
    assert config.structure_device == "cuda"


def test_gmft_config_model_variants() -> None:
    # Test v1.1-pub variant
    config = GMFTConfig(structure_model="microsoft/table-transformer-structure-recognition-v1.1-pub")
    assert "v1.1-pub" in config.structure_model

    # Test v1.1-fin variant
    config = GMFTConfig(structure_model="microsoft/table-transformer-structure-recognition-v1.1-fin")
    assert "v1.1-fin" in config.structure_model


# Detector and Formatter Tests


def test_table_detector_initialization() -> None:
    detector = TableDetector()
    assert detector.config is not None
    assert detector.is_available() is False  # No transformers installed


def test_table_formatter_initialization() -> None:
    formatter = TableFormatter()
    assert formatter.config is not None
    assert formatter.is_available() is False  # No transformers installed


def test_detector_with_custom_config() -> None:
    config = GMFTConfig(detection_device="cpu", detection_threshold=0.9)
    detector = TableDetector(config)
    assert detector.config == config
    assert detector._device == "cpu"


def test_formatter_with_custom_config() -> None:
    config = GMFTConfig(structure_device="cpu", structure_threshold=0.3)
    formatter = TableFormatter(config)
    assert formatter.config == config
    assert formatter._device == "cpu"


# Caching Tests


def test_cached_detector_instances() -> None:
    detector1 = _get_cached_detector("microsoft/table-transformer-detection", 0.7)
    detector2 = _get_cached_detector("microsoft/table-transformer-detection", 0.7)
    detector3 = _get_cached_detector("microsoft/table-transformer-detection", 0.8)

    # Same config should return same instance
    assert detector1 is detector2
    # Different config should return different instance
    assert detector1 is not detector3


def test_cached_formatter_instances() -> None:
    formatter1 = _get_cached_formatter("microsoft/table-transformer-structure-recognition-v1.1-all", 0.5)
    formatter2 = _get_cached_formatter("microsoft/table-transformer-structure-recognition-v1.1-all", 0.5)
    formatter3 = _get_cached_formatter("microsoft/table-transformer-structure-recognition-v1.1-pub", 0.5)

    # Same config should return same instance
    assert formatter1 is formatter2
    # Different config should return different instance
    assert formatter1 is not formatter3


# Type Tests


def test_cropped_table_creation() -> None:
    table = CroppedTable(
        rect=Rect((100, 200, 300, 400)),
        confidence_score=0.95,
        page_number=1,
        label="table",
    )
    assert table.rect.xmin == 100
    assert table.rect.ymin == 200
    assert table.rect.xmax == 300
    assert table.rect.ymax == 400
    assert table.confidence_score == 0.95


def test_formatted_table_creation() -> None:
    import polars as pl

    cropped = CroppedTable(
        rect=Rect((0, 0, 100, 100)),
        confidence_score=0.9,
        page_number=0,
        label="table",
    )

    predictions = TablePredictions(
        rows=BboxPredictions(boxes=(), scores=(), labels=()),
        columns=BboxPredictions(boxes=(), scores=(), labels=()),
        spanning_cells=BboxPredictions(boxes=(), scores=(), labels=()),
    )

    formatted = FormattedTable(
        cropped_table=cropped,
        dataframe=pl.DataFrame(),
        predictions=predictions,
        confidence_scores={"overall": 0.9},
        metadata={},
    )

    assert formatted.cropped_table == cropped
    assert isinstance(formatted.dataframe, pl.DataFrame)


def test_bbox_predictions_hashable() -> None:
    pred1 = BboxPredictions(
        boxes=((0, 0, 10, 10),),
        scores=(0.9,),
        labels=(1,),
    )
    pred2 = BboxPredictions(
        boxes=((0, 0, 10, 10),),
        scores=(0.9,),
        labels=(1,),
    )

    # Should be hashable and equal
    assert hash(pred1) == hash(pred2)
    assert pred1 == pred2

    # Can be used in sets
    predictions_set = {pred1, pred2}
    assert len(predictions_set) == 1


def test_table_predictions_hashable() -> None:
    empty_pred = BboxPredictions(boxes=(), scores=(), labels=())

    table_pred = TablePredictions(
        rows=empty_pred,
        columns=empty_pred,
        spanning_cells=empty_pred,
    )

    # Should be hashable
    assert hash(table_pred) is not None

    # Can be used as dict key
    cache = {table_pred: "cached_result"}
    assert cache[table_pred] == "cached_result"


# Algorithm Tests


def test_extract_cell_text() -> None:
    # Create a simple test image
    image = Image.new("RGB", (100, 100), color="white")

    # Extract text from a cell region (placeholder implementation returns dimensions)
    text = _extract_cell_text(image, (10, 10, 50, 50))
    assert text == "[40x40]"  # Based on current placeholder implementation

    # Invalid bounding box
    text = _extract_cell_text(image, (50, 50, 10, 10))  # Invalid: right < left
    assert text == ""


def test_filter_predictions_cached() -> None:
    predictions = BboxPredictions(
        boxes=((0, 0, 10, 10), (20, 20, 30, 30)),
        scores=(0.9, 0.3),
        labels=(1, 2),
    )

    # Filter with threshold 0.5
    filtered = _filter_predictions_cached(predictions, 0.5)
    assert len(filtered.boxes) == 1
    assert filtered.scores[0] == 0.9

    # Filter with threshold 0.2
    filtered = _filter_predictions_cached(predictions, 0.2)
    assert len(filtered.boxes) == 2


def test_calculate_cell_intersection() -> None:
    # Test intersection calculation
    row_box = (0, 0, 100, 50)
    col_box = (25, 0, 75, 100)

    # Calculate intersection (cached function)
    intersection = _calculate_cell_intersection(row_box, col_box)
    assert 0 <= intersection <= 1

    # No intersection
    row_box = (0, 0, 10, 10)
    col_box = (100, 100, 110, 110)
    intersection = _calculate_cell_intersection(row_box, col_box)
    assert intersection == 0


def test_apply_non_maximum_suppression() -> None:
    boxes = [
        (0.0, 0.0, 100.0, 100.0),
        (10.0, 10.0, 90.0, 90.0),  # Highly overlapping with first
        (200.0, 200.0, 300.0, 300.0),  # Non-overlapping
    ]
    scores = [0.9, 0.8, 0.7]

    kept_indices = _apply_non_maximum_suppression(boxes, scores, threshold=0.5)

    # Should keep first (highest score) and third (non-overlapping)
    assert 0 in kept_indices
    assert 2 in kept_indices
    assert 1 not in kept_indices  # Suppressed due to overlap


def test_merge_close_predictions() -> None:
    boxes = [
        (0.0, 0.0, 100.0, 100.0),
        (95.0, 95.0, 200.0, 200.0),  # Close to first
        (500.0, 500.0, 600.0, 600.0),  # Far from others
    ]
    scores = [0.9, 0.8, 0.7]
    labels = [1, 1, 2]

    merged_boxes, _merged_scores, _merged_labels = _merge_close_predictions(boxes, scores, labels, merge_threshold=0.1)

    # With low threshold, should not merge anything
    assert len(merged_boxes) == 3


def test_extract_table_dataframe() -> None:
    """Test the main table extraction algorithm."""
    # Create a test image
    image = Image.new("RGB", (200, 200), color="white")

    # Create minimal predictions
    predictions = TablePredictions(
        rows=BboxPredictions(
            boxes=((0, 0, 200, 50), (0, 50, 200, 100)),
            scores=(0.9, 0.9),
            labels=(2, 2),
        ),
        columns=BboxPredictions(
            boxes=((0, 0, 100, 100), (100, 0, 200, 100)),
            scores=(0.9, 0.9),
            labels=(1, 1),
        ),
        spanning_cells=BboxPredictions(boxes=(), scores=(), labels=()),
    )

    config = GMFTConfig()

    # Extract dataframe
    df = extract_table_dataframe(image, predictions, config)

    # Should create a 2x2 grid
    assert df.shape == (2, 2)  # 2 rows, 2 columns


# Integration Tests


def test_extract_tables_sync_missing_deps() -> None:
    from pathlib import Path

    test_pdf = Path("tests/test_source_files/gmft/tiny.pdf")

    # Should raise MissingDependencyError when transformers not installed
    with pytest.raises(MissingDependencyError):
        extract_tables_sync(test_pdf)


@pytest.mark.anyio
async def test_extract_tables_async_missing_deps() -> None:
    from pathlib import Path

    test_pdf = Path("tests/test_source_files/gmft/tiny.pdf")

    # Should raise MissingDependencyError when transformers not installed
    with pytest.raises(MissingDependencyError):
        await extract_tables_async(test_pdf)


def test_rect_operations() -> None:
    rect = Rect((10, 20, 30, 40))
    assert rect.xmin == 10
    assert rect.ymin == 20
    assert rect.xmax == 30
    assert rect.ymax == 40
    assert rect.width == 20
    assert rect.height == 20
    assert rect.area == 400
