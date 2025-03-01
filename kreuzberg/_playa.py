from __future__ import annotations

from dataclasses import asdict
from typing import Any, TypedDict, cast

import playa
from playa.page import Page, PathObject, PathOperator
from playa.pdftypes import ObjRef, PSLiteral

from kreuzberg._sync import run_taskgroup


class DestinationMetadata(TypedDict):
    """Metadata for a PDF destination."""

    page_idx: int | None
    """Page index for the destination."""
    display: str | None
    """Display mode for the destination."""
    coords: tuple[float | None, ...]
    """Coordinates of the destination."""


class ActionMetadata(TypedDict):
    """Metadata for a PDF action."""

    type: str
    """Type of action."""

    destination: DestinationMetadata | None
    """Destination reference, if present."""


class MarkedContentMetadata(TypedDict):
    """Metadata for marked content sections."""

    tag: str
    """Name of the marked content tag."""

    properties: dict[str, Any]
    """Properties associated with the marked content."""

    mcid: int | None
    """Marked Content Identifier, if present."""


class TextObjectMetadata(TypedDict):
    """Metadata for a text object in a PDF."""

    bbox: tuple[float, float, float, float]
    """Bounding box coordinates (left, top, right, bottom)."""

    graphic_state: dict[str, Any]
    """The graphic state of the path object."""

    marked_content: MarkedContentMetadata
    """Marked content section information."""


class PathSegmentMetadata(TypedDict):
    """Metadata for a path segment."""

    type: PathOperator
    """Type of path segment (move, line, curve, etc.)."""

    points: list[tuple[float, float]]
    """List of points defining the path segment."""


class PathObjectMetadata(TypedDict):
    """Metadata for a path object in a PDF."""

    bbox: tuple[float, float, float, float]
    """Bounding box coordinates."""

    graphic_state: dict[str, Any]
    """The graphic state of the path object."""

    segments: list[PathSegmentMetadata]
    """List of path segments."""


class XObjectContentMetadata(TypedDict):
    """Metadata for content within an XObject."""

    type: str
    """Type of content object."""

    bbox: tuple[float, float, float, float]
    """Bounding box coordinates."""


class XObjectMetadata(TypedDict):
    """Metadata for a Form XObject in a PDF."""

    bbox: tuple[float, float, float, float]
    """Bounding box coordinates."""

    content: list[XObjectContentMetadata]
    """List of content objects within the XObject."""


class AnnotationMetadata(TypedDict):
    """Metadata for a PDF annotation."""

    subtype: str
    """Annotation subtype (Link, Widget, etc.)."""

    rect: tuple[float, float, float, float]
    """Rectangle coordinates defining the annotation's bounds."""

    properties: dict[str, Any]
    """Additional annotation properties."""


class MarkedContentSectionMetadata(TypedDict):
    """Metadata for a marked content section."""

    tag: str
    """Name of the marked content tag."""

    properties: dict[str, Any]
    """Properties associated with the marked content."""

    mcid: int | None
    """Marked Content Identifier, if present."""

    bbox: tuple[float, float, float, float]
    """Bounding box coordinates."""


class PageMetadata(TypedDict):
    """Detailed metadata for a single PDF page."""

    index: int
    """0-based page number."""

    page_number: str
    """Page label (could be roman numerals, letters, etc)."""

    dimensions: tuple[int, int]
    """Page dimensions (width, height) in points."""

    rotation: int
    """Page rotation in degrees."""

    text_objects: list[TextObjectMetadata]
    """List of text objects on the page."""

    path_objects: list[PathObjectMetadata]
    """List of path objects (graphics) on the page."""

    xobjects: list[XObjectMetadata]
    """List of Form XObjects on the page."""

    annotations: list[AnnotationMetadata]
    """List of page annotations."""

    marked_content: list[MarkedContentSectionMetadata]
    """List of marked content sections."""


class OutlineEntryMetadata(TypedDict):
    """Metadata for a document outline entry."""

    title: str | None
    """Title of the outline entry."""

    destination: DestinationMetadata | None
    """Destination reference."""

    action: ActionMetadata | None
    """Action to perform when entry is activated."""

    element: str | None
    """Associated structure element reference."""


class StructureElementMetadata(TypedDict):
    """Metadata for a logical structure element."""

    type: str
    """Type of structure element."""

    attributes: str
    """Element attributes as string."""

    children: list[str]
    """Names of child elements."""


class DocumentMetadata(TypedDict):
    """Complete metadata for a PDF document."""

    total_pages: int
    """Total number of pages in the document."""

    page_labels: list[str]
    """List of all page labels."""

    outline: list[OutlineEntryMetadata]
    """Document outline/bookmarks."""

    structure: list[StructureElementMetadata]
    """Logical structure elements."""

    pages: list[PageMetadata]
    """Detailed metadata for each page."""


def _parse_to_string(value: Any) -> str:
    """Parse a PDF object to a string representation.

    Args:
        value: The PDF object to parse.

    Returns:
        A string representation of the PDF object.
    """
    return f"/{value.name}" if isinstance(value, PSLiteral) else str(value)


def _normalize_to_dict(value: Any) -> dict[str, Any]:
    """Normalize a PDF object to a serializable dictionary.

    Args:
        value: The PDF object to normalize.

    Returns:
        A dictionary representation of the PDF object.
    """
    if not isinstance(value, dict):
        return {}

    normalized_items = [(k.lower(), v.resolve() if isinstance(v, ObjRef) else v) for k, v in value.items()]

    ret: dict[str, Any] = {}
    for k, v in normalized_items:
        if isinstance(v, dict):
            ret[k] = _normalize_to_dict(v)
        elif isinstance(v, list):
            ret[k] = [_normalize_to_dict(item) for item in v]
        elif isinstance(v, PSLiteral):
            ret[k] = _parse_to_string(v)
        else:
            ret[k] = v
    return ret


async def _extract_page_metadata(page: Page) -> PageMetadata:
    """Extract detailed metadata from a single PDF page.

    Args:
        page: A playa Page object.

    Returns:
        PageMetadata containing all extractable information from the page.
    """
    text_objects: list[TextObjectMetadata] = []
    path_objects: list[PathObjectMetadata] = []
    xobjects: list[XObjectMetadata] = []
    marked_content: list[MarkedContentSectionMetadata] = []

    for obj in page:
        if obj.object_type == "text":
            text_objects.append(
                TextObjectMetadata(
                    bbox=obj.bbox,
                    graphic_state=_normalize_to_dict(asdict(obj.gstate) if obj.gstate else {}),
                    marked_content=MarkedContentMetadata(
                        tag=obj.mcs.tag if obj.mcs else "",
                        properties=obj.mcs.props if obj.mcs else {},
                        mcid=obj.mcs.mcid if obj.mcs and hasattr(obj.mcs, "mcid") else None,
                    ),
                )
            )
        elif obj.object_type == "path":
            segments = [
                PathSegmentMetadata(type=seg.operator, points=[(float(x), float(y)) for x, y in seg.points])
                for seg in cast(PathObject, obj).raw_segments
            ]
            path_objects.append(
                PathObjectMetadata(
                    bbox=obj.bbox,
                    graphic_state=_normalize_to_dict(asdict(obj.gstate) if obj.gstate else {}),
                    segments=segments,
                )
            )
        elif obj.object_type == "xobject":
            xobject_content = [XObjectContentMetadata(type=item.object_type, bbox=item.bbox) for item in obj]
            xobjects.append(XObjectMetadata(bbox=obj.bbox, content=xobject_content))

        if hasattr(obj, "mcs") and obj.mcs is not None and obj.mcs.tag:
            marked_content.append(
                MarkedContentSectionMetadata(
                    tag=obj.mcs.tag if obj.mcs else "",
                    properties=_normalize_to_dict(obj.mcs.props) if obj.mcs else {},
                    mcid=obj.mcs.mcid if obj.mcs and hasattr(obj.mcs, "mcid") else None,
                    bbox=obj.bbox,
                )
            )

    return PageMetadata(
        annotations=[
            AnnotationMetadata(subtype=annot.subtype, rect=annot.rect, properties=_normalize_to_dict(annot.props))
            for annot in page.annotations
        ],
        dimensions=(int(page.width), int(page.height)),
        page_number=str(page.label) if page.label is not None else "",
        marked_content=marked_content,
        index=page.page_idx,
        path_objects=path_objects,
        rotation=page.rotate,
        text_objects=text_objects,
        xobjects=xobjects,
    )


async def extract_pdf_metadata(pdf_content: bytes) -> DocumentMetadata:
    """Extract complete metadata from a PDF document.

    Args:
        pdf_content: The raw PDF content.

    Returns:
        DocumentMetadata containing all extractable information.
    """
    document = playa.parse(pdf_content, max_workers=1, space="screen")
    tasks = [_extract_page_metadata(page) for page in document.pages]
    pages: list[PageMetadata] = await run_taskgroup(*tasks)

    outline = [
        OutlineEntryMetadata(
            title=entry.title,
            destination=DestinationMetadata(
                page_idx=entry.destination.page_idx,
                display=_parse_to_string(entry.destination.display),
                coords=entry.destination.coords,
            )
            if entry.destination
            else None,
            action=ActionMetadata(
                type=_parse_to_string(entry.action.type),
                destination=DestinationMetadata(
                    page_idx=entry.action.destination.page_idx,
                    display=_parse_to_string(entry.action.destination.display),
                    coords=entry.action.destination.coords,
                )
                if entry.action.destination
                else None,
            )
            if entry.action
            else None,
            element=str(entry.element),
        )
        for entry in (document.outline or [])
    ]

    structure = [
        StructureElementMetadata(
            type=str(getattr(element, "name", "")),
            attributes=str(getattr(element, "attributes", "")),
            children=[str(getattr(child, "name", "")) for child in (element or [])],
        )
        for element in (document.structure or [])
    ]

    return DocumentMetadata(
        total_pages=len(document.pages),
        page_labels=[str(page.label) if page.label is not None else "" for page in document.pages],
        outline=outline,
        structure=structure,
        pages=pages,
    )
