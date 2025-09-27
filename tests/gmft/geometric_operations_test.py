"""Tests for GMFT geometric operations and base functions."""

from __future__ import annotations

from kreuzberg._gmft._base import (
    EMPTY_RECT,
    Rect,
    iob_bbox,
    iob_for_rows,
    rect_area,
    rect_height,
    rect_intersect,
    rect_is_intersecting,
    rect_width,
)


def test_rect_creation() -> None:
    bbox = (10.0, 20.0, 100.0, 200.0)
    rect = Rect(bbox)
    assert rect.bbox == bbox


def test_rect_properties() -> None:
    rect = Rect((10.0, 20.0, 100.0, 200.0))
    assert rect.xmin == 10.0
    assert rect.ymin == 20.0
    assert rect.xmax == 100.0
    assert rect.ymax == 200.0
    assert rect.width == 90.0
    assert rect.height == 180.0
    assert rect.area == 16200.0


def test_rect_intersect_method() -> None:
    rect1 = Rect((10.0, 10.0, 50.0, 50.0))
    rect2 = Rect((30.0, 30.0, 70.0, 70.0))

    intersection = rect1.intersect(rect2)
    assert intersection.xmin == 30.0
    assert intersection.ymin == 30.0
    assert intersection.xmax == 50.0
    assert intersection.ymax == 50.0


def test_rect_intersect_with_bbox() -> None:
    rect = Rect((10.0, 10.0, 50.0, 50.0))
    bbox = (30.0, 30.0, 70.0, 70.0)

    intersection = rect.intersect(bbox)
    assert intersection.xmin == 30.0
    assert intersection.ymin == 30.0
    assert intersection.xmax == 50.0
    assert intersection.ymax == 50.0


def test_rect_is_intersecting_true() -> None:
    rect1 = Rect((10.0, 10.0, 50.0, 50.0))
    rect2 = Rect((30.0, 30.0, 70.0, 70.0))
    assert rect1.is_intersecting(rect2) is True


def test_rect_is_intersecting_false() -> None:
    rect1 = Rect((10.0, 10.0, 30.0, 30.0))
    rect2 = Rect((50.0, 50.0, 70.0, 70.0))
    assert rect1.is_intersecting(rect2) is False


def test_rect_hash() -> None:
    rect1 = Rect((10.0, 10.0, 50.0, 50.0))
    rect2 = Rect((10.0, 10.0, 50.0, 50.0))
    rect3 = Rect((20.0, 20.0, 60.0, 60.0))

    # Same bbox should have same hash
    assert hash(rect1) == hash(rect2)
    # Different bbox should have different hash (usually)
    assert hash(rect1) != hash(rect3)


def test_rect_equality() -> None:
    rect1 = Rect((10.0, 10.0, 50.0, 50.0))
    rect2 = Rect((10.0, 10.0, 50.0, 50.0))
    rect3 = Rect((20.0, 20.0, 60.0, 60.0))

    assert rect1 == rect2
    assert rect1 != rect3
    assert rect1 != "not a rect"


def test_rect_repr() -> None:
    rect = Rect((10.0, 20.0, 100.0, 200.0))
    assert repr(rect) == "Rect((10.0, 20.0, 100.0, 200.0))"


def test_rect_intersect_function() -> None:
    bbox1 = (10.0, 10.0, 50.0, 50.0)
    bbox2 = (30.0, 30.0, 70.0, 70.0)

    intersection = rect_intersect(bbox1, bbox2)
    assert intersection.bbox == (30.0, 30.0, 50.0, 50.0)


def test_rect_intersect_no_overlap() -> None:
    bbox1 = (10.0, 10.0, 30.0, 30.0)
    bbox2 = (50.0, 50.0, 70.0, 70.0)

    intersection = rect_intersect(bbox1, bbox2)
    assert intersection == EMPTY_RECT


def test_rect_intersect_touching_edges() -> None:
    bbox1 = (10.0, 10.0, 30.0, 30.0)
    bbox2 = (30.0, 30.0, 50.0, 50.0)

    intersection = rect_intersect(bbox1, bbox2)
    assert intersection == EMPTY_RECT  # Touching edges don't count as intersection


def test_rect_is_intersecting_function() -> None:
    bbox1 = (10.0, 10.0, 50.0, 50.0)
    bbox2 = (30.0, 30.0, 70.0, 70.0)
    bbox3 = (60.0, 60.0, 80.0, 80.0)

    assert rect_is_intersecting(bbox1, bbox2) is True
    assert rect_is_intersecting(bbox1, bbox3) is False


def test_rect_width_function() -> None:
    bbox = (10.0, 20.0, 100.0, 200.0)
    assert rect_width(bbox) == 90.0


def test_rect_height_function() -> None:
    bbox = (10.0, 20.0, 100.0, 200.0)
    assert rect_height(bbox) == 180.0


def test_rect_area_function() -> None:
    bbox = (10.0, 20.0, 100.0, 200.0)
    assert rect_area(bbox) == 16200.0


def test_rect_area_zero() -> None:
    bbox = (10.0, 20.0, 10.0, 20.0)  # Point, no area
    assert rect_area(bbox) == 0.0


def test_iob_bbox_function() -> None:
    bbox1 = (10.0, 10.0, 50.0, 50.0)  # 40x40 = 1600 area
    bbox2 = (30.0, 30.0, 70.0, 70.0)  # Intersection: 20x20 = 400 area

    iob = iob_bbox(bbox1, bbox2)
    expected = 400.0 / 1600.0  # 0.25
    assert abs(iob - expected) < 1e-10


def test_iob_bbox_no_intersection() -> None:
    bbox1 = (10.0, 10.0, 30.0, 30.0)
    bbox2 = (50.0, 50.0, 70.0, 70.0)

    iob = iob_bbox(bbox1, bbox2)
    assert iob == 0.0


def test_iob_bbox_zero_area() -> None:
    bbox1 = (10.0, 10.0, 10.0, 10.0)  # Zero area
    bbox2 = (5.0, 5.0, 15.0, 15.0)

    iob = iob_bbox(bbox1, bbox2)
    assert iob == 0.0


def test_iob_for_rows_function() -> None:
    bbox1 = (10.0, 10.0, 50.0, 30.0)  # Height: 20
    bbox2 = (30.0, 20.0, 70.0, 40.0)  # Height: 20, overlap: 10

    iob = iob_for_rows(bbox1, bbox2)
    expected = 10.0 / 20.0  # 0.5
    assert abs(iob - expected) < 1e-10


def test_iob_for_rows_no_overlap() -> None:
    bbox1 = (10.0, 10.0, 50.0, 30.0)
    bbox2 = (30.0, 40.0, 70.0, 60.0)  # No vertical overlap

    iob = iob_for_rows(bbox1, bbox2)
    assert iob == 0.0


def test_iob_for_rows_zero_height() -> None:
    bbox1 = (10.0, 20.0, 50.0, 20.0)  # Zero height
    bbox2 = (30.0, 15.0, 70.0, 25.0)

    iob = iob_for_rows(bbox1, bbox2)
    assert iob == 0.0


def test_empty_rect() -> None:
    assert EMPTY_RECT.bbox == (0, 0, 0, 0)
    assert EMPTY_RECT.area == 0
    assert EMPTY_RECT.width == 0
    assert EMPTY_RECT.height == 0


def test_rect_negative_coordinates() -> None:
    rect = Rect((-10.0, -20.0, 50.0, 100.0))
    assert rect.width == 60.0
    assert rect.height == 120.0
    assert rect.area == 7200.0


def test_rect_invalid_bbox() -> None:
    rect = Rect((50.0, 100.0, 10.0, 20.0))  # Invalid: right < left, bottom < top
    assert rect.width == -40.0  # Negative width
    assert rect.height == -80.0  # Negative height
    assert rect.area == 3200.0  # Still positive (negative * negative)


def test_intersect_with_invalid_bbox() -> None:
    bbox1 = (50.0, 50.0, 10.0, 10.0)  # Invalid bbox
    bbox2 = (10.0, 10.0, 50.0, 50.0)  # Valid bbox

    # Should still work mathematically
    intersection = rect_intersect(bbox1, bbox2)
    # The max/min operations should still produce a result
    assert isinstance(intersection, Rect)
