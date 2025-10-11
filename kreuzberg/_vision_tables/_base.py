from __future__ import annotations

BBox = tuple[float, float, float, float]


class Rect:
    __slots__ = ("bbox",)

    def __init__(self, bbox: BBox) -> None:
        self.bbox = bbox

    def intersect(self, other: BBox | Rect) -> Rect:
        other_bbox = other.bbox if isinstance(other, Rect) else other
        return rect_intersect(self.bbox, other_bbox)

    def is_intersecting(self, other: BBox | Rect) -> bool:
        other_bbox = other.bbox if isinstance(other, Rect) else other
        return rect_is_intersecting(self.bbox, other_bbox)

    @property
    def width(self) -> float:
        return rect_width(self.bbox)

    @property
    def height(self) -> float:
        return rect_height(self.bbox)

    @property
    def xmin(self) -> float:
        return self.bbox[0]

    @property
    def ymin(self) -> float:
        return self.bbox[1]

    @property
    def xmax(self) -> float:
        return self.bbox[2]

    @property
    def ymax(self) -> float:
        return self.bbox[3]

    @property
    def area(self) -> float:
        return rect_area(self.bbox)

    def __hash__(self) -> int:
        return hash(self.bbox)

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, Rect):
            return False
        return self.bbox == other.bbox

    def __repr__(self) -> str:
        return f"Rect({self.bbox})"


def rect_intersect(bbox1: BBox, bbox2: BBox) -> Rect:
    xmin = max(bbox1[0], bbox2[0])
    ymin = max(bbox1[1], bbox2[1])
    xmax = min(bbox1[2], bbox2[2])
    ymax = min(bbox1[3], bbox2[3])

    if xmin >= xmax or ymin >= ymax:
        return EMPTY_RECT

    return Rect((xmin, ymin, xmax, ymax))


def rect_is_intersecting(bbox1: BBox, bbox2: BBox) -> bool:
    xmin = max(bbox1[0], bbox2[0])
    ymin = max(bbox1[1], bbox2[1])
    xmax = min(bbox1[2], bbox2[2])
    ymax = min(bbox1[3], bbox2[3])

    return xmin < xmax and ymin < ymax


def rect_width(bbox: BBox) -> float:
    return bbox[2] - bbox[0]


def rect_height(bbox: BBox) -> float:
    return bbox[3] - bbox[1]


def rect_area(bbox: BBox) -> float:
    return (bbox[2] - bbox[0]) * (bbox[3] - bbox[1])


def iob_bbox(bbox1: BBox, bbox2: BBox) -> float:
    intersection = rect_intersect(bbox1, bbox2)
    bbox1_area = rect_area(bbox1)

    if bbox1_area > 0:
        return intersection.area / bbox1_area

    return 0.0


def iob_for_rows(bbox1: BBox, bbox2: BBox) -> float:
    a0, a1 = bbox1[1], bbox1[3]
    b0, b1 = bbox2[1], bbox2[3]

    intersect0, intersect1 = max(a0, b0), min(a1, b1)
    intersect_area = max(0, intersect1 - intersect0)

    bbox1_area = a1 - a0
    if bbox1_area > 0:
        return intersect_area / bbox1_area

    return 0.0


EMPTY_RECT = Rect((0, 0, 0, 0))
