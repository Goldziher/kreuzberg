"""Base geometric and data structures for table extraction.

Adapted from GMFT: https://github.com/conjuncts/gmft
Uses functional approaches and hashable structures for caching.
"""

from __future__ import annotations

# Type alias for bounding box coordinates
BBox = tuple[float, float, float, float]


class Rect:
    """A hashable floating-point rectangle for bounding box operations.

    Adapted from GMFT's Rect class with slots and caching support.
    """

    __slots__ = ("bbox",)

    def __init__(self, bbox: BBox) -> None:
        """Initialize rectangle with (xmin, ymin, xmax, ymax) coordinates."""
        self.bbox = bbox

    def intersect(self, other: BBox | Rect) -> Rect:
        """Return the intersection of this rectangle with another."""
        other_bbox = other.bbox if isinstance(other, Rect) else other
        return rect_intersect(self.bbox, other_bbox)

    def is_intersecting(self, other: BBox | Rect) -> bool:
        """Check if this rectangle intersects with another."""
        other_bbox = other.bbox if isinstance(other, Rect) else other
        return rect_is_intersecting(self.bbox, other_bbox)

    @property
    def width(self) -> float:
        """Width of the rectangle."""
        return rect_width(self.bbox)

    @property
    def height(self) -> float:
        """Height of the rectangle."""
        return rect_height(self.bbox)

    @property
    def xmin(self) -> float:
        """Left edge coordinate."""
        return self.bbox[0]

    @property
    def ymin(self) -> float:
        """Top edge coordinate."""
        return self.bbox[1]

    @property
    def xmax(self) -> float:
        """Right edge coordinate."""
        return self.bbox[2]

    @property
    def ymax(self) -> float:
        """Bottom edge coordinate."""
        return self.bbox[3]

    @property
    def area(self) -> float:
        """Area of the rectangle."""
        return rect_area(self.bbox)

    def __hash__(self) -> int:
        return hash(self.bbox)

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, Rect):
            return False
        return self.bbox == other.bbox

    def __repr__(self) -> str:
        return f"Rect({self.bbox})"


# Functional geometry operations for performance and caching
def rect_intersect(bbox1: BBox, bbox2: BBox) -> Rect:
    """Compute intersection of two bounding boxes."""
    xmin = max(bbox1[0], bbox2[0])
    ymin = max(bbox1[1], bbox2[1])
    xmax = min(bbox1[2], bbox2[2])
    ymax = min(bbox1[3], bbox2[3])

    if xmin >= xmax or ymin >= ymax:
        return EMPTY_RECT

    return Rect((xmin, ymin, xmax, ymax))


def rect_is_intersecting(bbox1: BBox, bbox2: BBox) -> bool:
    """Check if two bounding boxes intersect."""
    xmin = max(bbox1[0], bbox2[0])
    ymin = max(bbox1[1], bbox2[1])
    xmax = min(bbox1[2], bbox2[2])
    ymax = min(bbox1[3], bbox2[3])

    return xmin < xmax and ymin < ymax


def rect_width(bbox: BBox) -> float:
    """Compute width of a bounding box."""
    return bbox[2] - bbox[0]


def rect_height(bbox: BBox) -> float:
    """Compute height of a bounding box."""
    return bbox[3] - bbox[1]


def rect_area(bbox: BBox) -> float:
    """Compute area of a bounding box."""
    return (bbox[2] - bbox[0]) * (bbox[3] - bbox[1])


def iob_bbox(bbox1: BBox, bbox2: BBox) -> float:
    """Compute the intersection area over box area, for bbox1.
    Used in table structure analysis.
    """
    intersection = rect_intersect(bbox1, bbox2)
    bbox1_area = rect_area(bbox1)

    if bbox1_area > 0:
        return intersection.area / bbox1_area

    return 0.0


def iob_for_rows(bbox1: BBox, bbox2: BBox) -> float:
    """Modified iob for rows: pretend that the bboxes are infinitely wide.
    For bbox1.
    """
    a0, a1 = bbox1[1], bbox1[3]
    b0, b1 = bbox2[1], bbox2[3]

    intersect0, intersect1 = max(a0, b0), min(a1, b1)
    intersect_area = max(0, intersect1 - intersect0)

    bbox1_area = a1 - a0
    if bbox1_area > 0:
        return intersect_area / bbox1_area

    return 0.0


# Constants
EMPTY_RECT = Rect((0, 0, 0, 0))
