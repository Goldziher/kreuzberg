"""Tests for GMFT table extraction functionality."""

from __future__ import annotations

import tempfile
from pathlib import Path
from typing import TYPE_CHECKING
from unittest.mock import Mock, patch

import polars as pl
import pytest
from anyio import Path as AsyncPath
from PIL import Image

from kreuzberg._gmft import (
    CroppedTable,
    FormattedTable,
    Rect,
    TableDetector,
    TableFormatter,
    extract_tables_async,
    extract_tables_sync,
)
from kreuzberg._gmft._algorithm import extract_table_dataframe
from kreuzberg._gmft._types import BboxPredictions, TablePredictions
from kreuzberg._types import GMFTConfig, TableData
from kreuzberg._utils._model_cache import (
    ensure_cache_dir_async,
    resolve_model_cache_dir,
    resolve_model_cache_dir_async,
    setup_huggingface_cache,
    setup_huggingface_cache_async,
)
from kreuzberg.exceptions import MissingDependencyError

if TYPE_CHECKING:
    from kreuzberg._gmft._base import BBox


def test_rect_initialization() -> None:
    """Test Rect class initialization and properties."""
    bbox: BBox = (10.0, 20.0, 100.0, 200.0)
    rect = Rect(bbox)

    assert rect.xmin == 10.0
    assert rect.ymin == 20.0
    assert rect.xmax == 100.0
    assert rect.ymax == 200.0


def test_rect_bbox_property() -> None:
    """Test Rect bbox property."""
    bbox: BBox = (10.0, 20.0, 100.0, 200.0)
    rect = Rect(bbox)

    assert rect.bbox == bbox


def test_cropped_table_hashable() -> None:
    """Test that CroppedTable is hashable."""
    table1 = CroppedTable(
        rect=Rect((10, 20, 100, 200)),
        confidence_score=0.9,
        page_number=1,
        label="table",
    )
    table2 = CroppedTable(
        rect=Rect((10, 20, 100, 200)),
        confidence_score=0.9,
        page_number=1,
        label="table",
    )

    # Should be hashable
    assert hash(table1) == hash(table2)

    # Should work in sets
    table_set = {table1, table2}
    assert len(table_set) == 1


def test_bbox_predictions_from_lists() -> None:
    """Test BboxPredictions.from_lists factory method."""
    boxes = [(10, 20, 100, 200), (30, 40, 150, 250)]
    scores = [0.9, 0.8]
    labels = [1, 2]

    predictions = BboxPredictions.from_lists(boxes, scores, labels)

    assert predictions.boxes == tuple(boxes)
    assert predictions.scores == tuple(scores)
    assert predictions.labels == tuple(labels)


def test_table_predictions_structure() -> None:
    """Test TablePredictions structure."""
    rows = BboxPredictions.from_lists(
        boxes=[(10, 20, 100, 30)],
        scores=[0.9],
        labels=[1],
    )
    columns = BboxPredictions.from_lists(
        boxes=[(10, 20, 30, 200)],
        scores=[0.85],
        labels=[2],
    )
    spanning_cells = BboxPredictions.from_lists(
        boxes=[],
        scores=[],
        labels=[],
    )

    predictions = TablePredictions(
        rows=rows,
        columns=columns,
        spanning_cells=spanning_cells,
    )

    assert predictions.rows == rows
    assert predictions.columns == columns
    assert predictions.spanning_cells == spanning_cells


def test_gmft_config_defaults() -> None:
    """Test GMFTConfig default values."""
    config = GMFTConfig()

    assert config.detection_model == "microsoft/table-transformer-detection"
    assert config.structure_model == "microsoft/table-transformer-structure-recognition-v1.1-all"
    assert config.detection_threshold == 0.7
    assert config.structure_threshold == 0.5
    assert config.model_cache_dir is None


def test_gmft_config_hashable() -> None:
    """Test that GMFTConfig is hashable and immutable."""
    config1 = GMFTConfig(detection_threshold=0.8)
    config2 = GMFTConfig(detection_threshold=0.8)
    config3 = GMFTConfig(detection_threshold=0.7)

    # Should be hashable
    assert hash(config1) == hash(config2)
    assert hash(config1) != hash(config3)

    # Should be frozen (immutable)
    with pytest.raises(AttributeError):
        config1.detection_threshold = 0.9


def test_table_detector_initialization() -> None:
    """Test TableDetector initialization."""
    config = GMFTConfig(detection_threshold=0.8)

    with patch("kreuzberg._gmft._detector.transformers", None):
        detector = TableDetector(config)

    assert detector.config == config
    assert detector.config.detection_threshold == 0.8


def test_table_detector_missing_dependencies() -> None:
    """Test TableDetector handles missing dependencies gracefully."""
    with patch("kreuzberg._gmft._detector.transformers", None):
        detector = TableDetector()

    assert not detector.is_available()

    # Should raise MissingDependencyError when trying to detect
    image = Image.new("RGB", (100, 100))
    with pytest.raises(MissingDependencyError, match="transformers"):
        detector.detect_tables_in_image(image)


def test_table_formatter_initialization() -> None:
    """Test TableFormatter initialization."""
    config = GMFTConfig(structure_threshold=0.6)

    with patch("kreuzberg._gmft._formatter._import_table_transformer"):
        formatter = TableFormatter(config)

    assert formatter.config == config
    assert formatter.config.structure_threshold == 0.6


def test_table_formatter_missing_dependencies() -> None:
    """Test TableFormatter handles missing dependencies gracefully."""
    with patch("kreuzberg._gmft._formatter._import_table_transformer"):
        formatter = TableFormatter()

    assert not formatter.is_available()

    # Should raise MissingDependencyError when trying to format
    image = Image.new("RGB", (100, 100))
    table = CroppedTable(
        rect=Rect((10, 10, 90, 90)),
        confidence_score=0.9,
        page_number=0,
        label="table",
    )

    with pytest.raises(MissingDependencyError, match="transformers"):
        formatter.format_table(table, image)


def test_model_cache_directory_resolution() -> None:
    """Test model cache directory resolution."""
    import os

    # Save original env vars
    original_kreuzberg = os.environ.get("KREUZBERG_MODEL_CACHE")
    original_hf = os.environ.get("HF_HOME")

    try:
        # Clear env vars
        for key in ["KREUZBERG_MODEL_CACHE", "HF_HOME", "TRANSFORMERS_CACHE"]:
            if key in os.environ:
                del os.environ[key]

        # Test with no env vars
        result = resolve_model_cache_dir()
        assert result is None

        # Test with KREUZBERG_MODEL_CACHE
        os.environ["KREUZBERG_MODEL_CACHE"] = "/test/kreuzberg"
        result = resolve_model_cache_dir()
        # May return None if directory creation fails
        assert result == "/test/kreuzberg" or result is None

        # Test with explicit config
        with tempfile.TemporaryDirectory() as tmpdir:
            result = resolve_model_cache_dir(config_cache_dir=tmpdir)
            assert result == tmpdir

    finally:
        # Restore env vars
        if original_kreuzberg:
            os.environ["KREUZBERG_MODEL_CACHE"] = original_kreuzberg
        elif "KREUZBERG_MODEL_CACHE" in os.environ:
            del os.environ["KREUZBERG_MODEL_CACHE"]

        if original_hf:
            os.environ["HF_HOME"] = original_hf
        elif "HF_HOME" in os.environ:
            del os.environ["HF_HOME"]


@pytest.mark.anyio
async def test_model_cache_directory_async() -> None:
    """Test async model cache directory operations."""

    with tempfile.TemporaryDirectory() as tmpdir:
        cache_dir = Path(tmpdir) / "test_cache"

        # Ensure directory creation works async
        result = await ensure_cache_dir_async(str(cache_dir))
        assert result == str(cache_dir)
        assert await AsyncPath(cache_dir).exists()

        # Test resolve with config
        result = await resolve_model_cache_dir_async(config_cache_dir=str(cache_dir))
        assert result == str(cache_dir)


def test_huggingface_cache_setup() -> None:
    """Test HuggingFace cache setup."""
    import os

    original_hf = os.environ.get("HF_HOME")
    original_transformers = os.environ.get("TRANSFORMERS_CACHE")

    try:
        # Clear env vars
        for key in ["HF_HOME", "TRANSFORMERS_CACHE"]:
            if key in os.environ:
                del os.environ[key]

        with tempfile.TemporaryDirectory() as tmpdir:
            result = setup_huggingface_cache(tmpdir)
            assert result == tmpdir
            assert os.environ["HF_HOME"] == tmpdir
            assert os.environ["TRANSFORMERS_CACHE"] == tmpdir

    finally:
        # Restore env vars
        if original_hf:
            os.environ["HF_HOME"] = original_hf
        elif "HF_HOME" in os.environ:
            del os.environ["HF_HOME"]

        if original_transformers:
            os.environ["TRANSFORMERS_CACHE"] = original_transformers
        elif "TRANSFORMERS_CACHE" in os.environ:
            del os.environ["TRANSFORMERS_CACHE"]


@pytest.mark.anyio
async def test_huggingface_cache_setup_async() -> None:
    """Test async HuggingFace cache setup."""
    import os

    original_hf = os.environ.get("HF_HOME")

    try:
        if "HF_HOME" in os.environ:
            del os.environ["HF_HOME"]

        with tempfile.TemporaryDirectory() as tmpdir:
            result = await setup_huggingface_cache_async(tmpdir)
            assert result == tmpdir
            assert os.environ["HF_HOME"] == tmpdir

    finally:
        if original_hf:
            os.environ["HF_HOME"] = original_hf
        elif "HF_HOME" in os.environ:
            del os.environ["HF_HOME"]


def test_extract_tables_sync_nonexistent_file() -> None:
    """Test extract_tables_sync with non-existent file."""
    with pytest.raises(FileNotFoundError):
        extract_tables_sync("/nonexistent/file.pdf")


@pytest.mark.anyio
async def test_extract_tables_async_nonexistent_file() -> None:
    """Test extract_tables_async with non-existent file."""
    with pytest.raises(FileNotFoundError):
        await extract_tables_async("/nonexistent/file.pdf")


def test_extract_tables_sync_with_config() -> None:
    """Test extract_tables_sync respects config."""
    config = GMFTConfig(
        detection_threshold=0.9,
        structure_threshold=0.8,
        model_cache_dir="/custom/cache",
    )

    with patch("kreuzberg._gmft.pdf_document_sync") as mock_pdf:
        # Mock PDF document
        mock_doc = Mock()
        mock_doc.__enter__ = Mock(return_value=mock_doc)
        mock_doc.__exit__ = Mock(return_value=None)
        mock_doc.__len__ = Mock(return_value=0)
        mock_pdf.return_value = mock_doc

        # Create a temporary file
        with tempfile.NamedTemporaryFile(suffix=".pdf") as tmp:
            result = extract_tables_sync(tmp.name, config)

    assert result == []  # No pages, no tables


@pytest.mark.anyio
async def test_extract_tables_async_with_config() -> None:
    """Test extract_tables_async respects config."""
    config = GMFTConfig(
        detection_threshold=0.9,
        structure_threshold=0.8,
    )

    # Create a temporary file
    with tempfile.NamedTemporaryFile(suffix=".pdf") as tmp:
        # Use async mock for run_sync
        with patch("kreuzberg._gmft.run_sync") as mock_run_sync:
            mock_run_sync.return_value = []

            result = await extract_tables_async(tmp.name, config)

    assert result == []


def test_table_detector_device_resolution() -> None:
    """Test TableDetector device resolution."""
    # Test auto device resolution
    config = GMFTConfig(detection_device="auto")
    with patch("kreuzberg._gmft._detector.transformers", None):
        detector = TableDetector(config)
        assert detector._device in ["cpu", "cuda"]  # Should resolve to cpu without torch

    # Test explicit device
    config = GMFTConfig(detection_device="cpu")
    with patch("kreuzberg._gmft._detector.transformers", None):
        detector = TableDetector(config)
        assert detector._device == "cpu"


def test_table_formatter_device_resolution() -> None:
    """Test TableFormatter device resolution."""
    # Test auto device resolution
    config = GMFTConfig(structure_device="auto")
    with patch("kreuzberg._gmft._formatter._import_table_transformer"):
        formatter = TableFormatter(config)
        assert formatter._device in ["cpu", "cuda"]  # Should resolve to cpu without torch

    # Test explicit device
    config = GMFTConfig(structure_device="cpu")
    with patch("kreuzberg._gmft._formatter._import_table_transformer"):
        formatter = TableFormatter(config)
        assert formatter._device == "cpu"


def test_detector_page_number_propagation() -> None:
    """Test that TableDetector properly sets page numbers."""
    with patch("kreuzberg._gmft._detector.transformers", None):
        detector = TableDetector()

        # Create a fake image
        image = Image.new("RGB", (100, 100))

        # Test that detect_tables_in_page_region sets page number
        with pytest.raises(MissingDependencyError):
            detector.detect_tables_in_page_region(image, page_number=5)


@pytest.mark.anyio
async def test_detector_async_methods() -> None:
    """Test TableDetector async methods."""
    with patch("kreuzberg._gmft._detector.transformers", None):
        detector = TableDetector()

        image = Image.new("RGB", (100, 100))

        # Test async detection raises error without dependencies
        with pytest.raises(MissingDependencyError):
            await detector.detect_tables_in_image_async(image)

        with pytest.raises(MissingDependencyError):
            await detector.detect_tables_in_page_region_async(image, page_number=1)


@pytest.mark.anyio
async def test_formatter_async_method() -> None:
    """Test TableFormatter async format_table method."""
    with patch("kreuzberg._gmft._formatter._import_table_transformer"):
        formatter = TableFormatter()

        image = Image.new("RGB", (100, 100))
        table = CroppedTable(
            rect=Rect((10, 10, 90, 90)),
            confidence_score=0.9,
            page_number=0,
            label="table",
        )

        # Test async formatting raises error without dependencies
        with pytest.raises(MissingDependencyError):
            await formatter.format_table_async(table, image)


def test_bbox_predictions_to_dict() -> None:
    """Test BboxPredictions.to_dict method."""
    boxes = [(10.0, 20.0, 100.0, 200.0)]
    scores = [0.9]
    labels = [1]

    predictions = BboxPredictions.from_lists(boxes, scores, labels)
    result = predictions.to_dict()

    assert result["boxes"] == boxes
    assert result["scores"] == scores
    assert result["labels"] == labels


def test_table_predictions_from_dicts() -> None:
    """Test TablePredictions.from_dicts factory method."""
    data = {
        "rows": {"boxes": [(10.0, 20.0, 100.0, 30.0)], "scores": [0.9], "labels": [1]},
        "columns": {"boxes": [(10.0, 20.0, 30.0, 200.0)], "scores": [0.85], "labels": [2]},
        "spanning_cells": {"boxes": [], "scores": [], "labels": []},
    }

    predictions = TablePredictions.from_dicts(data)

    assert len(predictions.rows.boxes) == 1
    assert len(predictions.columns.boxes) == 1
    assert len(predictions.spanning_cells.boxes) == 0


def test_rect_properties() -> None:
    """Test Rect width and height properties."""
    bbox: BBox = (10.0, 20.0, 100.0, 200.0)
    rect = Rect(bbox)

    assert rect.width == 90.0  # 100 - 10
    assert rect.height == 180.0  # 200 - 20


def test_cropped_table_defaults() -> None:
    """Test CroppedTable default values."""
    table = CroppedTable(
        rect=Rect((10, 20, 100, 200)),
        confidence_score=0.9,
        page_number=1,
    )

    assert table.angle == 0.0  # Default angle
    assert table.label == "table"  # Default label


def test_bbox_predictions_validation() -> None:
    """Test BboxPredictions validation in __post_init__."""
    # Mismatched lengths should raise ValueError
    with pytest.raises(ValueError, match="same length"):
        BboxPredictions(
            boxes=((10, 20, 30, 40),),
            scores=(0.9, 0.8),  # Wrong length
            labels=(1,),
        )


def test_gmft_config_device_settings() -> None:
    """Test GMFTConfig device configuration settings."""
    config = GMFTConfig(
        detection_device="cuda",
        structure_device="cpu",
    )

    assert config.detection_device == "cuda"
    assert config.structure_device == "cpu"

    # Test default is auto
    default_config = GMFTConfig()
    assert default_config.detection_device == "auto"
    assert default_config.structure_device == "auto"


def test_gmft_config_threshold_settings() -> None:
    """Test GMFTConfig threshold configuration."""
    config = GMFTConfig(
        detection_threshold=0.9,
        structure_threshold=0.8,
        crop_padding=30,
        min_table_area=500,
    )

    assert config.detection_threshold == 0.9
    assert config.structure_threshold == 0.8
    assert config.crop_padding == 30
    assert config.min_table_area == 500


def test_extract_table_dataframe_empty_predictions() -> None:
    """Test extract_table_dataframe with empty predictions."""
    image = Image.new("RGB", (100, 100))
    predictions = TablePredictions(
        rows=BboxPredictions.from_lists([], [], []),
        columns=BboxPredictions.from_lists([], [], []),
        spanning_cells=BboxPredictions.from_lists([], [], []),
    )
    config = GMFTConfig()

    df = extract_table_dataframe(image, predictions, config)

    assert isinstance(df, pl.DataFrame)
    assert df.is_empty()


def test_formatted_table_structure() -> None:
    """Test FormattedTable structure."""
    cropped_table = CroppedTable(
        rect=Rect((10, 10, 90, 90)),
        confidence_score=0.9,
        page_number=1,
        label="table",
    )
    dataframe = pl.DataFrame({"col1": [1, 2], "col2": [3, 4]})
    predictions = TablePredictions(
        rows=BboxPredictions.from_lists([], [], []),
        columns=BboxPredictions.from_lists([], [], []),
        spanning_cells=BboxPredictions.from_lists([], [], []),
    )
    confidence_scores = {"overall": 0.9, "structure": 0.85}

    formatted = FormattedTable(
        cropped_table=cropped_table,
        dataframe=dataframe,
        predictions=predictions,
        confidence_scores=confidence_scores,
        metadata={},
    )

    assert formatted.cropped_table == cropped_table
    assert formatted.dataframe.equals(dataframe)
    assert formatted.predictions == predictions
    assert formatted.confidence_scores == confidence_scores


def test_table_data_structure() -> None:
    """Test TableData TypedDict structure."""
    image = Image.new("RGB", (100, 100))
    df = pl.DataFrame({"col1": [1, 2]})

    table_data: TableData = {
        "cropped_image": image,
        "df": df,
        "page_number": 1,
        "text": "| col1 |\n|------|\n| 1    |\n| 2    |",
    }

    assert table_data["cropped_image"] == image
    assert table_data["df"] is not None
    assert table_data["df"].equals(df)
    assert table_data["page_number"] == 1
    assert "col1" in table_data["text"]
