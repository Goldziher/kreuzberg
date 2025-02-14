from __future__ import annotations

import json
import subprocess
from asyncio import gather
from dataclasses import dataclass
from tempfile import NamedTemporaryFile
from typing import TYPE_CHECKING, Any, Final, Literal, TypedDict, cast

from anyio import Path as AsyncPath

from kreuzberg._string import normalize_spaces
from kreuzberg._sync import run_sync
from kreuzberg.exceptions import MissingDependencyError, ParsingError, ValidationError

if TYPE_CHECKING:  # pragma: no cover
    from collections.abc import Mapping
    from os import PathLike

try:  # pragma: no cover
    from typing import NotRequired  # type: ignore[attr-defined]
except ImportError:  # pragma: no cover
    from typing_extensions import NotRequired

version_ref: Final[dict[str, bool]] = {"checked": False}


# Block-level node types in Pandoc AST
BLOCK_HEADER: Final = "Header"  # Header with level, attributes and inline content
BLOCK_PARA: Final = "Para"  # Paragraph containing inline content
BLOCK_CODE: Final = "CodeBlock"  # Code block with attributes and string content
BLOCK_QUOTE: Final = "BlockQuote"  # Block quote containing blocks
BLOCK_LIST: Final = "BulletList"  # Bullet list containing items (blocks)
BLOCK_ORDERED: Final = "OrderedList"  # Numbered list with attrs and items

# Inline-level node types in Pandoc AST
INLINE_STR: Final = "Str"  # Plain text string
INLINE_SPACE: Final = "Space"  # Single space
INLINE_EMPH: Final = "Emph"  # Emphasized text (contains inlines)
INLINE_STRONG: Final = "Strong"  # Strong/bold text (contains inlines)
INLINE_LINK: Final = "Link"  # Link with text and target
INLINE_IMAGE: Final = "Image"  # Image with alt text and source
INLINE_CODE: Final = "Code"  # Inline code span
INLINE_MATH: Final = "Math"  # Math expression

# Metadata node types in Pandoc AST
META_MAP: Final = "MetaMap"  # Key-value mapping of metadata
META_LIST: Final = "MetaList"  # List of metadata values
META_INLINES: Final = "MetaInlines"  # Inline content in metadata
META_STRING: Final = "MetaString"  # Plain string in metadata
META_BLOCKS: Final = "MetaBlocks"  # Block content in metadata

# Node content field name
CONTENT_FIELD: Final = "c"
TYPE_FIELD: Final = "t"

# Valid node types
NodeType = Literal[
    # Block types
    "Header",
    "Para",
    "CodeBlock",
    "BlockQuote",
    "BulletList",
    "OrderedList",
    # Inline types
    "Str",
    "Space",
    "Emph",
    "Strong",
    "Link",
    "Image",
    "Code",
    "Math",
    # Meta types
    "MetaMap",
    "MetaList",
    "MetaInlines",
    "MetaString",
    "MetaBlocks",
]

MIMETYPE_TO_PANDOC_TYPE_MAPPING: Final[Mapping[str, str]] = {
    "application/csl+json": "csljson",
    "application/docbook+xml": "docbook",
    "application/epub+zip": "epub",
    "application/rtf": "rtf",
    "application/vnd.oasis.opendocument.text": "odt",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document": "docx",
    "application/x-biblatex": "biblatex",
    "application/x-bibtex": "bibtex",
    "application/x-endnote+xml": "endnotexml",
    "application/x-fictionbook+xml": "fb2",
    "application/x-ipynb+json": "ipynb",
    "application/x-jats+xml": "jats",
    "application/x-latex": "latex",
    "application/x-opml+xml": "opml",
    "application/x-research-info-systems": "ris",
    "application/x-typst": "typst",
    "text/csv": "csv",
    "text/tab-separated-values": "tsv",
    "text/troff": "man",
    "text/x-commonmark": "commonmark",
    "text/x-dokuwiki": "dokuwiki",
    "text/x-gfm": "gfm",
    "text/x-markdown": "markdown",
    "text/x-markdown-extra": "markdown_phpextra",
    "text/x-mdoc": "mdoc",
    "text/x-multimarkdown": "markdown_mmd",
    "text/x-org": "org",
    "text/x-pod": "pod",
    "text/x-rst": "rst",
}

MIMETYPE_TO_FILE_EXTENSION_MAPPING: Final[Mapping[str, str]] = {
    "application/csl+json": "json",
    "application/docbook+xml": "xml",
    "application/epub+zip": "epub",
    "application/rtf": "rtf",
    "application/vnd.oasis.opendocument.text": "odt",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document": "docx",
    "application/x-biblatex": "bib",
    "application/x-bibtex": "bib",
    "application/x-endnote+xml": "xml",
    "application/x-fictionbook+xml": "fb2",
    "application/x-ipynb+json": "ipynb",
    "application/x-jats+xml": "xml",
    "application/x-latex": "tex",
    "application/x-opml+xml": "opml",
    "application/x-research-info-systems": "ris",
    "application/x-typst": "typst",
    "text/csv": "csv",
    "text/tab-separated-values": "tsv",
    "text/troff": "1",
    "text/x-commonmark": "md",
    "text/x-dokuwiki": "wiki",
    "text/x-gfm": "md",
    "text/x-markdown": "md",
    "text/x-markdown-extra": "md",
    "text/x-mdoc": "md",
    "text/x-multimarkdown": "md",
    "text/x-org": "org",
    "text/x-pod": "pod",
    "text/x-rst": "rst",
}


class Metadata(TypedDict, total=False):
    """Document metadata extracted from Pandoc document.

    All fields are optional but will only be included if they contain non-empty values.
    Any field that would be empty or None is omitted from the dictionary.
    """

    title: NotRequired[str]
    """Document title."""
    subtitle: NotRequired[str]
    """Document subtitle."""
    abstract: NotRequired[str | list[str]]
    """Document abstract, summary or description."""
    authors: NotRequired[list[str]]
    """List of document authors."""
    date: NotRequired[str]
    """Document date as string to preserve original format."""
    subject: NotRequired[str]
    """Document subject or topic."""
    description: NotRequired[str]
    """Extended description."""
    keywords: NotRequired[list[str]]
    """Keywords or tags."""
    categories: NotRequired[list[str]]
    """Categories or classifications."""
    version: NotRequired[str]
    """Version identifier."""
    language: NotRequired[str]
    """Document language code."""
    references: NotRequired[list[str]]
    """Reference entries."""
    citations: NotRequired[list[str]]
    """Citation identifiers."""
    copyright: NotRequired[str]
    """Copyright information."""
    license: NotRequired[str]
    """License information."""
    identifier: NotRequired[str]
    """Document identifier."""
    publisher: NotRequired[str]
    """Publisher name."""
    contributors: NotRequired[list[str]]
    """Additional contributors."""
    creator: NotRequired[str]
    """Document creator."""
    institute: NotRequired[str | list[str]]
    """Institute or organization."""


@dataclass
class PandocResult:
    """Result of a pandoc conversion including content and metadata."""

    content: str
    """The processed markdown content."""
    metadata: Metadata
    """Document metadata extracted from the source."""


def _extract_inline_text(node: dict[str, Any]) -> str | None:
    if node_type := node.get(TYPE_FIELD):
        if node_type == INLINE_STR:
            return node.get(CONTENT_FIELD)
        if node_type == INLINE_SPACE:
            return " "
        if node_type in (INLINE_EMPH, INLINE_STRONG):
            return _extract_inlines(node.get(CONTENT_FIELD, []))
    return None  # pragma: no cover


def _extract_inlines(nodes: list[dict[str, Any]]) -> str | None:
    texts = [text for node in nodes if (text := _extract_inline_text(node))]
    result = "".join(texts).strip()
    return result if result else None


def _extract_meta_value(node: Any) -> str | list[str] | None:
    if not isinstance(node, dict) or CONTENT_FIELD not in node or TYPE_FIELD not in node:
        return None

    content = node[CONTENT_FIELD]
    node_type = node[TYPE_FIELD]

    if not content or node_type not in {
        META_STRING,
        META_INLINES,
        META_LIST,
        META_BLOCKS,
    }:
        return None

    if node_type == META_STRING and isinstance(content, str):
        return content

    if isinstance(content, list) and (content := [v for v in content if isinstance(v, dict)]):
        if node_type == META_INLINES:
            return _extract_inlines(cast(list[dict[str, Any]], content))

        if node_type == META_LIST:
            results = []
            for value in [value for item in content if (value := _extract_meta_value(item))]:
                if isinstance(value, list):
                    results.extend(value)  # pragma: no cover
                else:
                    results.append(value)
            return results

        if blocks := [block for block in content if block.get(TYPE_FIELD) == BLOCK_PARA]:
            block_texts = []
            for block in blocks:
                block_content = block.get(CONTENT_FIELD, [])
                if isinstance(block_content, list) and (text := _extract_inlines(block_content)):
                    block_texts.append(text)
            return block_texts if block_texts else None

    return None


def _extract_metadata(raw_meta: dict[str, Any]) -> Metadata:
    meta: Metadata = {}

    for key, value in raw_meta.items():
        if extracted := _extract_meta_value(value):
            meta[key] = extracted  # type: ignore[literal-required]

    citations = [
        cite["citationId"]
        for block in raw_meta.get("blocks", [])
        if block.get(TYPE_FIELD) == "Cite"
        for cite in block.get(CONTENT_FIELD, [[{}]])[0]
        if isinstance(cite, dict)
    ]
    if citations:
        meta["citations"] = citations

    return meta


def _get_pandoc_type_from_mime_type(mime_type: str) -> str:
    if mime_type not in MIMETYPE_TO_PANDOC_TYPE_MAPPING or not any(
        mime_type.startswith(value) for value in MIMETYPE_TO_PANDOC_TYPE_MAPPING
    ):
        raise ValidationError(
            f"Unsupported mime type: {mime_type}",
            context={
                "mime_type": mime_type,
                "supported_mimetypes": ",".join(sorted(MIMETYPE_TO_PANDOC_TYPE_MAPPING)),
            },
        )

    return MIMETYPE_TO_PANDOC_TYPE_MAPPING.get(mime_type) or next(
        MIMETYPE_TO_PANDOC_TYPE_MAPPING[k] for k in MIMETYPE_TO_PANDOC_TYPE_MAPPING if k.startswith(mime_type)
    )


async def _validate_pandoc_version() -> None:
    try:
        if version_ref["checked"]:
            return

        command = ["pandoc", "--version"]
        result = await run_sync(subprocess.run, command, capture_output=True)
        version = result.stdout.decode().split("\n")[0].split()[1]
        if not version.startswith("3."):
            raise MissingDependencyError("Pandoc version 3 or above is required.")

        version_ref["checked"] = True

    except FileNotFoundError as e:
        raise MissingDependencyError("Pandoc is not installed.") from e


async def _handle_extract_metadata(input_file: str | PathLike[str], *, mime_type: str) -> Metadata:
    pandoc_type = _get_pandoc_type_from_mime_type(mime_type)

    with NamedTemporaryFile(suffix=".json", delete=False) as metadata_file:
        try:
            command = [
                "pandoc",
                str(input_file),
                f"--from={pandoc_type}",
                "--to=json",
                "--standalone",
                "--quiet",
                "--output",
                metadata_file.name,
            ]

            result = await run_sync(
                subprocess.run,
                command,
                capture_output=True,
            )

            if result.returncode != 0:
                raise ParsingError(
                    "Failed to extract file data", context={"file": str(input_file), "error": result.stderr.decode()}
                )

            json_data = json.loads(await AsyncPath(metadata_file.name).read_text("utf-8"))
            return _extract_metadata(json_data)

        except (RuntimeError, OSError, json.JSONDecodeError) as e:
            raise ParsingError("Failed to extract file data", context={"file": str(input_file)}) from e

        finally:
            metadata_file.close()
            await AsyncPath(metadata_file.name).unlink()


async def _handle_extract_file(
    input_file: str | PathLike[str], *, mime_type: str, extra_args: list[str] | None = None
) -> str:
    pandoc_type = _get_pandoc_type_from_mime_type(mime_type)

    with NamedTemporaryFile(suffix=".md", delete=False) as output_file:
        try:
            command = [
                "pandoc",
                str(input_file),
                f"--from={pandoc_type}",
                "--to=markdown",
                "--standalone",
                "--wrap=preserve",
                "--quiet",
                "--output",
                output_file.name,
            ]

            if extra_args:
                command.extend(extra_args)

            result = await run_sync(
                subprocess.run,
                command,
                capture_output=True,
            )

            if result.returncode != 0:
                raise ParsingError(
                    "Failed to extract file data", context={"file": str(input_file), "error": result.stderr.decode()}
                )

            text = await AsyncPath(output_file.name).read_text("utf-8")

            return normalize_spaces(text)

        except (RuntimeError, OSError) as e:
            raise ParsingError("Failed to extract file data", context={"file": str(input_file)}) from e

        finally:
            output_file.close()
            await AsyncPath(output_file.name).unlink()


async def process_file(
    input_file: str | PathLike[str], *, mime_type: str, extra_args: list[str] | None = None
) -> PandocResult:
    """Process a single file using Pandoc and convert to markdown.

    Args:
        input_file: The path to the file to process.
        mime_type: The mime type of the file.
        extra_args: Additional Pandoc command line arguments.

    Returns:
        PandocResult containing processed content and metadata.
    """
    await _validate_pandoc_version()

    metadata, content = await gather(
        *[
            _handle_extract_metadata(input_file, mime_type=mime_type),
            _handle_extract_file(input_file, mime_type=mime_type, extra_args=extra_args),
        ]
    )
    return PandocResult(
        content=content,  # type: ignore[arg-type]
        metadata=metadata,  # type: ignore[arg-type]
    )


async def process_content(content: bytes, *, mime_type: str, extra_args: list[str] | None = None) -> PandocResult:
    """Process content using Pandoc and convert to markdown.

    Args:
        content: The content to process.
        mime_type: The mime type of the content.
        extra_args: Additional Pandoc command line arguments.

    Returns:
        PandocResult containing processed content and metadata.
    """
    extension = MIMETYPE_TO_FILE_EXTENSION_MAPPING.get(mime_type) or "md"

    with NamedTemporaryFile(suffix=f".{extension}", delete=False) as input_file:
        try:
            await AsyncPath(input_file.name).write_bytes(content)
            return await process_file(input_file.name, mime_type=mime_type, extra_args=extra_args)

        finally:
            input_file.close()
            await AsyncPath(input_file.name).unlink()
